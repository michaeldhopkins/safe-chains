//! Variables a command sets in the current shell other than by a plain `NAME=value`.
//!
//! The scope walk binds `X=./a` and then reads `$X` as `./a`. `export X=-delete`, `declare`,
//! `read X`, `unset X` or a sourced file change `X` behind that binding, and a later `$X` read as
//! the stale value is approved for what it no longer is. Each name such a command could set is
//! returned, so the caller marks it uncertain.

use super::walk::TRUSTED_VARS;
use crate::cst::{SimpleCmd, Word, WordPart};
use crate::parse::Token;

/// The names `cmd` may set. When it is not known which (a flag such as `declare -n`, a name that
/// is itself unknown, a sourced file, an arithmetic assignment), every name that matters: each one
/// bound so far, the trusted session variables, and `IFS`.
pub(crate) fn declared_names(cmd: &SimpleCmd) -> Vec<String> {
    let any = || {
        let mut names = crate::pathctx::item_shape::bound_names();
        names.extend(TRUSTED_VARS.iter().chain(&["IFS"]).map(|s| (*s).to_string()));
        names
    };
    if cmd.words.iter().any(assigns_in_arithmetic) {
        return any();
    }
    let Some(name) = cmd.words.first().map(|w| Token::from_raw(w.eval()).command_name().to_string()) else {
        return Vec::new();
    };
    let args = &cmd.words[1..];
    match name.as_str() {
        "source" | "." | "let" => any(),
        "printf" if args.iter().any(|w| w.eval().starts_with("-v")) => any(),
        "export" | "declare" | "typeset" | "local" | "readonly" | "read" | "unset" | "mapfile" | "readarray" | "getopts" => {
            let mut names = Vec::new();
            for arg in args {
                let text = arg.eval();
                let declared = text.split('=').next().unwrap_or_default();
                let unknown = arg.expand().len() != 1 || declared.contains(['$', '*', '?', '[']) || declared.contains("__SAFE_CHAINS_");
                if unknown || nameref(&name, &text) {
                    return any();
                }
                if text.starts_with(['-', '+']) {
                    continue;
                }
                names.push(declared.to_string());
            }
            names
        }
        _ => Vec::new(),
    }
}

/// `$((X = 1))`, `$((X += 1))`, `$((X++))`: arithmetic that assigns.
fn assigns_in_arithmetic(word: &Word) -> bool {
    word.0.iter().any(|part| match part {
        WordPart::Arith(body) => {
            let text = body.eval().replace("==", "").replace("!=", "").replace("<=", "").replace(">=", "");
            text.contains(['=']) || text.contains("++") || text.contains("--") || assigns_in_arithmetic(body)
        }
        WordPart::DQuote(inner) => assigns_in_arithmetic(inner),
        _ => false,
    })
}

/// `declare -n REF=NAME` makes a later `REF=…` set `NAME`, so which variable changes is not
/// the one written.
fn nameref(command: &str, arg: &str) -> bool {
    command != "read" && arg.starts_with(['-', '+']) && arg.contains('n')
}

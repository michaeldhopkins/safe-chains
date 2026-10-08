//! What the values behind a loop variable, a piped stream or a substitution can look like.

use super::walk::GLOB_CHARS;
use super::{BLANK, GLOB, Lead, OPAQUE, SPLIT as WORD_SPLIT, shape};
use crate::cst::{Cmd, Script, Word};
use crate::parse::Token;
use crate::pathctx::item_shape::{EMPTY, LEAD, SPLIT, UNKNOWN};
use crate::pathctx::{ItemShape, ItemShapeGuard};

/// A stream representative no stage names, so a stage that hands it back unchanged is one that
/// passes its input through.
const PASSTHROUGH: &str = "\u{1}safe-chains-passthrough";

/// The shape of the values a `for` variable takes from its list.
pub(crate) fn items_shape(items: &[Word]) -> ItemShape {
    let mut out = ItemShape::NONE;
    for item in items {
        let tokens = item.expand();
        let s = shape(item, tokens.len());
        let texts = s.concrete.clone().unwrap_or(tokens);
        let globs = texts.iter().any(|t| t.contains(GLOB_CHARS));
        let listed = s.concrete.is_some() && s.facts.has(GLOB);
        let unmatched = listed && texts == [item.eval()];
        let leads_a_glob = ((listed && !unmatched) || !s.facts.has(GLOB)) && texts.iter().any(|t| t.starts_with(GLOB_CHARS));
        out.set(LEAD, s.lead != Lead::No || s.facts.has(WORD_SPLIT) || leads_a_glob || texts.iter().any(|t| t.starts_with('-')));
        out.set(
            SPLIT,
            s.facts.has(BLANK) || (s.facts.has(GLOB) && !listed) || globs || texts.iter().any(|t| t.contains(char::is_whitespace)),
        );
        out.set(EMPTY, s.facts.has(OPAQUE) || texts.iter().any(String::is_empty));
    }
    out
}

/// The shape of the items a pipeline stage writes, given the shape of what it reads. `find`'s
/// items each begin with the root it was given, `echo` writes its own known words, `which` absolute
/// paths, `id -u` a number or a name, `pwd` the
/// working directory (as trusted as `$PWD`), and a line-preserving filter passes its input
/// through. Anything else could print anything.
pub(crate) fn stage_shape(cmd: &Cmd, input: ItemShape) -> ItemShape {
    let Cmd::Simple(s) = cmd else {
        return UNKNOWN;
    };
    let name = s.words.first().map(|w| Token::from_raw(w.eval()).command_name().to_string()).unwrap_or_default();
    let filter = matches!(name.as_str(), "sort" | "uniq" | "cat" | "tac" | "head" | "tail" | "tee");
    if filter && crate::cst::check::stage_output_repr(cmd, Some(PASSTHROUGH)) == PASSTHROUGH {
        return input;
    }
    let args = &s.words[1.min(s.words.len())..];
    let unknown = |w: &Word| {
        let s = shape(w, w.expand().len());
        s.facts.has(OPAQUE | GLOB) || s.concrete.is_some()
    };
    if args.iter().any(unknown) {
        return UNKNOWN;
    }
    let texts: Vec<String> = args.iter().flat_map(Word::expand).collect();
    let texts_str: Vec<&str> = texts.iter().map(String::as_str).collect();
    match name.as_str() {
        "find" if prints_only_paths(&texts_str) => SPLIT,
        "pwd" => ItemShape::NONE,
        "id" if !texts.is_empty()
            && texts
                .iter()
                .all(|t| matches!(t.as_str(), "-u" | "-g" | "-n" | "-r" | "-un" | "-gn" | "-nu" | "-ng" | "-ur" | "-gr")) =>
        {
            ItemShape::NONE
        }
        "which" => SPLIT | EMPTY,
        "echo" if !texts.iter().any(|t| t.starts_with('-') || t.contains('\\')) => {
            ItemShape::when(SPLIT, texts.iter().any(|t| t.contains(char::is_whitespace))) | ItemShape::when(EMPTY, texts.is_empty())
        }
        _ => UNKNOWN,
    }
}

/// The shape of what a substitution prints: what the last stage of its one pipeline writes.
pub(super) fn substitution_shape(script: &Script) -> ItemShape {
    let [stmt] = script.0.as_slice() else {
        return UNKNOWN;
    };
    if stmt.pipeline.bang || !matches!(stmt.op, None | Some(crate::cst::ListOp::Semi)) {
        return UNKNOWN;
    }
    let mut items = UNKNOWN;
    for cmd in &stmt.pipeline.commands {
        items = stage_shape(cmd, items);
    }
    items
}

/// Binds a `for` variable's locus representatives and the shape of its list's values together.
pub(crate) fn enter_loop(var: &str, items: &[Word], read: String, write: String) -> (crate::pathctx::LoopGuard, ItemShapeGuard) {
    (crate::pathctx::enter_loop_var(var.to_string(), read, write), crate::pathctx::enter_loop_shape(var.to_string(), items_shape(items)))
}

/// Binds a `while read` variable to the piped items. `read` keeps a line's inner blanks, so an
/// unquoted use can split whatever the items are.
pub(crate) fn enter_read_var(var: String, repr: &str) -> (crate::pathctx::LoopGuard, ItemShapeGuard) {
    let shape = ItemShape::when(LEAD, crate::pathctx::stdin_shape().has(LEAD)) | SPLIT | EMPTY;
    (crate::pathctx::enter_loop_var(var.clone(), repr.to_string(), repr.to_string()), crate::pathctx::enter_loop_shape(var, shape))
}

/// Whether a `find`/`fd` invocation writes only the paths it found. An action that runs a command
/// (`-exec echo /etc/shadow`, `fd -x cat`) or formats its own lines (`-printf`, `--format`) writes
/// whatever it likes.
pub(crate) fn prints_only_paths(args: &[&str]) -> bool {
    const ACTIONS: &[&str] = &[
        "-exec", "-execdir", "-ok", "-okdir", "-printf", "-fprintf", "-ls", "-fls", "-fprint", "-fprint0", "-x", "--exec", "-X",
        "--exec-batch", "--format", "-l", "--list-details",
    ];
    !args.iter().any(|a| ACTIONS.contains(&a.split('=').next().unwrap_or(a)))
}

/// Binds the stream a pipeline stage reads: where its items point and what they can look like.
pub(crate) fn enter_stdin(repr: String, items: ItemShape) -> (crate::pathctx::StdinReprGuard, ItemShapeGuard) {
    (crate::pathctx::enter_stdin_repr(repr), crate::pathctx::enter_stdin_shape(items))
}

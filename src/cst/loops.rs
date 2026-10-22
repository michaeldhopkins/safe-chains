//! Loop bodies: run once per item, and each pass after what the previous one left behind.
//!
//! A `for` over literal words is classified once per word, with the variable bound to that word,
//! and is approved only if every pass is. A handler then judges `awk "$p"` on the program `$p` will
//! hold, not on the spelling `$p`, which reads as harmless text in a program, a path or a header.
//! Any other list (a glob, a substitution, an unknown variable) leaves the variable opaque: its
//! locus is the list's representative, and its words are probed as unknown (`opaque`).
//!
//! A body also runs after itself. What one pass assigns, defines or changes directory to is in
//! force at the start of the next, so those names, functions and the directory are uncertain
//! while the body is judged: `q=./a; for i in 1 2; do cat "$q"; q=/etc/shadow; done` reads
//! `/etc/shadow` on its second pass.

use super::check::{
    MAX_CD_SCAN_DEPTH, ShellEffects, charge_classify_work, poison_function, redirect_verdict, scan_script_effects, script_verdict,
    words_sub_verdict,
};
use super::{Cmd, Script, Word, WordPart};
use crate::verdict::Verdict;

pub(super) fn for_verdict(var: &str, items: &[Word], body: &Script, redirs: &[super::Redir]) -> Verdict {
    let redir_v = redirect_verdict(redirs).combine(crate::envvars::loop_var_verdict(var, items));
    if let Verdict::Denied = redir_v {
        return Verdict::Denied;
    }
    let items_v = words_sub_verdict(items);
    let _carried = Carried::enter(&[body], Some(var));
    let body_v = match literal_values(items) {
        Some(values) => each_value(var, items, body, &values),
        None => opaque_items(var, items, body),
    };
    items_v.combine(body_v).combine(redir_v)
}

pub(super) fn while_verdict(cond: &Script, body: &Script, redirs: &[super::Redir]) -> Verdict {
    let redir_v = redirect_verdict(redirs);
    if let Verdict::Denied = redir_v {
        return Verdict::Denied;
    }
    let _carried = Carried::enter(&[cond, body], None);
    let cond_v = script_verdict(cond);
    // `while read VAR; do … "$VAR" …` binds each read var to the piped stdin's item locus, as a
    // `for` binds its list var. Only when a modeled source set the stdin repr; otherwise the vars
    // stay unbound (fail-closed).
    let _binds: Vec<_> = match crate::pathctx::stdin_item_repr() {
        Some(repr) => read_loop_vars(cond).into_iter().map(|v| super::opaque::enter_read_var(v, &repr)).collect(),
        None => Vec::new(),
    };
    cond_v.combine(script_verdict(body)).combine(redir_v)
}

/// The body classified once per distinct value, the variable bound to it. Each pass draws on the
/// classification budget, so a long list fails closed instead of stalling the hook.
fn each_value(var: &str, items: &[Word], body: &Script, values: &[String]) -> Verdict {
    let mut verdict = Verdict::Allowed(crate::verdict::SafetyLevel::Inert);
    for value in values {
        if !charge_classify_work(1) {
            return Verdict::Denied;
        }
        let _bound = crate::pathctx::enter_var(var.to_string(), value.clone());
        verdict = verdict.combine(super::netargs::with_loop(var, items, body, || script_verdict(body)));
        if !verdict.is_allowed() {
            return verdict;
        }
    }
    verdict
}

fn opaque_items(var: &str, items: &[Word], body: &Script) -> Verdict {
    let item_strs: Vec<String> = items.iter().map(Word::eval).collect();
    match crate::engine::resolve::loop_reprs(&item_strs) {
        Some((read_repr, write_repr)) => {
            let _g = super::opaque::enter_loop(var, items, read_repr, write_repr);
            super::netargs::with_loop(var, items, body, || script_verdict(body))
        }
        None => script_verdict(body),
    }
}

/// The values a `for` list gives its variable, when every one is written out in the command:
/// literal text, quoted or not, brace expansion included, or a variable whose value is certain.
/// `None` when any word holds an unknown expansion, a substitution, a glob character (quoted or
/// not, since an unquoted `$f` is globbed) or a `~`, or when the
/// list is empty (`for x; do` walks the positional parameters).
pub(super) fn literal_values(items: &[Word]) -> Option<Vec<String>> {
    if items.is_empty() {
        return None;
    }
    let mut values: Vec<String> = Vec::new();
    for item in items {
        let words = if literal_word(item) { item.expand() } else { super::opaque::known_words(item)? };
        if words.iter().any(|w| w.contains(['*', '?', '['])) {
            return None;
        }
        for value in words {
            if !values.contains(&value) {
                values.push(value);
            }
        }
    }
    Some(values)
}

fn literal_word(word: &Word) -> bool {
    word.0.iter().all(|part| match part {
        WordPart::Lit(s) => !s.contains(['$', '*', '?', '[', '~']),
        WordPart::DQuote(inner) => inner.0.iter().all(|p| match p {
            WordPart::Lit(s) => !s.contains('$'),
            WordPart::Escape(_) => true,
            _ => false,
        }),
        WordPart::SQuote(_) | WordPart::AnsiC(_) | WordPart::Escape(_) => true,
        WordPart::CmdSub(_) | WordPart::ProcSub(_) | WordPart::Backtick(_) | WordPart::Arith(_) => false,
    })
}

/// The variables a `while`/`until` condition of the form `read VAR…` (incl. `IFS= read -r VAR`) binds
/// from stdin — its non-flag positionals — so the body's `$VAR` can be gated at the pipe's item locus.
/// Empty for any other condition.
fn read_loop_vars(cond: &Script) -> Vec<String> {
    let [stmt] = cond.0.as_slice() else {
        return Vec::new();
    };
    let [Cmd::Simple(s)] = stmt.pipeline.commands.as_slice() else {
        return Vec::new();
    };
    let words: Vec<String> = s.words.iter().map(Word::eval).collect();
    if words.first().map(String::as_str) != Some("read") {
        return Vec::new();
    }
    words[1..].iter().filter(|w| !w.starts_with('-')).cloned().collect()
}

/// What an earlier pass of a loop may have left in the shell, held uncertain while its body is
/// judged: every name the body may set (but the `for` variable, which each pass sets afresh), the
/// working directory when the body may change it, and every function it may redefine.
struct Carried {
    _vars: Vec<crate::pathctx::VarGuard>,
    _cwd: Option<crate::pathctx::Guard>,
}

impl Carried {
    fn enter(scripts: &[&Script], loop_var: Option<&str>) -> Self {
        let mut effects = ShellEffects::default();
        let mut seen = Vec::new();
        let mut defined = Vec::new();
        for script in scripts {
            scan_script_effects(script, MAX_CD_SCAN_DEPTH, &mut seen, &mut effects);
            defined_bodies(script, &mut defined);
        }
        for body in defined {
            scan_script_effects(body, MAX_CD_SCAN_DEPTH, &mut seen, &mut effects);
        }
        for name in effects.funcs {
            poison_function(name);
        }
        let mut vars = Vec::new();
        super::scope_budget::rebind_uncertain(effects.vars.into_iter().filter(|n| Some(n.as_str()) != loop_var), &mut vars);
        let cwd = effects.cwd.then(|| crate::pathctx::enter_cwd(Some(crate::pathctx::UNRESOLVED_CWD.to_string())));
        Carried { _vars: vars, _cwd: cwd }
    }
}

/// The bodies of the functions a script defines, at any depth: a pass may define one and call it,
/// so what it does is in force on the next pass, though no call to it resolves before the body runs.
fn defined_bodies<'a>(script: &'a Script, out: &mut Vec<&'a Script>) {
    for cmd in script.0.iter().flat_map(|st| &st.pipeline.commands) {
        match cmd {
            Cmd::FunctionDef { body, .. } => {
                out.push(body);
                defined_bodies(body, out);
            }
            Cmd::Subshell { body, .. } | Cmd::BraceGroup { body, .. } | Cmd::For { body, .. } => defined_bodies(body, out),
            Cmd::While { cond, body, .. } | Cmd::Until { cond, body, .. } => {
                defined_bodies(cond, out);
                defined_bodies(body, out);
            }
            Cmd::If { branches, else_body, .. } => {
                for branch in branches {
                    defined_bodies(&branch.cond, out);
                    defined_bodies(&branch.body, out);
                }
                if let Some(body) = else_body {
                    defined_bodies(body, out);
                }
            }
            Cmd::Case { arms, .. } => arms.iter().for_each(|arm| defined_bodies(&arm.body, out)),
            Cmd::Simple(_) | Cmd::DoubleBracket { .. } => {}
        }
    }
}

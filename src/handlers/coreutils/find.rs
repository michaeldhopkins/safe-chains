use crate::engine::resolve::frozen_reach;
use crate::engine::resolve::frozen_walk::Walk;
use crate::parse::Token;
use crate::policy::FlagSet;
use crate::verdict::{SafetyLevel, Verdict};

// find's expression is an ALLOWLIST of READ-ONLY primaries: tests (`-name`/`-type`/`-size`/…),
// read-only actions (`-print`/`-ls`/`-prune`/…), positional + global options, and operators. A
// primary NOT listed — `-ok`/`-okdir`, `-fprint*`/`-fls`, or any new/BSD write primary —
// denies by OMISSION (fail closed), where the old denylist failed open on anything it hadn't
// enumerated. `-exec`/`-execdir` and `-delete` are handled separately (they delegate, so the
// traversal locus decides them). A VALUED
// primary consumes its next token, so a `-`-prefixed VALUE (`-mtime -7`, `-perm -644`) is not
// mistaken for a primary and a filename that looks like an action (`find . -name -delete`) stays a
// value. SAFETY (as in mlr): a value-taking primary MUST be in VALUED, never STANDALONE, or the
// walk would fail to skip its value — over-denying at best, or (if the value were an action)
// swallowing it silently.

pub(in crate::handlers::coreutils) fn is_safe_find(tokens: &[Token]) -> Verdict {
    // find's `{}` placeholder is substituted with each traversed path, which lives UNDER
    // find's path operand. So the nested command's locus is find's traversal scope, not a
    // bare worktree name: `find /etc -exec rm {}` deletes /etc files, not `./file`. Bind `{}`
    // to a path under each find operand and delegate once per operand (deny-absorbing), so a
    // system/home traversal denies. Default operand `.` when none is given (find's default).
    let bases: Vec<&str> = {
        // Skip the leading GLOBAL options (`-H`/`-L`/`-P`/`-D debugopts`/`-O[level]`), which
        // precede the path operand — otherwise `find -L /etc -exec …` stops at `-L`, defaults
        // the base to the cwd, and binds `{}` to a *worktree* path while find actually
        // traverses /etc. Collect ALL leading path operands (deny-absorbing below), so even a
        // mis-counted option arg still surfaces the real path.
        let mut i = 1;
        while i < tokens.len() {
            match tokens[i].as_str() {
                "-H" | "-L" | "-P" => i += 1,
                "-D" => i += 2,
                s if s.starts_with("-O") => i += 1,
                _ => break,
            }
        }
        // `-D` advances by 2 (option + debugopts arg); a TRAILING `-D` with no arg pushes `i` past
        // the end, so slice with `get` rather than `tokens[i..]` (which would panic out of range).
        let leading: Vec<&str> = tokens
            .get(i..)
            .unwrap_or(&[])
            .iter()
            .take_while(|t| !t.as_str().starts_with('-'))
            .map(Token::as_str)
            .collect();
        if leading.is_empty() { vec!["."] } else { leading }
    };

    // The primary vocabulary is DATA and lives in commands/search/find.toml. Absent it, nothing is
    // recognised, so fail closed rather than fall through to the deny-by-omission arm and pretend
    // every primary was simply unknown.
    let Some((safe_standalone, safe_valued)) = crate::registry::fallback_flag_sets("find") else {
        return Verdict::Denied;
    };

    let words: Vec<&str> = tokens[1..].iter().map(Token::as_str).collect();
    let skip = crate::handlers::coreutils::find_skip::skip_of(&words);
    let walk = Walk { skip: &skip, ..Walk::items(true, frozen_reach::find_follows(&words)) };
    let mut level = SafetyLevel::Inert;
    let mut i = 1;
    while i < tokens.len() {
        let s = tokens[i].as_str();
        // `-exec`/`-execdir`: delegate to the inner command, bound to each traversal base.
        if s == "-exec" || s == "-execdir" {
            let cmd_start = i + 1;
            let cmd_end = tokens[cmd_start..]
                .iter()
                .position(|t| *t == ";" || *t == "+")
                .map(|p| cmd_start + p)
                .unwrap_or(tokens.len());
            let exec: Vec<&str> = tokens[cmd_start..cmd_end].iter().map(Token::as_str).collect();
            if cmd_start >= cmd_end || crate::handlers::fd::climbs_past_placeholder(&exec) {
                return Verdict::Denied;
            }
            for base in &bases {
                let mut runs_in = vec![(crate::pathctx::cwd(), *base)];
                if s == "-execdir" {
                    if base.trim_end_matches('/').rsplit('/').next() == Some("..") {
                        return Verdict::Denied;
                    }
                    let item_base = if base.starts_with(['/', '~', '$']) { *base } else { "." };
                    runs_in.extend(execdir_directories(base).into_iter().map(|dir| (dir, item_base)));
                }
                for (dir, item_base) in runs_in {
                    let _there = crate::pathctx::enter_cwd(dir);
                    match judge_exec(&exec, item_base, walk) {
                        Verdict::Denied => return Verdict::Denied,
                        Verdict::Allowed(l) => level = level.max(l),
                    }
                }
            }
            i = cmd_end + 1;
            continue;
        }
        // `-delete` removes every match, so it is a DESTROY bound to the traversal bases exactly
        // as `-exec rm {}` is. It deliberately does NOT live in the read-only standalone list:
        // membership there costs nothing, so it would be admitted with no locus reasoning at all.
        // Routing it through the same delegation is what makes the two spellings of one operation
        // agree STRUCTURALLY — one code path, the same bases — instead of by a test that can
        // drift, and `find / -delete` inherits its refusal the way `find / -exec rm {}` already
        // has one. Modelled as `rm -r`, not `rm`: `-delete` implies `-depth` and removes
        // directories bottom-up, so the recursive spelling is both the accurate analogue and the
        // more conservative one.
        if s == "-delete" {
            for base in &bases {
                let judge = |bound: &str| crate::command_verdict(&shell_words::join(["rm", "-r", bound]));
                match frozen_reach::judged_over(base, walk, judge) {
                    Verdict::Denied => return Verdict::Denied,
                    Verdict::Allowed(l) => level = level.max(l),
                }
            }
            i += 1;
            continue;
        }
        // A path operand, a primary's value, or an operator (`(` `)` `!` `,`).
        if !s.starts_with('-') {
            i += 1;
            continue;
        }
        // Global query-optimisation level `-O2`/`-Ofind` (glued).
        if s.starts_with("-O") {
            i += 1;
            continue;
        }
        // A read-only VALUED primary consumes its value; `-newer`/`-newerXY` take a reference.
        if safe_valued.contains_flag(s) || s.starts_with("-newer") {
            i += 2;
            continue;
        }
        if safe_standalone.contains_flag(s) {
            i += 1;
            continue;
        }
        // Anything else — `-ok*`, `-fprint*`/`-fls`, or an unknown/newer primary — denies.
        return Verdict::Denied;
    }
    Verdict::Allowed(level)
}

fn judge_exec(exec: &[&str], base: &str, walk: Walk) -> Verdict {
    let judge = |item: &str, shield: bool| {
        let words: Vec<String> = exec
            .iter()
            .map(|w| {
                let bound = w.replace("{}", item);
                if shield { crate::handlers::fd::synthetic_operand(&bound, w, item) } else { bound }
            })
            .collect();
        let _mark = crate::cst::netargs::enter(true);
        crate::command_verdict(&shell_words::join(&words))
    };
    let verdict = frozen_reach::judged_over(base, walk, |bound| {
        let item = if bound.ends_with("/f") { format!("{bound}{}", crate::parse::UNKNOWN_CONTENT) } else { bound.to_string() };
        judge(&item, true)
    });
    // A match's name is text nobody has seen, so the command must also stand one that is code.
    let embedded = exec.iter().any(|w| w.contains("{}") && *w != "{}");
    crate::cst::opaque::refused_as_any_content(verdict, embedded, |probe| judge(probe, false))
}

/// The directories `-execdir` runs its command in for one search path: the directory holding the
/// path itself (`dirname`, as find spells it, so `..` and `.` both run in `.`), and the path, which
/// holds its first level of matches. Deeper matches run further down the same tree. A path whose
/// parent cannot be named gives a directory no relative path resolves in.
/// The caller also judges from the working directory, since implementations differ on where the
/// search path itself runs.
fn execdir_directories(base: &str) -> Vec<Option<String>> {
    let cwd = crate::pathctx::cwd();
    let unnamed = || Some(crate::pathctx::UNRESOLVED_CWD.to_string());
    let trimmed = base.trim_end_matches('/');
    let parent = match trimmed.rfind('/') {
        _ if trimmed.is_empty() => Some("/"),
        None if trimmed.starts_with('~') || trimmed.contains('$') || trimmed.contains(crate::cst::eval::TAGGED_PREFIX) => None,
        None => Some("."),
        Some(at) if trimmed[at..].contains('$') || trimmed[at..].contains(crate::cst::eval::TAGGED_PREFIX) => None,
        Some(at) => Some(match trimmed[..at].trim_end_matches('/') {
            "" => "/",
            head => head,
        }),
    };
    let parent = parent.map_or_else(unnamed, |p| crate::pathctx::join_cwd(cwd.as_deref(), p).or_else(|| cwd.clone()));
    let own = crate::pathctx::join_cwd(cwd.as_deref(), base).or_else(|| cwd.clone());
    vec![parent, own]
}

pub(in crate::handlers::coreutils) fn dispatch(cmd: &str, tokens: &[Token]) -> Option<Verdict> {
    match cmd {
        "find" => Some(is_safe_find(tokens)),
        _ => None,
    }
}

pub(in crate::handlers::coreutils) fn command_docs() -> Vec<crate::docs::CommandDoc> {
    vec![crate::docs::CommandDoc::handler(
        "find",
        "https://www.gnu.org/software/findutils/manual/html_mono/find.html",
        "Read-only predicates and actions allowed (tests like -name/-type/-size, -print/-ls/-prune, \
             operators, positional and global options). -exec/-execdir allowed when the executed \
             command is itself safe (each `{}` binds to the traversal path).",
        "fs",
    )]
}

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        find_name: "find . -name '*.rb'",
        find_type_name: "find . -type f -name '*.py'",
        find_maxdepth: "find /tmp -maxdepth 2",
        find_print: "find . -name '*.log' -print",
        find_print0: "find . -name '*.log' -print0",
        find_exec_grep_l: "find . -name '*.rb' -exec grep -l pattern {} \\;",
        find_exec_grep_l_plus: "find . -name '*.rb' -exec grep -l pattern {} +",
        find_exec_cat: "find . -exec cat {} \\;",
        find_execdir_cat: "find . -execdir cat {} \\;",
        find_execdir_grep: "find . -execdir grep pattern {} \\;",
        find_exec_grep_safe: "find . -name '*.py' -exec grep pattern {} +",
        find_exec_nested_bash_safe: "find . -exec bash -c 'git status' \\;",
        // engine-authoritative: deleting WORKTREE files via -exec is admitted at developer
        // ({} binds to the find path, so these stay inside the worktree).
        find_exec_rm: "find . -exec rm {} \\;",
        find_exec_rm_rf: "find . -exec rm -rf {} +",
        // `-delete` delegates like `-exec rm -r {}`, so a WORKTREE base is admitted for the same
        // reason and an out-of-worktree base still refuses (see the denied list).
        find_delete: "find . -name '*.tmp' -delete",
        find_type_delete: "find . -type f -name '*.bak' -delete",
        find_delete_after_valued: "find . -mtime -7 -delete",
        find_execdir_unsafe: "find . -execdir rm {} \\;",
        // Allowlist coverage: `-`-prefixed VALUES of valued primaries aren't mistaken for primaries.
        find_size_negative: "find . -size -1M",
        find_mtime_negative: "find . -mtime -7 -type f",
        find_perm_dash: "find . -perm -644",
        find_newer: "find . -newer ref.txt",
        find_newermt: "find . -newermt 2024-01-01",
        find_mindepth_maxdepth: "find . -mindepth 1 -maxdepth 3 -type d",
        find_empty: "find . -empty",
        find_not_operator: "find . -not -name '*.tmp'",
        find_or_operator: "find . -name '*.rb' -o -name '*.py'",
        find_printf: "find . -printf '%p\\n'",
        find_help: "find --help",
        // A filename that looks like an action is just the value of `-name` (the old denylist
        // false-DENIED this; the allowlist skips the value correctly).
        find_name_dash_delete_value: "find . -name -delete",
        // Regression (fuzzer, crash-*): a TRAILING `-D` (advances the leading-option scan by 2)
        // overran `tokens[i..]` and panicked the hook. All 6 crash inputs minimize to a bare `-D`
        // (or global opts) as the last token; each is a harmless no-op find that must not panic.
        find_trailing_D: "find -D",
        find_global_L_trailing_D: "find -L -D",
        find_global_H_P_trailing_D: "find -H -P -D",
    }

    denied! {
        find_delete_outside_denied: "find /etc -name '*.tmp' -delete",
        // a leading global option must not hide the real path: base is /etc, not the cwd.
        find_global_opt_L_system: "find -L /etc -exec rm -rf {} \\;",
        find_global_opt_P_system: "find -P / -exec rm {} \\;",
        find_global_opt_O_system: "find -O3 /etc -exec rm {} \\;",
        find_global_opt_D_system: "find -D tree /etc -exec rm {} \\;",
        find_ok_denied: "find . -ok rm {} \\;",
        find_okdir_denied: "find . -okdir rm {} \\;",
        find_exec_nested_bash_chain_denied: "find . -exec bash -c 'ls && rm -rf /' \\;",
        find_type_delete_home_denied: "find ~ -type f -name '*.bak' -delete",
        find_fprint_denied: "find . -fprint /tmp/list.txt",
        find_fprint0_denied: "find . -fprint0 /tmp/list.txt",
        find_fls_denied: "find . -fls /tmp/list.txt",
        find_fprintf_denied: "find . -fprintf /tmp/list.txt '%p'",
        // Allowlist fail-closed wins: an unknown/newer/BSD primary denies by OMISSION (the old
        // denylist would have allowed anything it hadn't enumerated).
        find_unknown_primary_denied: "find . -frobnicate",
        find_delete_after_valued_outside_denied: "find /etc -mtime -7 -delete",
        find_unknown_action_after_test_denied: "find . -type f -flushcache",
    }
}

#[cfg(test)]
mod proptests {
    // Property tests for the pure-logic functions in this file: `dispatch` and `is_safe_find`.
    // Each property asserts a real invariant of one of those functions:
    //   - `dispatch` is a thin router: a property states its exact routing contract.
    //   - `is_safe_find` is the read-only allowlist walk; properties state the deny-by-omission
    //     arm (a non-allow-listed primary forces Denied), and the read-only-monotone case
    //     (a known-safe primary appended to an already-allowed expression does not flip it).
    use super::{dispatch, is_safe_find};
    use crate::parse::Token;
    use crate::verdict::Verdict;
    use proptest::prelude::*;

    /// A token composed solely of read-only allow-listed standalone primaries (`-print`,
    /// `-print0`, `-ls`, `-prune`, `-quit`, `-readable`, `-writable`, `-executable`, `-empty`,
    /// `-true`, `-false`, `-depth`, `-xdev`, …) and a benign `-`-free tail. None of these
    /// performs a write that crosses the worktree, none takes a value, and none changes the
    /// predicate's verdict beyond the read-only band. Composing them is the property the
    /// walk's "standalone advances by 1" branch promises.
    fn arb_read_only_primary() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("-print".to_string()),
            Just("-print0".to_string()),
            Just("-ls".to_string()),
            Just("-prune".to_string()),
            Just("-quit".to_string()),
            Just("-readable".to_string()),
            Just("-writable".to_string()),
            Just("-executable".to_string()),
            Just("-empty".to_string()),
            Just("-true".to_string()),
            Just("-false".to_string()),
            Just("-depth".to_string()),
            Just("-xdev".to_string()),
        ]
    }

    /// A token that the deny-by-omission arm catches: starts with `-`, is NOT a leading global
    /// option (`-H`/`-L`/`-P`/`-D`/`-O*`), is NOT `-exec`/`-execdir`/`-delete` (which have
    /// dedicated delegation branches), and is NOT in the standalone or valued allowlists in
    /// `commands/search/find.toml`. Such a token is a primary the registry doesn't know, so
    /// the walk must hit the explicit `Verdict::Denied` arm at the bottom of the loop.
    fn arb_unknown_primary() -> impl Strategy<Value = String> {
        prop_oneof![
            // outright unknowns (the BSD/macOS `-fprint` family is in the engine's interceptor
            // layer too, but here we hit the handler's own allowlist-fail-closed branch).
            Just("-frobnicate".to_string()),
            Just("-flushcache".to_string()),
            Just("-z".to_string()),
            Just("-E".to_string()),
            // well-known deny-by-omission primaries documented in find's toml `[command.output]`.
            Just("-fprint".to_string()),
            Just("-fprint0".to_string()),
            Just("-fprintf".to_string()),
            Just("-fls".to_string()),
            Just("-ok".to_string()),
            Just("-okdir".to_string()),
        ]
    }

    /// An arbitrary token the walk must accept as a benign operand: arbitrary-but-non-empty text
    /// that does not start with `-`, so it gets classified as a path operand or a valued
    /// primary's value rather than a primary. Used to vary the suffix without forcing the walk
    /// into a deny branch.
    fn arb_benign_operand() -> impl Strategy<Value = String> {
        "[a-zA-Z0-9_./]{1,16}".prop_filter("must not start with a dash", |s| !s.starts_with('-'))
    }

    proptest! {
        // Differential. `dispatch`'s contract is a one-arm router on `cmd`: it forwards
        // `find` to `is_safe_find` and returns `None` for anything else. Wrapping the
        // forward in `Some(_)` must therefore mirror `is_safe_find` exactly. A regression
        // that lost the `Some(...)` wrapper, swapped arms, or skipped the call would fail
        // here on the very first generated input.
        #[test]
        fn dispatch_routes_find_to_is_safe_find_with_some_wrapper(
            args in proptest::collection::vec(any::<String>(), 0..16),
        ) {
            let mut tokens: Vec<Token> = Vec::with_capacity(args.len() + 1);
            tokens.push(Token::from_test("find"));
            for a in &args {
                tokens.push(Token::from_test(a));
            }
            proptest::prop_assert_eq!(
                dispatch("find", &tokens),
                Some(is_safe_find(&tokens)),
                "tokens after program name: {:?}",
                &args[..],
            );
        }

        // Differential (complement). The non-`find` arm of `dispatch` returns `None` for
        // every input — there is no path by which a non-`find` command name ever reaches a
        // verdict. A regression that defaulted to `Some(Allowed(Inert))` instead of `None`
        // would let every command through `find.rs` and silently bypass the rest of the
        // classifier. The cmd list is built as `String` (`select` over `&str` would yield
        // `&str` and trip `needless_borrow`); `cmd.as_str()` then gives the uniform `&str`
        // both consumers need.
        #[test]
        fn dispatch_returns_none_for_every_non_find_command(
            cmd in proptest::sample::select(vec![
                "".to_string(), "ls".to_string(), "cat".to_string(),
                "grep".to_string(), "tar".to_string(),
                "FIND".to_string(), "FIND ".to_string(), "Find".to_string(),
                "findutils".to_string(), "-find".to_string(), "_find".to_string(),
                "find-extra".to_string(), "xfind".to_string(),
            ]),
            args in proptest::collection::vec(any::<String>(), 0..8),
        ) {
            let mut tokens: Vec<Token> = Vec::with_capacity(args.len() + 1);
            tokens.push(Token::from_test(cmd.as_str()));
            for a in &args {
                tokens.push(Token::from_test(a));
            }
            proptest::prop_assert_eq!(
                dispatch(cmd.as_str(), &tokens),
                None,
                "non-find cmd `{}` must return None, got Some",
                cmd,
            );
        }

        // Invariant (allowlist fail-closed). The walk's terminal branch is the explicit
        // `return Verdict::Denied` for any primary the registry doesn't list. Promising
        // that branch is reachable is the *point* of the allowlist — the old denylist
        // falsely admitted primaries it hadn't enumerated. For any expression made of
        // a benign worktree base plus a deny-by-omission primary (anywhere — leading,
        // middle, trailing), `is_safe_find` must deny. A regression that accidentally
        // swallowed the primary, treated it as a value, or added it to the allowlist
        // would surface here.
        #[test]
        fn is_safe_find_denies_any_unknown_primary_regardless_of_position(
            unknown in arb_unknown_primary(),
            position in 0usize..=2,
            suffix in proptest::collection::vec(arb_read_only_primary(), 0..4),
        ) {
            // Build the expression: `find . [read-only prefix] <unknown> [read-only suffix]`.
            // `position` selects whether the unknown primary sits first, second, or last
            // (position 0/1 within the test primaries), so a position-sensitive bug in the
            // walk (off-by-one after a valued primary, e.g.) fails the property.
            let mut tokens: Vec<Token> = vec![Token::from_test("find"), Token::from_test(".")];
            match position {
                0 => {
                    tokens.push(Token::from_test(&unknown));
                    tokens.extend(suffix.iter().map(|s| Token::from_test(s)));
                }
                _ => {
                    tokens.push(Token::from_test("-print"));
                    if position == 1 {
                        tokens.push(Token::from_test(&unknown));
                    }
                    tokens.extend(suffix.iter().map(|s| Token::from_test(s)));
                    if position == 2 {
                        tokens.push(Token::from_test(&unknown));
                    }
                }
            }
            proptest::prop_assert_eq!(
                is_safe_find(&tokens),
                Verdict::Denied,
                "deny-by-omission primary `{}` at position {} of {:?} was not denied",
                unknown,
                position,
                &tokens[..],
            );
        }

        // Metamorphic. Once `is_safe_find` has classified an expression as Allowed, appending
        // a read-only standalone primary (one whose allowlist presence advances the cursor
        // by exactly one token and adds no destructive capacity) must keep the verdict
        // unchanged. The test composes a benign worktree expression with an arbitrary
        // read-only primary tail and asserts the tail-appended expression classifies
        // identically — Allowed iff Allowed, exactly the property the walk's
        // `safe_standalone.contains_flag(s)` branch implements. A regression that, say,
        // re-classified the expression from scratch on every primary (or made the tail's
        // safety decision depend on what came before) would flip the verdict here.
        #[test]
        fn is_safe_find_verdict_is_invariant_under_appending_a_read_only_standalone_primary(
            prefix_primaries in proptest::collection::vec(arb_read_only_primary(), 0..4),
            tail_primary in arb_read_only_primary(),
            operand in arb_benign_operand(),
        ) {
            let base = {
                let mut tokens: Vec<Token> = vec![Token::from_test("find"), Token::from_test(".")];
                for p in &prefix_primaries {
                    tokens.push(Token::from_test(p));
                }
                tokens.push(Token::from_test(&operand));
                tokens
            };
            let tail = {
                let mut tokens = base.clone();
                tokens.push(Token::from_test(&tail_primary));
                tokens
            };
            proptest::prop_assert_eq!(
                is_safe_find(&base),
                is_safe_find(&tail),
                "appending `{}` to {:?} flipped the verdict: base -> {:?}, tail -> {:?}",
                tail_primary,
                &base[..],
                is_safe_find(&base),
                is_safe_find(&tail),
            );
        }
    }
}

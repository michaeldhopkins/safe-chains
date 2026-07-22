use super::*;
use proptest::prelude::*;

const SHELL_KEYWORDS: &[&str] = &[
    "do", "done", "for", "while", "until", "if", "then", "elif", "else", "fi", "in", "case",
    "esac", "select",
];

fn starts_with_keyword(s: &str) -> bool {
    SHELL_KEYWORDS.iter().any(|kw| {
        s.starts_with(kw)
            && !s
                .as_bytes()
                .get(kw.len())
                .is_some_and(|&b| b.is_ascii_alphanumeric() || b == b'_')
    })
}

fn arb_shell_word() -> impl Strategy<Value = String> {
    prop::string::string_regex("[a-zA-Z0-9_./-]+")
        .expect("valid regex")
        .prop_filter("must not look like a shell keyword", |s| !starts_with_keyword(s))
}

fn arb_env_name() -> impl Strategy<Value = String> {
    prop::string::string_regex("[A-Z_][A-Z0-9_]{0,5}")
        .expect("valid regex")
}

fn arb_word(depth: u32) -> BoxedStrategy<Word> {
    prop::collection::vec(arb_word_part(depth), 1..3)
        .prop_map(Word)
        .boxed()
}

fn arb_word_part(depth: u32) -> BoxedStrategy<WordPart> {
    let leaf = prop_oneof![
        arb_shell_word().prop_map(WordPart::Lit),
        arb_shell_word().prop_map(WordPart::SQuote),
        prop::char::range('a', 'z').prop_map(WordPart::Escape),
    ];

    if depth == 0 {
        return leaf.boxed();
    }

    prop_oneof![
        3 => arb_shell_word().prop_map(WordPart::Lit),
        2 => arb_shell_word().prop_map(WordPart::SQuote),
        1 => arb_dq_word(depth - 1).prop_map(WordPart::DQuote),
        1 => arb_script(depth - 1).prop_map(WordPart::CmdSub),
    ]
    .boxed()
}

fn arb_dq_word(depth: u32) -> BoxedStrategy<Word> {
    prop::collection::vec(arb_dq_part(depth), 1..3)
        .prop_map(Word)
        .boxed()
}

fn arb_dq_part(depth: u32) -> BoxedStrategy<WordPart> {
    let leaf = prop_oneof![
        arb_shell_word().prop_map(WordPart::Lit),
        prop_oneof![Just('"'), Just('\\'), Just('$'), Just('`')].prop_map(WordPart::Escape),
    ];

    if depth == 0 {
        return leaf.boxed();
    }

    prop_oneof![
        3 => arb_shell_word().prop_map(WordPart::Lit),
        1 => arb_script(depth - 1).prop_map(WordPart::CmdSub),
    ]
    .boxed()
}

fn arb_heredoc_delimiter() -> impl Strategy<Value = String> {
    prop::string::string_regex("[A-Z_][A-Z0-9_]{0,5}").expect("valid regex")
}

fn arb_redir() -> BoxedStrategy<Redir> {
    prop_oneof![
        (0..3u32, arb_word(0), any::<bool>()).prop_map(|(fd, target, append)| {
            Redir::Write { fd, target, append }
        }),
        (0..3u32, arb_word(0)).prop_map(|(fd, target)| Redir::Read { fd, target }),
        arb_word(0).prop_map(Redir::HereStr),
        (arb_heredoc_delimiter(), any::<bool>()).prop_map(|(delimiter, strip_tabs)| {
            Redir::HereDoc { delimiter, strip_tabs }
        }),
        (0..3u32, prop_oneof!["0", "1", "2", "-"].prop_map(String::from))
            .prop_map(|(src, dst)| Redir::DupFd { src, dst }),
    ]
    .boxed()
}

fn arb_simple_cmd(depth: u32) -> BoxedStrategy<SimpleCmd> {
    let word_strat = arb_word(depth);
    (
        prop::collection::vec((arb_env_name(), arb_word(0)), 0..2),
        prop::collection::vec(word_strat, 1..4),
        prop::collection::vec(arb_redir(), 0..2),
    )
        .prop_map(|(env, words, redirs)| SimpleCmd { env, words, redirs })
        .boxed()
}

fn arb_cmd(depth: u32) -> BoxedStrategy<Cmd> {
    if depth == 0 {
        return arb_simple_cmd(0).prop_map(Cmd::Simple).boxed();
    }

    prop_oneof![
        4 => arb_simple_cmd(depth).prop_map(Cmd::Simple),
        1 => arb_script(depth - 1).prop_map(|body| Cmd::Subshell { body, redirs: vec![] }),
        1 => (
            arb_env_name(),
            prop::collection::vec(arb_word(0), 1..3),
            arb_script(depth - 1),
        ).prop_map(|(var, items, body)| Cmd::For { var, items, body, redirs: vec![] }),
        1 => (arb_script(depth - 1), arb_script(depth - 1))
            .prop_map(|(cond, body)| Cmd::While { cond, body, redirs: vec![] }),
        1 => (
            prop::collection::vec(
                (arb_script(depth - 1), arb_script(depth - 1))
                    .prop_map(|(cond, body)| Branch { cond, body }),
                1..3,
            ),
            prop::option::of(arb_script(depth - 1)),
        ).prop_map(|(branches, else_body)| Cmd::If { branches, else_body, redirs: vec![] }),
        1 => prop::collection::vec(arb_word(0), 1..4)
            .prop_map(|words| Cmd::DoubleBracket { words, redirs: vec![] }),
        1 => (arb_func_name(), arb_script(depth - 1))
            .prop_map(|(name, body)| Cmd::FunctionDef { name, body }),
    ]
    .boxed()
}

/// A function name: an identifier that isn't a shell keyword (so `for(){…}` isn't generated, keeping
/// the roundtrip unambiguous with the keyword-compound parsers).
fn arb_func_name() -> impl Strategy<Value = String> {
    prop::string::string_regex("[a-z][a-z0-9_]{0,7}")
        .expect("valid regex")
        .prop_filter("not a shell keyword", |s| !starts_with_keyword(s))
}

/// A path value spanning safe (in-workspace) and hot (system/home/parent/credential) loci, with no
/// `$` or shell-special chars so it drops cleanly into both the `VAR=…` and the expanded command.
fn arb_locus_value() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("./data".to_string()),
        Just("sub/dir".to_string()),
        Just("/work/inside".to_string()),
        Just("/etc".to_string()),
        Just("/var/log".to_string()),
        Just("../outside".to_string()),
        Just("~/.ssh".to_string()),
        Just("/Users/x/.aws".to_string()),
        prop::string::string_regex("[a-z][a-z0-9_./-]{0,10}").expect("valid regex"),
    ]
}

fn arb_pipeline(depth: u32) -> BoxedStrategy<Pipeline> {
    (any::<bool>(), prop::collection::vec(arb_cmd(depth), 1..3))
        .prop_map(|(bang, commands)| Pipeline { bang, commands })
        .boxed()
}

fn arb_list_op() -> impl Strategy<Value = ListOp> {
    prop_oneof![
        Just(ListOp::And),
        Just(ListOp::Or),
        Just(ListOp::Semi),
        Just(ListOp::Amp),
    ]
}

fn arb_stmt(depth: u32) -> BoxedStrategy<Stmt> {
    (arb_pipeline(depth), prop::option::of(arb_list_op()))
        .prop_map(|(pipeline, op)| Stmt { pipeline, op })
        .boxed()
}

fn arb_script(depth: u32) -> BoxedStrategy<Script> {
    prop::collection::vec(arb_stmt(depth), 1..3)
        .prop_map(|mut stmts| {
            let len = stmts.len();
            for (i, stmt) in stmts.iter_mut().enumerate() {
                if i == len - 1 {
                    stmt.op = None;
                } else if stmt.op.is_none() {
                    stmt.op = Some(ListOp::Semi);
                }
            }
            Script(stmts)
        })
        .boxed()
}

fn arb_dev_null_word() -> impl Strategy<Value = Word> {
    Just(Word(vec![WordPart::Lit("/dev/null".to_string())]))
}

fn arb_safe_redir() -> BoxedStrategy<Redir> {
    prop_oneof![
        (0..3u32, arb_dev_null_word(), any::<bool>()).prop_map(|(fd, target, append)| {
            Redir::Write { fd, target, append }
        }),
        (0..3u32, arb_dev_null_word()).prop_map(|(fd, target)| Redir::Read { fd, target }),
        arb_word(0).prop_map(Redir::HereStr),
        (arb_heredoc_delimiter(), any::<bool>()).prop_map(|(delimiter, strip_tabs)| {
            Redir::HereDoc { delimiter, strip_tabs }
        }),
        (0..3u32, prop_oneof!["0", "1", "2"].prop_map(String::from))
            .prop_map(|(src, dst)| Redir::DupFd { src, dst }),
    ]
    .boxed()
}

fn unsafe_rm() -> Cmd {
    Cmd::Simple(SimpleCmd {
        env: vec![],
        words: vec![
            Word(vec![WordPart::Lit("rm".into())]),
            Word(vec![WordPart::Lit("-rf".into())]),
            Word(vec![WordPart::Lit("/".into())]),
        ],
        redirs: vec![],
    })
}

fn unsafe_script() -> Script {
    Script(vec![Stmt {
        pipeline: Pipeline {
            bang: false,
            commands: vec![unsafe_rm()],
        },
        op: None,
    }])
}

fn inject_unsafe_into_pipeline(pipeline: &Pipeline, pos: usize) -> Pipeline {
    let mut commands = pipeline.commands.clone();
    let idx = pos % (commands.len() + 1);
    commands.insert(idx, unsafe_rm());
    Pipeline {
        bang: pipeline.bang,
        commands,
    }
}

fn inject_unsafe_into_script(script: &Script, pos: usize) -> Script {
    if script.0.is_empty() {
        return Script(vec![Stmt {
            pipeline: Pipeline {
                bang: false,
                commands: vec![unsafe_rm()],
            },
            op: None,
        }]);
    }
    let stmt_idx = pos % script.0.len();
    let mut stmts = script.0.clone();
    stmts[stmt_idx].pipeline = inject_unsafe_into_pipeline(&stmts[stmt_idx].pipeline, pos / 2);
    Script(stmts)
}

proptest! {
    #[test]
    fn roundtrip(script in arb_script(2)) {
        let normalized = script.normalize();
        let rendered = normalized.to_string();
        let parsed = parse(&rendered);
        prop_assert!(
            parsed.is_some(),
            "failed to parse rendered script: {rendered}"
        );
        prop_assert_eq!(parsed.unwrap(), normalized);
    }

    #[test]
    fn eval_determinism(word in arb_word(2)) {
        let a = word.eval();
        let b = word.eval();
        prop_assert_eq!(a, b);
    }

    /// A function DEFINITION has no effect, so it is Inert/allowed for ANY body — even one that, run,
    /// would be denied (`f(){ rm -rf /; }`). Safety comes from classifying the CALL, not the def.
    #[test]
    fn function_definition_is_always_inert(script in arb_script(1), pos in 0..8usize) {
        let unsafe_body = inject_unsafe_into_script(&script, pos);
        let def = Cmd::FunctionDef { name: "f".into(), body: unsafe_body };
        prop_assert_eq!(
            check::cmd_verdict(&def),
            crate::verdict::Verdict::Allowed(crate::verdict::SafetyLevel::Inert),
            "a function definition must classify Inert regardless of its body"
        );
    }

    /// FAITHFULNESS (the security crux): binding a CERTAIN literal is EXACTLY manual substitution —
    /// `VAR=<lit>; cmd $VAR/<suf>` must classify identically to `cmd <lit>/<suf>`. If these ever
    /// disagree, resolution has under- or over-approved relative to what bash actually runs. Covers
    /// safe (`./data`) and hot (`/etc`, `~/.ssh`, `../out`) values, and read (`cat`) and write (`tee`).
    #[test]
    fn var_substitution_matches_manual_expansion(
        value in arb_locus_value(),
        suffix in "[a-z][a-z0-9_./]{0,10}",
        // Span the classifier paths: engine readers/writers (cat/tee/grep), a legacy pathgate reader
        // (od) and in-place writer (sed -i), and a coreutil (head) — substitution must be consistent
        // across ALL of them, or a bound var to a hot path could slip a layer that doesn't expand.
        reader in prop_oneof![
            Just("cat"), Just("tee"), Just("grep x"), Just("od"), Just("head -1"),
            Just("sed -i s/a/b/"),
        ],
    ) {
        let _ctx = crate::pathctx::enter(crate::pathctx::PathCtx {
            cwd: Some("/work".into()), root: Some("/work".into()),
        });
        let with_var = format!("VAR={value}; {reader} $VAR/{suffix}");
        let expanded = format!("{reader} {value}/{suffix}");
        prop_assert_eq!(
            crate::is_safe_command(&with_var),
            crate::is_safe_command(&expanded),
            "resolution disagreed with expansion:\n  `{}`\n  `{}`", with_var, expanded
        );
    }

    /// FAITHFULNESS for function args: `f(){ cmd "$1"; }; f <lit>` must classify identically to
    /// `cmd <lit>` — the call resolves to its body with $1 bound to the literal arg.
    #[test]
    fn function_arg_binding_matches_direct_call(
        value in arb_locus_value(),
        reader in prop_oneof![Just("cat"), Just("tee")],
    ) {
        let _ctx = crate::pathctx::enter(crate::pathctx::PathCtx {
            cwd: Some("/work".into()), root: Some("/work".into()),
        });
        let via_fn = format!("f(){{ {reader} \"$1\"; }}; f {value}");
        let direct = format!("{reader} {value}");
        prop_assert_eq!(
            crate::is_safe_command(&via_fn),
            crate::is_safe_command(&direct),
            "arg binding disagreed:\n  `{}`\n  `{}`", via_fn, direct
        );
    }

    #[test]
    fn redirect_safety(redirs in prop::collection::vec(arb_redir(), 1..4)) {
        let result = check::check_redirects(&redirs);
        let expected = redirs.iter().all(|r| match r {
            Redir::Write { target, .. } => target.eval() == "/dev/null",
            _ => true,
        });
        prop_assert_eq!(result, expected);
    }

    #[test]
    fn safe_redirects_always_pass(redirs in prop::collection::vec(arb_safe_redir(), 1..4)) {
        prop_assert!(check::check_redirects(&redirs));
    }

    #[test]
    fn unsafe_sub_detected_in_word(safe_word in arb_shell_word()) {
        let word_with_sub = Word(vec![
            WordPart::Lit(safe_word),
            WordPart::CmdSub(unsafe_script()),
        ]);
        prop_assert!(!check::word_subs_safe(&word_with_sub));
    }

    #[test]
    fn unsafe_sub_in_dquote_detected(safe_word in arb_shell_word()) {
        let word_nested = Word(vec![
            WordPart::DQuote(Word(vec![
                WordPart::Lit(safe_word),
                WordPart::CmdSub(unsafe_script()),
            ])),
        ]);
        prop_assert!(!check::word_subs_safe(&word_nested));
    }

    #[test]
    fn safe_word_no_subs(parts in prop::collection::vec(
        prop_oneof![
            arb_shell_word().prop_map(WordPart::Lit),
            arb_shell_word().prop_map(WordPart::SQuote),
            prop::char::range('a', 'z').prop_map(WordPart::Escape),
        ],
        1..5,
    )) {
        let word = Word(parts);
        prop_assert!(check::word_subs_safe(&word));
    }

    #[test]
    fn unsafe_injected_into_pipeline(
        script in arb_script(1),
        pos in 0..20usize,
    ) {
        let injected = inject_unsafe_into_script(&script, pos);
        prop_assert!(
            !check::is_safe_script(&injected),
            "unsafe command not detected after injection into: {}",
            injected.to_string()
        );
    }

    #[test]
    fn unsafe_in_subshell_detected(script in arb_script(0)) {
        let with_unsafe = Script(vec![
            Stmt {
                pipeline: Pipeline {
                    bang: false,
                    commands: vec![Cmd::Subshell { body: inject_unsafe_into_script(&script, 0), redirs: vec![] }],
                },
                op: None,
            },
        ]);
        prop_assert!(!check::is_safe_script(&with_unsafe));
    }

    #[test]
    fn unsafe_in_for_body_detected(
        var in arb_env_name(),
        items in prop::collection::vec(arb_word(0), 1..3),
    ) {
        let cmd = Cmd::For {
            var,
            items,
            body: unsafe_script(),
            redirs: vec![],
        };
        prop_assert!(!check::is_safe_cmd(&cmd));
    }

    #[test]
    fn unsafe_in_while_body_detected(safe_cond in arb_script(0)) {
        let cmd = Cmd::While {
            cond: safe_cond,
            body: unsafe_script(),
            redirs: vec![],
        };
        prop_assert!(!check::is_safe_cmd(&cmd));
    }

    #[test]
    fn unsafe_in_while_cond_detected(safe_body in arb_script(0)) {
        let cmd = Cmd::While {
            cond: unsafe_script(),
            body: safe_body,
            redirs: vec![],
        };
        prop_assert!(!check::is_safe_cmd(&cmd));
    }

    #[test]
    fn unsafe_in_if_body_detected(safe_cond in arb_script(0)) {
        let cmd = Cmd::If {
            branches: vec![Branch {
                cond: safe_cond,
                body: unsafe_script(),
            }],
            else_body: None,
            redirs: vec![],
        };
        prop_assert!(!check::is_safe_cmd(&cmd));
    }

    #[test]
    fn unsafe_in_if_cond_detected(safe_body in arb_script(0)) {
        let cmd = Cmd::If {
            branches: vec![Branch {
                cond: unsafe_script(),
                body: safe_body,
            }],
            else_body: None,
            redirs: vec![],
        };
        prop_assert!(!check::is_safe_cmd(&cmd));
    }

    #[test]
    fn unsafe_in_else_detected(safe_cond in arb_script(0), safe_body in arb_script(0)) {
        let cmd = Cmd::If {
            branches: vec![Branch {
                cond: safe_cond,
                body: safe_body,
            }],
            else_body: Some(unsafe_script()),
            redirs: vec![],
        };
        prop_assert!(!check::is_safe_cmd(&cmd));
    }

    #[test]
    fn unsafe_in_for_items_sub_detected(
        var in arb_env_name(),
        safe_body in arb_script(0),
    ) {
        let cmd = Cmd::For {
            var,
            items: vec![Word(vec![WordPart::CmdSub(unsafe_script())])],
            body: safe_body,
            redirs: vec![],
        };
        prop_assert!(!check::is_safe_cmd(&cmd));
    }

    #[test]
    fn file_redirect_promotes_to_safewrite(
        cmd_word in arb_shell_word(),
        // Safe write targets only: an in-tree relative data path. Absolute,
        // home, parent-escaping, and auto-executed (.git/.envrc) targets are
        // covered by the unsafe-target test below.
        target in arb_shell_word().prop_filter("safe in-tree write target", |s| {
            !s.is_empty()
                && !s.starts_with('/')
                && !s.starts_with('~')
                && !s.contains('$')
                && !s.split('/').any(|seg| seg == ".." || seg == ".git" || seg == ".envrc")
        }),
        fd in 0..3u32,
        append in any::<bool>(),
    ) {
        let cmd = SimpleCmd {
            env: vec![],
            words: vec![Word(vec![WordPart::Lit(cmd_word)])],
            redirs: vec![Redir::Write {
                fd,
                target: Word(vec![WordPart::Lit(target)]),
                append,
            }],
        };
        prop_assert!(!check::check_redirects(&cmd.redirs));
        prop_assert_eq!(
            check::redirect_verdict(&cmd.redirs),
            crate::verdict::Verdict::Allowed(crate::verdict::SafetyLevel::SafeWrite),
        );
    }

    #[test]
    fn redirect_to_auto_executed_target_is_denied(
        // Targets another tool auto-executes/trusts, or that escape the tree.
        target in prop_oneof![
            Just(".git/hooks/pre-commit".to_string()),
            Just(".envrc".to_string()),
            arb_env_name().prop_map(|n| format!("sub/{n}/.git/config")),
            arb_env_name().prop_map(|n| format!("/etc/{n}")),
            arb_env_name().prop_map(|n| format!("../{n}")),
            arb_env_name().prop_map(|n| format!("$HOME/.ssh/{n}")),
        ],
        append in any::<bool>(),
    ) {
        let redirs = vec![Redir::Write {
            fd: 1,
            target: Word(vec![WordPart::Lit(target)]),
            append,
        }];
        prop_assert_eq!(check::redirect_verdict(&redirs), crate::verdict::Verdict::Denied);
    }

    #[test]
    fn parse_never_panics(input in "[ -~]{0,200}") {
        let _ = parse(&input);
    }

    #[test]
    fn heredoc_always_safe(
        cmd_word in arb_shell_word(),
        delimiter in arb_heredoc_delimiter(),
        strip_tabs in any::<bool>(),
    ) {
        let redir = Redir::HereDoc { delimiter, strip_tabs };
        prop_assert!(check::check_redirects(&[redir]));

        let cmd = SimpleCmd {
            env: vec![],
            words: vec![Word(vec![WordPart::Lit(cmd_word)])],
            redirs: vec![Redir::HereDoc {
                delimiter: "EOF".to_string(),
                strip_tabs: false,
            }],
        };
        prop_assert!(check::check_redirects(&cmd.redirs));
    }

    #[test]
    fn unicode_prefix_never_matches_allowlist(
        prefix in "[\\u{0080}-\\u{FFFF}]{1,3}",
        cmd in "(git|cat|ls|grep)"
    ) {
        let mangled = format!("{prefix}{cmd} --version");
        prop_assert!(!crate::is_safe_command(&mangled),
            "Unicode-prefixed command was approved: {}", mangled);
    }

    #[test]
    fn unicode_suffix_never_matches_allowlist(
        cmd in "(git|cat|ls|grep)",
        suffix in "[\\u{0080}-\\u{FFFF}]{1,3}"
    ) {
        let mangled = format!("{cmd}{suffix} --version");
        prop_assert!(!crate::is_safe_command(&mangled),
            "Unicode-suffixed command was approved: {}", mangled);
    }
}

// ── Function/variable resolution: concrete cases ────────────────────────────────────────────────
#[cfg(test)]
mod resolution {
    use super::super::*;

    fn workspace() -> crate::pathctx::Guard {
        crate::pathctx::enter(crate::pathctx::PathCtx {
            cwd: Some("/work".into()),
            root: Some("/work".into()),
        })
    }
    fn allowed(cmd: &str) -> bool {
        crate::is_safe_command(cmd)
    }

    #[test]
    fn a_definition_shadows_a_builtin_so_the_call_is_the_body() {
        // The security case Phase 1 alone opened: a function redefines a safe builtin. The CALL must
        // classify the BODY, not the real command.
        assert!(!allowed("ls(){ rm -rf /; }; ls"), "rebound ls to rm must DENY");
        assert!(allowed("ls(){ echo hi; }; ls"), "rebound ls to echo must ALLOW");
        // …and only AFTER the definition — a call before it is still the real command.
        assert!(allowed("ls; ls(){ rm -rf /; }"), "ls before its definition is the real ls");
    }

    #[test]
    fn call_resolves_body_and_binds_args() {
        let _w = workspace();
        assert!(allowed("greet(){ echo hi; }; greet"), "no-arg safe body");
        assert!(allowed("greet(){ cat \"$1\"; }; greet ./notes.txt"), "$1 = in-workspace read");
        assert!(!allowed("greet(){ cat \"$1\"; }; greet /etc/shadow"), "$1 = system read denies");
        assert!(!allowed("greet(){ rm -rf \"$1\"; }; greet /"), "$1 into rm -rf / denies");
    }

    #[test]
    fn var_assignment_substitutes_by_locus() {
        let _w = workspace();
        assert!(allowed("GEM=./data; cat $GEM/notes.txt"), "in-workspace resolves + allows");
        assert!(!allowed("GEM=/etc; cat $GEM/hosts"), "system path resolves + denies");
        assert!(allowed("A=./d; B=$A/sub; cat $B/x"), "chained certain assignments resolve");
    }

    #[test]
    fn uncertain_and_unbound_fail_closed() {
        let _w = workspace();
        assert!(!allowed("GEM=$(evilcmd); cat $GEM/x"), "cmd-sub value is uncertain → deny");
        assert!(!allowed("cat $UNDEFINED/x"), "unbound var → untouched → deny");
        // reassignment to uncertain must not leave the earlier certain value live.
        assert!(!allowed("GEM=./safe; GEM=$(x); cat $GEM/f"), "stale certain value must not survive");
        // last CERTAIN assignment wins — here to a system path.
        assert!(!allowed("GEM=./safe; GEM=/etc; cat $GEM/f"), "last assignment (/etc) wins → deny");
    }

    #[test]
    fn a_shadowed_builtin_never_falls_through_when_resolution_is_blocked() {
        // A defined function shadows the builtin UNCONDITIONALLY. If resolution is blocked — by the
        // classification budget (exhaust it with harmless calls first) or by recursion — the call
        // must FAIL CLOSED, never run the real command. Else the rebound name is a bypass.
        let exhaust = "g;".repeat(600);
        let budget = format!("g(){{ echo hi; }}; ls(){{ rm -rf /; }}; {exhaust} ls");
        assert!(!allowed(&budget), "budget-blocked rebind must not fall through to real ls");
        assert!(!allowed("ls(){ ls; }; ls"), "recursion-blocked rebind must not fall through to real ls");
    }

    #[test]
    fn a_function_shadows_the_eval_builtin() {
        // A user function named `eval` shadows the eval builtin (`eval(){ rm; }; eval "echo hi"` runs
        // the function). Resolution must run BEFORE the eval special-case, or the rebind is a bypass.
        assert!(!allowed(r#"eval(){ rm -rf /; }; eval "echo hi""#), "rebound eval to rm must DENY");
        assert!(allowed("eval(){ echo hi; }; eval x"), "rebound eval to echo must ALLOW");
    }

    #[test]
    fn passthrough_filters_carry_the_pipe_locus() {
        let _w = workspace();
        // A line-preserving filter (head/tail/sort/uniq/cat/tee) between a producer and an injecting
        // consumer must carry the producer's item locus THROUGH — else xargs worst-cases and denies.
        assert!(allowed("find /work -type f | head -1 | xargs cat"), "head passes find's locus");
        assert!(allowed("find ./src | sort | uniq | xargs cat"), "sort|uniq chain passes through");
        // …but a TRANSFORMING filter must NOT (it can rewrite a line to any path — a bypass).
        assert!(!allowed("find ./src | sed s@.*@/etc/shadow@ | xargs cat"), "sed rewrite must not pass through");
        assert!(!allowed("find ./src | head -1 realfile | xargs cat"), "head reading a FILE isn't a stdin passthrough");
        assert!(!allowed("find /etc | head -1 | xargs cat"), "system source propagates → deny");
        // `sort --files0-from=F` emits the CONTENTS of the files listed in F — file-derived output,
        // not the piped stream — so it must NOT launder a safe upstream locus onto a destroyer. The
        // `=`-glued form is a lone `-`-prefixed token that would otherwise masquerade as passthrough.
        assert!(!allowed("echo x | sort --files0-from=./list.txt | xargs rm"), "=-glued files0-from is not a passthrough");
        assert!(!allowed("echo x | sort --files0-from ./list.txt | xargs rm"), "space files0-from is not a passthrough");
        assert!(!allowed("echo x | sort --files0-from=./list.txt | while read f; do rm \"$f\"; done"), "files0-from into while-read must not launder");
    }

    #[test]
    fn while_read_binds_the_loop_var_to_the_pipe_locus() {
        let _w = workspace();
        assert!(allowed("find ./src -type f | while read f; do cat \"$f\"; done"), "read var reads worktree");
        assert!(allowed("find /work | head | while IFS= read -r f; do cat \"$f\"; done"), "passthrough + read compose");
        assert!(!allowed("find /etc -type f | while read f; do cat \"$f\"; done"), "system source → deny");
        assert!(!allowed("find /etc | while read f; do rm \"$f\"; done"), "system write via read var → deny");
        assert!(!allowed("while read f; do cat \"$f\"; done"), "no pipe source → unbound → fail closed");
    }

    #[test]
    fn recursion_and_depth_terminate_and_deny() {
        // Direct and mutual recursion must terminate (bounded) and deny — no hang, no stack blow-up.
        assert!(!allowed("f(){ f; }; f"), "direct recursion");
        assert!(!allowed("f(){ g; }; g(){ f; }; f"), "mutual recursion");
        let deep = (0..80).map(|i| format!("f{i}(){{ f{}; }}; ", i + 1)).collect::<String>() + "f0";
        assert!(!allowed(&deep), "deep non-recursive chain past the depth cap denies, not overflows");
    }
}

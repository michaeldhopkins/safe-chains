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
        1 => arb_script(depth - 1).prop_map(Cmd::Subshell),
        1 => (
            arb_env_name(),
            prop::collection::vec(arb_word(0), 1..3),
            arb_script(depth - 1),
        ).prop_map(|(var, items, body)| Cmd::For { var, items, body }),
        1 => (arb_script(depth - 1), arb_script(depth - 1))
            .prop_map(|(cond, body)| Cmd::While { cond, body }),
        1 => (
            prop::collection::vec(
                (arb_script(depth - 1), arb_script(depth - 1))
                    .prop_map(|(cond, body)| Branch { cond, body }),
                1..3,
            ),
            prop::option::of(arb_script(depth - 1)),
        ).prop_map(|(branches, else_body)| Cmd::If { branches, else_body }),
    ]
    .boxed()
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

    #[test]
    fn redirect_safety(redirs in prop::collection::vec(arb_redir(), 1..4)) {
        let result = check::check_redirects(&redirs);
        let expected = redirs.iter().all(|r| match r {
            Redir::Write { target, .. } | Redir::Read { target, .. } => {
                target.eval() == "/dev/null"
            }
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
                    commands: vec![Cmd::Subshell(inject_unsafe_into_script(&script, 0))],
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
        };
        prop_assert!(!check::is_safe_cmd(&cmd));
    }

    #[test]
    fn unsafe_in_while_body_detected(safe_cond in arb_script(0)) {
        let cmd = Cmd::While {
            cond: safe_cond,
            body: unsafe_script(),
        };
        prop_assert!(!check::is_safe_cmd(&cmd));
    }

    #[test]
    fn unsafe_in_while_cond_detected(safe_body in arb_script(0)) {
        let cmd = Cmd::While {
            cond: unsafe_script(),
            body: safe_body,
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
        };
        prop_assert!(!check::is_safe_cmd(&cmd));
    }

    #[test]
    fn file_redirect_always_denied(
        cmd_word in arb_shell_word(),
        target in arb_shell_word().prop_filter("not /dev/null", |s| s != "/dev/null"),
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

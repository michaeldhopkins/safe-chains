use super::*;
use proptest::prelude::*;

fn arb_shell_word() -> impl Strategy<Value = String> {
    prop::string::string_regex("[a-zA-Z0-9_./-]+")
        .expect("valid regex")
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

fn arb_redir() -> BoxedStrategy<Redir> {
    prop_oneof![
        (0..3u32, arb_word(0), any::<bool>()).prop_map(|(fd, target, append)| {
            Redir::Write { fd, target, append }
        }),
        (0..3u32, arb_word(0)).prop_map(|(fd, target)| Redir::Read { fd, target }),
        arb_word(0).prop_map(Redir::HereStr),
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
        (0..3u32, prop_oneof!["0", "1", "2"].prop_map(String::from))
            .prop_map(|(src, dst)| Redir::DupFd { src, dst }),
    ]
    .boxed()
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
        let unsafe_script = Script(vec![Stmt {
            pipeline: Pipeline {
                bang: false,
                commands: vec![Cmd::Simple(SimpleCmd {
                    env: vec![],
                    words: vec![
                        Word(vec![WordPart::Lit("rm".to_string())]),
                        Word(vec![WordPart::Lit("-rf".to_string())]),
                        Word(vec![WordPart::Lit("/".to_string())]),
                    ],
                    redirs: vec![],
                })],
            },
            op: None,
        }]);

        let word_with_sub = Word(vec![
            WordPart::Lit(safe_word),
            WordPart::CmdSub(unsafe_script),
        ]);
        prop_assert!(!check::word_subs_safe(&word_with_sub));
    }

    #[test]
    fn unsafe_sub_in_dquote_detected(safe_word in arb_shell_word()) {
        let unsafe_script = Script(vec![Stmt {
            pipeline: Pipeline {
                bang: false,
                commands: vec![Cmd::Simple(SimpleCmd {
                    env: vec![],
                    words: vec![
                        Word(vec![WordPart::Lit("rm".to_string())]),
                        Word(vec![WordPart::Lit("-rf".to_string())]),
                        Word(vec![WordPart::Lit("/".to_string())]),
                    ],
                    redirs: vec![],
                })],
            },
            op: None,
        }]);

        let word_nested = Word(vec![
            WordPart::DQuote(Word(vec![
                WordPart::Lit(safe_word),
                WordPart::CmdSub(unsafe_script),
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
}

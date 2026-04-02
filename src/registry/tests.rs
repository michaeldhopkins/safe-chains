use super::*;
    use super::types::DispatchKind;
    use crate::parse::Token;
    use crate::policy::FlagStyle;
    use crate::verdict::{SafetyLevel, Verdict};

    fn toks(words: &[&str]) -> Vec<Token> {
        words.iter().map(|s| Token::from_test(s)).collect()
    }

    fn load_one(toml_str: &str) -> CommandSpec {
        let mut specs = load_toml(toml_str);
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
    fn flat_positional_style() {
        let spec = load_one(r#"
            [[command]]
            name = "echo"
            bare = true
            positional_style = true
            standalone = ["-n", "-e"]
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["echo", "--unknown", "hello"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
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
        "#);
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
        "#);
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
    fn positional_style_unknown_eq() {
        let spec = load_one(r#"
            [[command]]
            name = "echo"
            bare = true
            positional_style = true
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["echo", "--foo=bar"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
    }

    #[test]
    fn positional_style_with_max() {
        let spec = load_one(r#"
            [[command]]
            name = "echo"
            bare = true
            positional_style = true
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

    // ---------------------------------------------------------------
    // Integration: TOML registry rejects unknown flags
    // ---------------------------------------------------------------

    #[test]
    fn toml_registry_rejects_unknown_flags() {
        let mut failures = Vec::new();
        for (name, spec) in TOML_REGISTRY.iter() {
            match &spec.kind {
                DispatchKind::Policy { policy, .. } | DispatchKind::RequireAny { policy, .. } => {
                    if policy.flag_style == FlagStyle::Positional {
                        continue;
                    }
                }
                _ => {}
            }
            let test = format!("{name} --xyzzy-unknown-42");
            if crate::is_safe_command(&test) {
                failures.push(format!("{name}: accepted unknown flag"));
            }
        }
        assert!(failures.is_empty(), "TOML commands accepted unknown flags:\n{}", failures.join("\n"));
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

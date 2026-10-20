//! `safe-chains` judging its own invocations.
//!
//! Most of the CLI only reads, and the fallback grammar in `commands/tools/safe-chains.toml`
//! describes it. Two subcommands are different: `config` changes the rules safe-chains approves by,
//! and `mcp` serves tools that change them. An agent that could run either unasked could approve
//! itself.
//!
//! So this approves only invocations whose every positional word is known before the shell runs:
//! a plain word, or a quoted command string. Anything the shell could turn into `config` or `mcp`
//! at run time is refused: an ANSI-C or locale quote (`$'mcp'`), a substitution (`$(printf mcp)`),
//! a glob (`m*`), or an operand that `xargs` or `find -exec` appends. Review found every one of
//! those approving `safe-chains mcp` while a literal-word check looked fine. The positional that
//! is left then picks the subcommand exactly as clap will: `config` goes to its TOML subs (only
//! `list` is there), `mcp` to nothing, `hook` must name a target, and anything else is a command
//! to check.
use crate::parse::Token;
use crate::registry;
use crate::verdict::Verdict;

pub fn check_safe_chains(tokens: &[Token]) -> Verdict {
    let Some((standalone, valued)) = registry::fallback_flag_sets("safe-chains") else {
        return Verdict::Denied;
    };
    let mut positionals: Vec<&str> = Vec::new();
    let mut first_positional_at = None;
    let mut i = 1;
    let mut after_separator = false;
    while i < tokens.len() {
        let word = tokens[i].as_str();
        if !after_separator && word == "--" {
            after_separator = true;
        } else if !after_separator && valued.iter().any(|v| v == word) {
            i += 1;
        } else if after_separator || !word.starts_with('-') || word == "-" {
            first_positional_at.get_or_insert(i);
            positionals.push(word);
        } else if !standalone.iter().any(|s| s == word) && !valued.iter().any(|v| word.starts_with(&format!("{v}="))) {
            return Verdict::Denied;
        }
        i += 1;
    }
    if !positionals.iter().all(|p| known_before_it_runs(p)) {
        return Verdict::Denied;
    }
    match positionals.as_slice() {
        ["config", ..] if first_positional_at == Some(1) => registry::try_sub_dispatch("safe-chains", tokens).unwrap_or(Verdict::Denied),
        ["hook", tool] if crate::targets::find(tool).is_some() => fallback(tokens),
        ["help"] | ["help", "hook"] => fallback(tokens),
        [] => fallback(tokens),
        [command] if !SUBCOMMANDS.contains(command) => fallback(tokens),
        _ => Verdict::Denied,
    }
}

pub(crate) fn cargo_run_reaches_a_rule_change(tokens: &[Token]) -> bool {
    let Some(run_at) = tokens.iter().skip(1).position(|t| t.as_str() == "run").map(|p| p + 1) else {
        return false;
    };
    let valued = registry::sub_valued_flags("cargo", "run").unwrap_or_default();
    let mut k = run_at + 1;
    while k < tokens.len() {
        let word = tokens[k].as_str();
        if word == "--" {
            k += 1;
            break;
        }
        if !word.starts_with('-') || word == "-" {
            break;
        }
        k += if valued.iter().any(|v| v == word) { 2 } else { 1 };
    }
    let Some(arguments) = tokens.get(k..) else {
        return false;
    };
    if !arguments
        .iter()
        .any(|t| matches!(t.as_str(), "config" | "mcp") || could_change_at_run_time(t.as_str()))
    {
        return false;
    }
    let mut as_safe_chains = vec![Token::from_raw("safe-chains".to_string())];
    as_safe_chains.extend(arguments.iter().cloned());
    !check_safe_chains(&as_safe_chains).is_allowed()
}

fn could_change_at_run_time(word: &str) -> bool {
    word.contains(['$', '`', '*', '?', '[', '{']) || word.contains("__SAFE_CHAINS") || word.contains("sc_item")
}

/// Every word clap reads as a subcommand.
const SUBCOMMANDS: &[&str] = &["config", "help", "hook", "mcp"];

fn fallback(tokens: &[Token]) -> Verdict {
    registry::try_fallback_grammar("safe-chains", tokens).unwrap_or(Verdict::Denied)
}

/// Whether the shell will pass `word` through as written. A word with a space in it was quoted, so
/// no glob or brace expands it. Without one it must be plain text. Expansion markers and the
/// stand-ins for operands that arrive at run time never count as known.
fn known_before_it_runs(word: &str) -> bool {
    if word.contains(['$', '`']) || word.contains("__SAFE_CHAINS") || word.contains("sc_item") {
        return false;
    }
    if word.chars().any(char::is_whitespace) {
        return true;
    }
    word.starts_with(|c: char| c.is_ascii_alphanumeric() || c == '_')
        && word
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '+' | ':' | '@' | '=' | ',' | '-'))
}
#[cfg(test)]
mod tests {
    use crate::verdict::{SafetyLevel, Verdict};

    fn verdict(cmd: &str) -> Verdict {
        crate::command_verdict(cmd)
    }

    #[test]
    fn listing_the_rules_only_reads() {
        assert_eq!(verdict("safe-chains config list"), Verdict::Allowed(SafetyLevel::SafeRead));
    }

    /// The point of the handler: no change to the rules, and no server that makes one, runs unasked.
    #[test]
    fn changing_the_rules_is_never_approved() {
        for cmd in [
            "safe-chains config add mytool",
            "safe-chains config add 'mytool sync'",
            "safe-chains config add --level inert -- mytool",
            "safe-chains config grant ~/notes --read",
            "safe-chains config grant --read -- ~/notes",
            "safe-chains config remove command mytool",
            "safe-chains config remove grant ~/notes",
            "safe-chains config",
            "safe-chains config list --all",
            "safe-chains --log config add mytool",
            "safe-chains --level yolo config grant ~/x --write",
            "safe-chains mcp",
            "safe-chains --log mcp",
            "echo '{}' | safe-chains mcp",
            // Spellings the shell turns into a subcommand at run time (adversarial review).
            "safe-chains $'mcp'",
            "safe-chains $\"mcp\"",
            "safe-chains $'\\x6dcp'",
            "safe-chains $(printf mcp)",
            "safe-chains \"$(echo mcp)\"",
            "safe-chains `echo mcp`",
            "safe-chains m*",
            "safe-chains mc?",
            "safe-chains [m]cp",
            "safe-chains c* [az]*",
            "safe-chains {config,x}",
            "echo 'config add mytool' | xargs safe-chains",
            "echo mcp | xargs safe-chains --log",
            "ls | xargs safe-chains",
            "echo mcp | xargs -I{} safe-chains {}",
            r"find mcp -prune -exec safe-chains {} \;",
            "safe-chains ~",
            "safe-chains hook nosuchtool",
            "safe-chains help config add",
        ] {
            assert!(!verdict(cmd).is_allowed(), "{cmd} must not be approved");
        }
    }

    #[test]
    fn asking_config_for_help_only_reads() {
        assert_eq!(verdict("safe-chains config -h"), Verdict::Allowed(SafetyLevel::Inert));
        assert_eq!(verdict("safe-chains config --help"), Verdict::Allowed(SafetyLevel::Inert));
        for cmd in ["safe-chains config add -h", "safe-chains config -h add", "safe-chains mcp -h", "safe-chains --log config -h"] {
            assert!(!verdict(cmd).is_allowed(), "{cmd} must not be approved");
        }
    }

    #[test]
    fn running_this_project_to_change_the_rules_is_never_approved() {
        for cmd in [
            "cargo run -- config add zz",
            "cargo run config add zz",
            "cargo run -q -- config grant ~/x --write",
            "cargo run --release -- --log config remove command zz",
            "cargo run --bin safe-chains -- mcp",
            "cargo +nightly run -- mcp",
            "cargo -q run -- config add zz",
            "cargo run -- $'mcp'",
            "cargo run -- $(printf mcp)",
            "cargo run -- m*",
            "cargo run -- 'config' add zz",
            "/usr/bin/env cargo run -- config add zz",
            "FOO=1 cargo run -- config add zz",
            "(cargo run -- mcp)",
            "ls && cargo run -- config add zz",
            "cargo run --bin x config add zz",
            "cargo run --features a,b -- mcp",
            "echo mcp | xargs cargo run --",
        ] {
            assert!(!verdict(cmd).is_allowed(), "{cmd} must not be approved");
        }
    }

    #[test]
    fn running_this_project_for_anything_else_is_unaffected() {
        for cmd in [
            "cargo run", "cargo run -- config list", "cargo run -- 'git status'", "cargo run --release -- --explain 'cat foo'",
            "cargo run -q -- hook claude", "cargo run -- ./input.txt", "cargo run --manifest-path ./sub/Cargo.toml",
            "cargo run --bin config", "cargo run -p mcp -- ~/notes", "cargo run -- 'cat *.txt'",
        ] {
            assert!(verdict(cmd).is_allowed(), "{cmd} should be approved");
        }
    }

    #[test]
    fn checking_a_command_that_mentions_config_is_unaffected() {
        assert!(verdict("safe-chains 'git config --get user.name'").is_allowed());
        assert!(verdict("safe-chains --explain 'cat foo'").is_allowed());
    }

    #[test]
    fn a_log_flag_still_raises_the_level() {
        assert_eq!(verdict("safe-chains --log 'cat foo'"), Verdict::Allowed(SafetyLevel::SafeWrite));
        assert_eq!(verdict("safe-chains --log-everything hook claude"), Verdict::Allowed(SafetyLevel::SafeWrite));
        assert_eq!(verdict("safe-chains 'cat foo'"), Verdict::Allowed(SafetyLevel::Inert));
    }
}

#[cfg(test)]
mod properties {
    use proptest::prelude::*;

    proptest! {
        /// Whatever surrounds it, a `config` edit or `mcp` is never approved. `config -h` and
        /// `config --help` only print help, so they are left out here and pinned in
        /// `asking_config_for_help_only_reads`.
        #[test]
        fn no_rule_change_is_ever_approved(
            before in prop::collection::vec(prop::sample::select(vec!["--log", "--explain", "--level yolo", "--cwd /w", "-v"]), 0..3),
            sub in prop::sample::select(vec!["config add", "config remove command", "config remove grant", "config grant", "mcp", "config", "$'mcp'", "m*", "$(printf mcp)", "c* add", "`echo mcp`", "[m]cp"]),
            after in prop::collection::vec("[a-z~/.-]{1,8}", 0..4),
        ) {
            prop_assume!(!(before.is_empty() && sub == "config" && matches!(after.as_slice(), [flag] if flag == "-h" || flag == "--help")));
            let cmd = format!("safe-chains {} {sub} {}", before.join(" "), after.join(" "));
            prop_assert!(!crate::command_verdict(&cmd).is_allowed(), "{}", cmd);
        }

        /// The same holds when `cargo run` starts this project's binary with those words.
        #[test]
        fn no_rule_change_is_approved_through_cargo_run(
            cargo_flags in prop::collection::vec(prop::sample::select(vec!["-q", "--release", "--bin safe-chains", "-p safe-chains", "--offline"]), 0..3),
            separator in prop::sample::select(vec!["--", ""]),
            before in prop::collection::vec(prop::sample::select(vec!["--log", "--explain", "--level yolo", "-v"]), 0..3),
            sub in prop::sample::select(vec!["config add", "config remove command", "config remove grant", "config grant", "mcp", "config", "$'mcp'", "m*", "$(printf mcp)", "c* add", "`echo mcp`", "[m]cp"]),
            after in prop::collection::vec("[a-z~/.-]{1,8}", 0..4),
        ) {
            prop_assume!(!(sub == "config" && after.first().is_some_and(|a| a == "-h")));
            let cmd = format!("cargo run {} {separator} {} {sub} {}", cargo_flags.join(" "), before.join(" "), after.join(" "));
            prop_assert!(!crate::command_verdict(&cmd).is_allowed(), "{}", cmd);
        }
    }
}

#[cfg(test)]
mod subcommands {
    /// The words this handler treats as subcommands are exactly clap's, so a subcommand added to
    /// `cli.rs` cannot be read here as a command to check.
    #[test]
    fn the_handler_knows_every_subcommand_clap_has() {
        use clap::CommandFactory;
        let mut clap: Vec<String> = crate::cli::Cli::command().get_subcommands().map(|s| s.get_name().to_string()).collect();
        clap.push("help".to_string());
        clap.sort();
        let mut ours: Vec<String> = super::SUBCOMMANDS.iter().map(|s| (*s).to_string()).collect();
        ours.sort();
        assert_eq!(ours, clap);
    }
}

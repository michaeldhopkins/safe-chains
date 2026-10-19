use super::*;
use crate::pathctx::{PathCtx, enter};
use proptest::prelude::*;

fn home() -> String {
    std::env::var("HOME").unwrap_or_else(|_| "/home/tester".into())
}

fn kind(command: &str) -> GateKind {
    let project = format!("{}/projects/gateprobe", home());
    let _ctx = enter(PathCtx { cwd: Some(project.clone()), root: Some(project), session_id: None });
    let explanation = crate::explain_with_coverage_at_level(command, None);
    gate_kind(&explanation, &|c| crate::is_safe_command(c), &|_| false)
}

const NOT_YET: &[&str] =
    &["zzgatetool sync --dry-run", "zzgatetool build ./out", "git push --force origin main", "cat ./x | zzgatetool", "echo hi > $(zzgen)"];

const BOUNDARY: &[&str] = &[
    "cat ~/.ssh/id_rsa", "echo x >> ~/.zshrc", "cp a ~/.local/bin/x", "echo x > ~/.claude/settings.json",
    "echo x > ~/.config/safe-chains.toml", "git diff --output=~/.zshrc", "zzgatetool --config=$HOME/.bashrc",
    "safe-chains config add zzgatetool", "safe-chains config grant ~/x", "safe-chains mcp", "claude -p hi", "zzgatetool > /dev/rdisk0",
    "zzgatetool /etc/sudoers",
];

#[test]
fn a_command_nothing_approves_yet_is_not_yet() {
    for command in NOT_YET {
        assert_eq!(kind(command), GateKind::NotYet, "{command}");
    }
}

#[test]
fn a_boundary_stays_a_boundary() {
    for command in BOUNDARY {
        assert_eq!(kind(command), GateKind::Boundary, "{command}");
    }
}

#[test]
fn an_approved_command_is_never_called_not_yet() {
    assert_eq!(kind("ls"), GateKind::Boundary);
}

#[test]
fn words_are_read_through_quotes_redirections_and_flags() {
    assert_eq!(bare_word("--output=~/.zshrc"), "~/.zshrc");
    assert_eq!(bare_word(">>~/.zshrc"), "~/.zshrc");
    assert_eq!(bare_word("'~/.zshrc'"), "~/.zshrc");
    assert_eq!(bare_word("\"$HOME/.bashrc\";"), "$HOME/.bashrc");
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    #[test]
    fn one_boundary_segment_makes_the_whole_chain_a_boundary(
        a in prop::sample::select(NOT_YET),
        b in prop::sample::select(BOUNDARY),
        op in prop::sample::select(&["&&", ";", "||"][..]),
        boundary_first in any::<bool>(),
    ) {
        let chain = if boundary_first { format!("{b} {op} {a}") } else { format!("{a} {op} {b}") };
        prop_assert_eq!(kind(&chain), GateKind::Boundary, "{}", chain);
    }

    #[test]
    fn a_write_to_a_runs_later_place_is_a_boundary_whatever_the_command(
        name in "zz[a-z]{2,8}",
        place in prop::sample::select(&["~/.zshrc", "~/.bashrc", "~/.local/bin/x", "~/.claude/settings.json", "~/.grok/config.toml", "~/Library/LaunchAgents/x.plist"][..]),
        form in 0..3usize,
    ) {
        let command = match form {
            0 => format!("{name} > {place}"),
            1 => format!("{name} --out={place}"),
            _ => format!("{name} {place}"),
        };
        prop_assert_eq!(kind(&command), GateKind::Boundary, "{}", command);
    }
}

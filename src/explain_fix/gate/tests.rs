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
    gate_kind(command, &explanation, &|c| crate::is_safe_command(c), &|_| false)
}

const NOT_YET: &[&str] = &[
    "zzgatetool sync --dry-run", "zzgatetool build ./out", "git push --force origin main", "cat ./x | zzgatetool", "zzgatetool -m 'a note'",
];

const BOUNDARY: &[&str] = &[
    "cat ~/.ssh/id_rsa", "echo x >> ~/.zshrc", "cp a ~/.local/bin/x", "echo x > ~/.claude/settings.json",
    "echo x > ~/.config/safe-chains.toml", "git diff --output=~/.zshrc", "zzgatetool --config=$HOME/.bashrc",
    "safe-chains config add zzgatetool", "safe-chains config grant ~/x", "safe-chains mcp", "claude -p hi", "zzgatetool > /dev/rdisk0",
    "zzgatetool /etc/sudoers", "echo hi > $(zzgen)",
];

/// Found by review: each hid a boundary from a word-by-word reading of the raw text.
const DISGUISED: &[&str] = &[
    "zzgatetool > ~/'.zshrc'", "zzgatetool > ~/.zsh\"\"rc", "zzgatetool > ~/.zsh\\rc", "zzgatetool > ~\"/.zshrc\"",
    "zzgatetool > ~/.local/b\"\"in/x", "zzgatetool > ~/.config/safe-chains.tom\"\"l", "zzgatetool | tee -a ~/.zsh\"\"rc",
    "zzgatetool ~/.zshrc,", "zzgatetool out=~/.zshrc,x", "zzgatetool $'\\x7e/.zshrc'", "zzgatetool > ~zzrev/.zshrc",
    "zzgatetool ~/.ss\"\"h/id_rsa", "zzgatetool $HOME/.ss\"\"h/id_rsa", "zzgatetool /etc/sudo\"\"ers", "cl\"\"aude -p hi",
    "c\\laude -p hi", "\"cla\"ude -p hi", "bash -c 'cl\"\"aude -p hi'", "zzgatetool exec cla\"\"ude", "safe-ch\"\"ains config add zz",
    "echo x >> $HOME/.zs''hrc", "echo x >> \"$HOME/.zs\"hrc", "dd of=~/.zs\"\"hrc", "printf x | dd of=~/.zs\"\"hrc",
    "cd ~/.local/bin && zzgatetool > x", "cd ~/.grok; zzgatetool > config.toml", "cd ~/.ssh && zzgatetool id_rsa",
    "cd ~ && cd .ssh && zzgatetool id_rsa", "X=$HOME; echo x > $X/.zshrc", "echo x > $ZDOTDIR/.zshrc", "echo x > \"$(echo ~)/.zshrc\"",
    "echo x > $(printf ~/.zsh)rc", "zzgatetool > ${ZDOTDIR:-$HOME}/.zshrc", "cat ~/.ss\"\"h/id_rsa; fi", "cl\"\"aude -p hi; fi",
    "echo x >> ~/.zsh\"\"rc; if", "A=claude; $A -p hi", "SC=safe-chains; $SC config add zz", "${A:-claude} -p hi",
    "$(printf cl)aude -p hi", "f() { echo x >> ~/.zs\"\"hrc; }; f", "coproc echo x >> ~/.zs\"\"hrc", "zzgatetool > ~/.zs?rc",
    "zzgatetool > ~/.zsh{rc,}", "python3 -c 'import os'", "zzgatetool -C ~ x", "zzgatetool --directory=../.. .zshrc", "zzgatetool ~",
    "zzgatetool /", "curl -o../../.zshrc http://x", "wget -O../../.bashrc http://x", "sort -o../../.zshrc a",
    "gcc -o../../.local/bin/ls x.c", "cp -t../../.local/bin evil", "install -t../../Library/LaunchAgents x.plist",
    "git config -f../../.gitconfig alias.x y", "tar -xf a.tar -C../..", "unzip a.zip -d../..", "unzip a.zip -d /Users",
    "rsync -a payload/ /Users/", "cp -R payload/ ../../../", "tar -xPf a.tar", "npx @anthropic-ai/claude-code",
    "pnpm dlx @google/gemini-cli", "uvx aider-chat", "claude-code", "echo Y2xhdWRl | base64 -d | xargs env",
    "echo cl aude | tr -d x | xargs nohup", "awk 'BEGIN { system(1) }' x", "zzgatetool > .git/hooks/pre-commit", "zzgatetool > .envrc",
    "zzgatetool > .grok/settings.json", "zzgatetool > .vscode/tasks.json", "zzgatetool > ./.git/config", "zzgatetool > /opt/elsewhere/x",
];

#[test]
fn a_command_nothing_approves_yet_is_not_yet() {
    for command in NOT_YET {
        assert_eq!(kind(command), GateKind::NotYet, "{command}");
    }
}

#[test]
fn a_boundary_stays_a_boundary() {
    for command in BOUNDARY.iter().chain(DISGUISED) {
        assert_eq!(kind(command), GateKind::Boundary, "{command}");
    }
}

#[test]
fn an_approved_command_is_never_called_not_yet() {
    assert_eq!(kind("ls"), GateKind::Boundary);
}

#[test]
fn words_are_read_as_the_shell_names_them() {
    let read = |text: &str| words(text).collect::<Vec<_>>();
    assert_eq!(read("--output=~/.zshrc"), ["--output", "~/.zshrc"]);
    assert_eq!(read(">>~/'.zsh'\"rc\","), ["~/.zshrc"]);
    assert_eq!(read("c\\la\"ude\";x|y"), ["claude", "x", "y"]);
}

/// Ways to spell `~/.zshrc`, `~/.ssh/id_rsa` and the rest so the shell reads the same name.
fn disguise(place: &str, how: usize, at: usize) -> String {
    let at = at % place.len().max(1);
    let (head, tail) = place.split_at(at);
    match how {
        0 => format!("{head}\"\"{tail}"),
        1 => format!("{head}''{tail}"),
        2 if !tail.is_empty() => format!("{head}\\{tail}"),
        3 => format!("'{place}'"),
        _ => place.to_string(),
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

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

    /// Metamorphic: quoting a boundary differently, or reaching it from a `cd`, names the same place.
    #[test]
    fn a_write_to_a_runs_later_place_is_a_boundary_whatever_the_command_or_spelling(
        name in "zz[a-z]{2,8}",
        place in prop::sample::select(&["~/.zshrc", "~/.bashrc", "~/.local/bin/x", "~/.claude/settings.json", "~/.grok/config.toml", "~/Library/LaunchAgents/x.plist", "~/.ssh/id_rsa"][..]),
        how in 0..5usize,
        at in 2..40usize,
        form in 0..5usize,
    ) {
        let spelled = disguise(place, how, at);
        let (dir, file) = place.rsplit_once('/').unwrap_or(("~", place));
        let command = match form {
            0 => format!("{name} > {spelled}"),
            1 => format!("{name} --out={spelled}"),
            2 => format!("{name} {spelled},"),
            3 => format!("cd {dir} && {name} > {file}"),
            _ => format!("{name} {spelled}"),
        };
        prop_assert_eq!(kind(&command), GateKind::Boundary, "{}", command);
    }

    /// Metamorphic: an agent's name, however quoted, is still the agent.
    #[test]
    fn an_agent_is_a_boundary_however_its_name_is_quoted(
        agent in prop::sample::select(&["claude", "codex", "gemini", "grok", "safe-chains"][..]),
        how in 0..5usize,
        at in 1..10usize,
        wrapped in any::<bool>(),
    ) {
        let spelled = disguise(agent, how, at);
        let command = if wrapped { format!("zzgatetool run {spelled} -p hi") } else { format!("{spelled} -p hi") };
        prop_assert_eq!(kind(&command), GateKind::Boundary, "{}", command);
    }
}

fn in_project<T>(root: String, body: impl FnOnce() -> T) -> T {
    let _ctx = enter(PathCtx { cwd: Some(root.clone()), root: Some(root), session_id: None });
    body()
}

fn in_gate_probe<T>(body: impl FnOnce() -> T) -> T {
    in_project(format!("{}/projects/gateprobe", home()), body)
}

fn segment(text: &str) -> GateKind {
    in_gate_probe(|| segment_kind(text, &|c| crate::is_safe_command(c), false))
}

#[test]
fn a_refused_segment_is_a_boundary_only_when_its_reason_is() {
    assert_eq!(segment("cat ~/.ssh/id_rsa"), GateKind::Boundary);
    assert_eq!(segment("jj -R ~/.claude log"), GateKind::NotYet);
}

#[test]
fn a_runs_later_place_is_a_boundary_unless_the_segment_only_reads_it() {
    assert_eq!(segment("tee ~/Library/LaunchAgents/x.plist"), GateKind::Boundary);
    assert_eq!(segment("cat ~/Library/LaunchAgents/x.plist"), GateKind::NotYet);
}

#[test]
fn only_credentials_devices_and_frozen_places_are_boundary_reasons() {
    let boundary = [
        ReachReason::Credential,
        ReachReason::OtherUsersHome,
        ReachReason::RawDevice,
        ReachReason::FrozenTrustFile,
        ReachReason::FrozenTrustRoot,
        ReachReason::FrozenSystemIntegrity,
        ReachReason::Unconfined,
    ];
    for reason in boundary {
        assert!(is_boundary_reason(reason), "{reason:?}");
    }
    for reason in [ReachReason::RepositorySnapshot, ReachReason::OutsideWorkspace, ReachReason::ForeignTemp] {
        assert!(!is_boundary_reason(reason), "{reason:?}");
    }
}

#[test]
fn a_command_reaches_a_boundary_when_an_operand_names_one() {
    in_gate_probe(|| {
        assert!(reaches_a_boundary("cat ~/.ssh/id_rsa"));
        assert!(!reaches_a_boundary("cat ./notes.txt"));
    });
}

#[test]
fn a_whole_command_that_cannot_be_read_is_a_boundary_though_its_refused_segment_can() {
    assert_eq!(kind("zzgatetool sync"), GateKind::NotYet);
    assert_eq!(kind("echo '*' && zzgatetool sync"), GateKind::Boundary);
}

#[test]
fn only_a_user_name_after_a_tilde_is_opaque() {
    assert!(!text_is_opaque("zzgatetool ~"));
    assert!(!text_is_opaque("zzgatetool ~/notes"));
    assert!(text_is_opaque("zzgatetool ~zzsomeone"));
}

#[test]
fn a_word_leaves_the_workspace_when_it_resolves_outside_it() {
    in_gate_probe(|| {
        assert!(leaves_the_workspace(".."));
        assert!(leaves_the_workspace("~"));
        assert!(!leaves_the_workspace("~/projects/gateprobe/notes"));
        assert!(!leaves_the_workspace("a/b"));
    });
}

#[test]
fn a_tilde_before_a_name_is_not_expanded_to_the_home_folder() {
    in_project(format!("{}x", home()), || {
        assert!(leaves_the_workspace("~x/y"));
    });
}

#[test]
fn a_protected_place_is_named_by_the_home_folder_and_by_a_protected_path() {
    in_gate_probe(|| {
        assert!(names_a_protected_place("~"));
        assert!(names_a_protected_place("~/.config/safe-chains.toml"));
        assert!(!names_a_protected_place("notes"));
    });
}

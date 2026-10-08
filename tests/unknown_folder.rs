//! The generated adversarial set for writes in an unknown folder (docs/design/unknown-folder-writes.md
//! §8, sets 2 and 3).
//!
//! Every sensitive name the region table, the anchor module's own lists and the usual dotfiles
//! know, in every write spelling, with the command run in the unknown folder or a directory below
//! it. No folder level may approve any of them: a miss is a bug. Beside them, near-miss inputs that
//! `developer` must approve, so a level that refuses everything cannot pass. And the accepted risk
//! (§6): generic names that would land in a sensitive folder if the agent ran the command there,
//! counted per level so the docs can state the number.

use std::collections::BTreeSet;

use safe_chains::pathctx::anchor::{FolderLevel, GIT_HOOKS, NAMED_FILES};
use safe_chains::pathctx::{self, PathCtx, folder};
use safe_chains::targets::UNKNOWN_WORKDIR;
use safe_chains::verdict::{SafetyLevel, Verdict};

const ROOT: &str = "/work/projects/app";
const LEVELS: [FolderLevel; 3] = [FolderLevel::Reads, FolderLevel::Developer, FolderLevel::Workspace];

/// Whether a hook at `level`, told nothing about the folder, approves `command`.
fn approved(level: FolderLevel, command: &str) -> bool {
    let _ctx = pathctx::enter(PathCtx { cwd: Some(UNKNOWN_WORKDIR.to_string()), root: Some(ROOT.to_string()), session_id: None });
    let _folder = folder::enter(level);
    match safe_chains::command_verdict(command) {
        Verdict::Allowed(l) => level > FolderLevel::Reads || l <= SafetyLevel::SafeRead,
        Verdict::Denied => false,
    }
}

const SPELLINGS: &[&str] = &[
    "echo x > {t}", "echo x >> {t}", "tee {t}", "tee -a {t}", "cp a.txt {t}", "mv a.txt {t}", "install -m 644 a.txt {t}",
    "ln -s a.txt {t}", "sed -i s/a/b/ {t}", "sort -o {t} a.txt", "curl -s -o {t} https://example.com/x", "touch {t}", "rm -f {t}",
];

const DOTFILES: &[&str] = &[
    ".zshrc", ".bashrc", ".bash_profile", ".profile", ".zprofile", ".gitconfig", ".envrc", ".npmrc", ".netrc", ".config/fish/config.fish",
    ".config/autostart/x.desktop", ".config/systemd/user/x.service", ".local/bin/ls", ".ssh/authorized_keys", ".aws/credentials",
    ".git/config", ".github/workflows/ci.yml", ".vscode/tasks.json",
];

const CLIMBING: &[&str] = &["../x", "../../x", "a/../../x", "./../x", "sub/../../x"];

/// Every relative spelling of a place the region table names: a segment node as a directory and
/// below one, and a home node with its `~/` taken off, so the generator grows with the table.
fn region_targets() -> BTreeSet<String> {
    let src = include_str!("../regions/default.toml");
    let file: toml::Value = toml::from_str(src).expect("regions/default.toml parses");
    let mut out = BTreeSet::new();
    for node in file.get("region").and_then(toml::Value::as_array).expect("[[region]] nodes") {
        let Some(path) = node.get("path").and_then(toml::Value::as_str) else { continue };
        if let Some(home) = path.strip_prefix("~/") {
            let home = home.trim_end_matches('/');
            out.insert(home.to_string());
            out.insert(format!("{home}/x"));
        } else if !path.starts_with('/') {
            out.insert(path.to_string());
            out.insert(format!("{path}/x"));
            out.insert(format!("src/{path}/x"));
        }
    }
    out
}

/// Names sensitive wherever they sit, so also below a `cd` into a subdirectory of the folder.
fn names_anywhere() -> BTreeSet<String> {
    let mut targets = BTreeSet::new();
    targets.extend(NAMED_FILES.iter().map(|n| (*n).to_string()));
    targets.extend(NAMED_FILES.iter().map(|n| format!("keys/{n}")));
    targets.extend(GIT_HOOKS.iter().map(|h| format!("hooks/{h}")));
    targets.extend(GIT_HOOKS.iter().map(|h| format!(".git/hooks/{h}")));
    targets.extend(DOTFILES.iter().map(|d| (*d).to_string()));
    targets
}

fn hazards() -> BTreeSet<String> {
    let spell = |t: &str| SPELLINGS.iter().map(move |s| s.replace("{t}", &format!("'{t}'"))).collect::<Vec<_>>();
    let mut out = BTreeSet::new();
    for t in region_targets()
        .iter()
        .chain(names_anywhere().iter())
        .map(String::as_str)
        .chain(CLIMBING.iter().copied())
    {
        out.extend(spell(t));
    }
    // Below `cd sub` a path is relative to `sub`: `../x` is back inside the folder and a home
    // place's name is no longer at home, so only what is sensitive anywhere, and two-level climbs.
    for t in names_anywhere().iter().map(String::as_str).chain(["../../x", "../../../x"]) {
        out.extend(spell(t).into_iter().map(|c| format!("cd sub && {c}")));
    }
    out
}

#[test]
fn no_level_approves_a_sensitive_or_climbing_write() {
    let hazards = hazards();
    let distinct_targets: BTreeSet<&str> = hazards.iter().filter_map(|c| c.rsplit('\'').nth(1)).collect();
    assert!(distinct_targets.len() >= 50, "only {} distinct hazard targets; the bar is 50", distinct_targets.len());
    for level in LEVELS {
        let misses: Vec<&String> = hazards.iter().filter(|c| approved(level, c)).collect();
        assert!(
            misses.is_empty(),
            "{} approved {} of {} hazards:\n{}",
            level.name(),
            misses.len(),
            hazards.len(),
            misses.iter().take(40).map(|s| s.as_str()).collect::<Vec<_>>().join("\n")
        );
    }
}

/// Approved at `developer` with the folder unknown: ordinary names in a project, some of them
/// one letter away from a sensitive one.
const NEAR_MISSES: &[&str] = &[
    "echo x > config.json", "tee notes/ssh.md", "mkdir -p build", "mkdir -p out/gen", "echo x > src/hooks.ts", "tee src/hooks/useThing.ts",
    "cp a.txt docs/credentials.md", "echo x > library.md", "touch bin.rs", "echo x >> README.md", "sed -i s/a/b/ src/main.rs",
    "cp a.txt b.txt", "cp a.txt out/b.txt", "touch notes.md", "tee -a log.txt", "echo x > ./out.txt", "cd sub && echo x > out.txt",
    "echo x > keys.txt", "echo x > environment.ts", "echo x > rc.txt", "echo x > hooks/README.md", "echo x > Library.md",
    "echo x > a/../b.txt",
];

#[test]
fn developer_approves_the_near_misses() {
    assert!(NEAR_MISSES.len() >= 20);
    let refused: Vec<&&str> = NEAR_MISSES.iter().filter(|c| !approved(FolderLevel::Developer, c)).collect();
    assert!(refused.is_empty(), "developer refused near misses: {refused:?}");
    let at_reads: Vec<&&str> = NEAR_MISSES.iter().filter(|c| approved(FolderLevel::Reads, c)).collect();
    assert!(at_reads.is_empty(), "reads approved writes: {at_reads:?}");
}

/// The residual §6 accepts: an ordinary name written in a sensitive folder. The folder is what is
/// unknown, so whether the name is approved does not depend on it; each approved name counts once
/// per sensitive folder. Published in docs/design/unknown-folder-writes.md.
const GENERIC: &[&str] = &["config", "config.json", "config.toml", "hosts", "settings.yml", "x.plist", "data.txt", "ls", "AGENTS.md"];
const SENSITIVE_FOLDERS: &[&str] = &[
    "~/.ssh", "~/.aws", "~/.kube", "~/.docker", "~/.gnupg", "~/.config/git", "~/Library/LaunchAgents", "~/bin", "~/.local/bin", "~/.codex",
    "~/.claude", ".git",
];
const ACCEPTED_RISK: [(FolderLevel, usize); 3] = [(FolderLevel::Reads, 0), (FolderLevel::Developer, 108), (FolderLevel::Workspace, 108)];

#[test]
fn the_accepted_risk_is_the_published_number() {
    for (level, published) in ACCEPTED_RISK {
        let names = GENERIC.iter().filter(|n| approved(level, &format!("echo x >> {n}"))).count();
        assert_eq!(names * SENSITIVE_FOLDERS.len(), published, "{}: update the published count in the design doc", level.name());
    }
}

/// A copy, move, link or sync INTO the folder writes a name the source brings, not one the
/// destination shows: `cp /tmp/.zshrc .` writes `~/.zshrc` when the folder is home. Only a tool that
/// declares what it writes in its folder may write to the folder itself.
#[test]
fn nothing_brings_a_name_into_the_folder() {
    let mut hazards = Vec::new();
    for name in names_anywhere()
        .iter()
        .filter_map(|t| t.rsplit('/').next().map(str::to_string))
        .collect::<BTreeSet<_>>()
    {
        for into in [".", "./", "sub/.."] {
            for verb in ["cp", "cp -r", "mv", "install -m 644", "ln -s", "rsync -a"] {
                hazards.push(format!("{verb} '/tmp/x/{name}' {into}"));
            }
        }
    }
    for contents in ["rsync -a /tmp/home/ .", "cp -r /tmp/home/. .", "cp -R /tmp/home/ ./", "rsync -a /tmp/home/. sub/.."] {
        hazards.push(contents.to_string());
    }
    assert!(hazards.len() >= 50, "only {} hazards", hazards.len());
    for level in LEVELS {
        let misses: Vec<&String> = hazards.iter().filter(|c| approved(level, c)).collect();
        assert!(misses.is_empty(), "{} approved:\n{}", level.name(), misses.iter().map(|s| s.as_str()).collect::<Vec<_>>().join("\n"));
    }
    for tool in ["gofmt -w .", "shfmt -w .", "rubocop -a ."] {
        assert!(approved(FolderLevel::Developer, tool), "`{tool}` declares that it rewrites its own files, so it may write the folder");
    }
}

fn cli(args: &[&str]) -> (String, i32) {
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_safe-chains"))
        .args(args)
        .env("HOME", "/nonexistent/unknown-folder-cli-home")
        .env("SAFE_CHAINS_NO_LOCAL", "1")
        .output()
        .expect("run safe-chains");
    (String::from_utf8_lossy(&out.stdout).into_owned(), out.status.code().unwrap_or(-1))
}

/// `--unknown-folder` checks a command as the hook would with the folder unknown, and `--explain`
/// says how each write was judged, without the known-folder advice about the working directory.
#[test]
fn the_cli_checks_and_explains_with_the_folder_unknown() {
    let root = ["--root", ROOT];
    let (_, code) = cli(&[&root[..], &["--unknown-folder", "developer", "cargo fmt"]].concat());
    assert_eq!(code, 0, "developer approves cargo fmt");
    let (_, code) = cli(&[&root[..], &["--unknown-folder", "reads", "cargo fmt"]].concat());
    assert_eq!(code, 1, "reads leaves it to the prompt");
    let (out, code) = cli(&[
        &root[..],
        &["--unknown-folder", "developer", "--explain", "echo x > notes.md && echo x >> .zshrc && rm -rf build && mise trust"],
    ]
    .concat());
    assert_eq!(code, 1);
    for expected in [
        "folder unknown: writes are judged at `developer`",
        "`notes.md` is relative-plain",
        "`.zshrc` is relative-sensitive",
        "`build` is relative-plain: `developer` leaves deleting",
        "`mise trust` writes in its folder",
    ] {
        assert!(out.contains(expected), "missing `{expected}` in:\n{out}");
    }
    assert!(!out.contains("outside the working directory"), "the known-folder advice does not apply:\n{out}");
    assert!(!out.contains("`x` is"), "an echoed word is not a path:\n{out}");
    let (out, code) = cli(&[&root[..], &["--unknown-folder", "reads", "--explain", "cargo fmt"]].concat());
    assert_eq!(code, 1, "reads does not approve a write:\n{out}");
    assert!(!out.contains("auto-approves"), "the header agrees with the verdict:\n{out}");
    assert!(out.contains("`reads` approves no write"), "{out}");
    let (out, code) = cli(&["--level", "reader", "--explain", "cargo fmt"]);
    assert_eq!(code, 1);
    assert!(!out.contains("auto-approves"), "--explain applies --level's ceiling too:\n{out}");
}

/// What an adversarial review found approved on the first build (2026-10-08), each now refused at
/// every level: run-time items standing in for a name, outputs named after an input, the ends of
/// home places written from inside one, the harnesses' own files, an assignment moving a declared
/// write, and a redirect target the shell expands.
#[test]
fn the_review_findings_stay_refused() {
    let findings = [
        "ls -A | xargs -I{} sh -c 'echo x >> {}'",
        "echo .zshrc | xargs -I{} sh -c 'echo x >> {}'",
        "echo hooks/pre-commit | xargs -I% cp /tmp/evil %",
        "ls -a | xargs sed -i s/a/b/",
        "ls -A | while read f; do echo x >> \"$f\"; done",
        "ls -A | xargs rm -rf",
        "echo .zshrc | xargs rm",
        "unxz -f authorized_keys.xz",
        "bunzip2 authorized_keys.bz2",
        "zstd -d authorized_keys.zst",
        "rsync -a /tmp/evil/ hooks/",
        "ditto /tmp/evil hooks",
        "xz a",
        "echo x > LaunchAgents/a.plist",
        "cp /tmp/x.plist LaunchAgents/",
        "echo x > git/config",
        "echo x > fish/conf.d/x.fish",
        "echo x > systemd/user/x.service",
        "echo x > autostart/x.desktop",
        "echo x > nvim/init.lua",
        "echo x > direnv/direnvrc",
        "echo x > gh/hosts.yml",
        "echo x > share/applications/x.desktop",
        "echo x > hooks.json",
        "echo x > rules/default.rules",
        "echo x > settings.local.json",
        "GIT_INDEX_FILE=.zshrc git add .",
        "CARGO_TARGET_DIR=../.. cargo build",
        "mise use -p .mise.toml node@20",
        "mise set --file .zshrc FOO=1",
        "echo x > {.,}envrc",
        "echo x >> {.zshrc,a}",
        "sed -i'.profil*' s/x/x/ e",
        "sed -i'.git/hooks/*' s/x/x/ pre-commit",
        "perl -i'.git/hooks/*' -pe 1 pre-commit",
        "ls | while read f; do sh -c \"echo x >> $f\"; done",
        "f(){ echo x >> \"$1\"; }; ls -A | while read l; do f \"$l\"; done",
        "cp a .//",
        "cp a ./.",
        "echo x > launchagents/x.plist",
        "echo x > Library//LaunchAgents/x",
    ];
    for level in LEVELS {
        let misses: Vec<&&str> = findings.iter().filter(|c| approved(level, c)).collect();
        assert!(misses.is_empty(), "{} approved: {misses:?}", level.name());
    }
}

/// Relative reads are approved at every level (the owner, 2026-10-08: running from `~` is the
/// user's choice), except where the path could be a secret: a credential-shield node as written or
/// under `~`, the end of one, a key file, or a path climbing out of the folder.
#[test]
fn a_relative_read_is_approved_unless_it_could_be_a_secret() {
    let src = include_str!("../regions/default.toml");
    let file: toml::Value = toml::from_str(src).expect("regions/default.toml parses");
    let mut secrets: BTreeSet<String> =
        ["id_rsa", "id_ed25519", ".ssh/id_rsa", "credentials", "Keychains/login.keychain-db", "../x", "../../etc/x"]
            .iter()
            .map(|s| (*s).to_string())
            .collect();
    for node in file.get("region").and_then(toml::Value::as_array).expect("[[region]] nodes") {
        let (Some(path), Some(role)) = (node.get("path").and_then(toml::Value::as_str), node.get("role").and_then(toml::Value::as_str))
        else {
            continue;
        };
        if role != "credential-store" {
            continue;
        }
        if let Some(home) = path.strip_prefix("~/") {
            secrets.insert(if home.ends_with('/') { format!("{home}x") } else { home.to_string() });
        } else if !path.starts_with('/') {
            secrets.insert(format!("{path}/x"));
        }
    }
    let ordinary =
        ["grep -r foo src", "ls src", "cat README.md", "grep -n x tests/*.rs", "cat .gitignore", "grep -r foo .", "head -5 src/main.rs"];
    for level in LEVELS {
        for command in ordinary {
            assert!(approved(level, command), "`{command}` is refused at {}", level.name());
        }
        for secret in &secrets {
            for spelling in ["cat '{s}'", "grep -r x '{s}'", "head '{s}'"] {
                let command = spelling.replace("{s}", secret);
                assert!(!approved(level, &command), "`{command}` is approved at {}", level.name());
            }
        }
    }
    assert!(secrets.len() >= 20, "only {} secrets", secrets.len());
}

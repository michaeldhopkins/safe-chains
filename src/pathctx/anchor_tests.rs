use super::*;
use proptest::prelude::*;

#[test]
fn each_anchor_value_has_its_examples_and_near_misses() {
    let cases: &[(&str, Anchor)] = &[
        ("/tmp/x", Anchor::Free),
        ("/dev/null", Anchor::Free),
        ("~/.zshrc", Anchor::Free),
        ("/etc/hosts", Anchor::Free),
        ("out.txt", Anchor::RelativePlain),
        ("build/x", Anchor::RelativePlain),
        ("./src/gen.rs", Anchor::RelativePlain),
        ("a/../b.txt", Anchor::RelativePlain),
        ("notes/ssh.md", Anchor::RelativePlain),
        ("config.json", Anchor::RelativePlain),
        ("config", Anchor::RelativePlain),
        ("src/hooks/useThing.ts", Anchor::RelativePlain),
        (".", Anchor::RelativePlain),
        (".zshrc", Anchor::RelativeSensitive),
        (".git/hooks/pre-commit", Anchor::RelativeSensitive),
        ("hooks/pre-commit", Anchor::RelativeSensitive),
        ("HOOKS/Pre-Commit", Anchor::RelativeSensitive),
        (".envrc", Anchor::RelativeSensitive),
        ("authorized_keys", Anchor::RelativeSensitive),
        ("keys/Authorized_Keys", Anchor::RelativeSensitive),
        ("Library/LaunchAgents/x.plist", Anchor::RelativeSensitive),
        (".config/fish/config.fish", Anchor::RelativeSensitive),
        ("config.fish", Anchor::RelativeSensitive),
        ("bin/ls", Anchor::RelativeSensitive),
        ("bin", Anchor::RelativeSensitive),
        ("credentials", Anchor::RelativeSensitive),
        ("settings.json", Anchor::RelativeSensitive),
        ("safe-chains.toml", Anchor::RelativeSensitive),
        ("../x", Anchor::RelativeUnplaced),
        ("a/../../x", Anchor::RelativeUnplaced),
        ("$DIR/x", Anchor::RelativeUnplaced),
        ("*.txt", Anchor::RelativeUnplaced),
        ("out/[ab]", Anchor::RelativeUnplaced),
        ("", Anchor::RelativeUnplaced),
    ];
    for (path, want) in cases {
        assert_eq!(of_path(path), *want, "{path}");
    }
}

#[test]
fn developer_places_writes_of_plain_paths_and_no_rebind() {
    let d = FolderLevel::Developer;
    assert_eq!(placement("out.txt", Use::Write, d).as_deref(), Some("out.txt"));
    assert_eq!(placement("./a/../b", Use::Write, d).as_deref(), Some("b"));
    assert_eq!(placement(".", Use::Write, d).as_deref(), Some("."));
    assert_eq!(placement("build", Use::Rebind, d), None);
    assert_eq!(placement(".zshrc", Use::Write, d), None);
    assert_eq!(placement("../lib/x", Use::Write, d), None);
    assert_eq!(placement("/tmp/x", Use::Write, d), None, "an anchor-free path needs no placing");
}

#[test]
fn workspace_also_places_rebinds_and_one_hop_into_a_sibling() {
    let w = FolderLevel::Workspace;
    assert_eq!(placement("build", Use::Rebind, w).as_deref(), Some("build"));
    assert_eq!(placement(".", Use::Rebind, w), None, "the folder itself is never removed");
    assert_eq!(placement("./", Use::Rebind, w), None);
    assert_eq!(placement("../lib/x", Use::Write, w).as_deref(), Some("../lib/x"));
    assert_eq!(placement("../lib/x", Use::Rebind, w).as_deref(), Some("../lib/x"));
    assert_eq!(placement("../lib", Use::Write, w).as_deref(), Some("../lib"));
    assert_eq!(placement("../lib", Use::Rebind, w), None, "a whole sibling is not removed");
    for refused in ["../.ssh/x", "../../x", "../lib/../../x", "../Library/LaunchAgents/x", "../$X", ".zshrc", "../"] {
        assert_eq!(placement(refused, Use::Write, w), None, "{refused}");
    }
}

#[test]
fn reads_places_no_write() {
    for path in ["out.txt", "build/x", "../lib/x"] {
        for use_ in [Use::Write, Use::Rebind] {
            assert_eq!(placement(path, use_, FolderLevel::Reads), None, "{path} {use_:?}");
        }
    }
}

/// The owner, 2026-10-08: if the agent reads from `~`, that is the user's choice, so a relative read is
/// placed at every level. A secret, a climb out and a glob above the last segment still are not.
#[test]
fn a_relative_read_is_placed_at_every_level_unless_it_could_be_a_secret() {
    for level in [FolderLevel::Reads, FolderLevel::Developer, FolderLevel::Workspace] {
        for (path, want) in [
            ("src", Some("src")),
            (".", Some(".")),
            ("tests/*.rs", Some("tests/*.rs")),
            (".github/workflows/ci.yml", Some(".github/workflows/ci.yml")),
            (".gitignore", Some(".gitignore")),
        ] {
            assert_eq!(placement(path, Use::Read, level).as_deref(), want, "{path} at {}", level.name());
        }
        for path in [".ssh/id_rsa", "id_rsa", "id_*", "credentials", "Keychains/x", "Library/Keychains/x", "../x", "*/x", ".*", "$X"] {
            assert_eq!(placement(path, Use::Read, level), None, "{path} at {}", level.name());
        }
    }
}

#[test]
fn level_names_round_trip() {
    for name in FolderLevel::NAMES {
        assert_eq!(FolderLevel::parse(name).map(FolderLevel::name), Some(name));
    }
    assert_eq!(FolderLevel::parse("Developer"), None);
    assert!(!FolderLevel::Reads.admits_implicit(Anchor::ImplicitOutput));
    assert!(FolderLevel::Developer.admits_implicit(Anchor::ImplicitSource));
    assert!(!FolderLevel::Workspace.admits_implicit(Anchor::RelativeSensitive));
}

fn segment() -> impl Strategy<Value = String> {
    prop_oneof![
        "[a-zA-Z0-9_-]{1,8}",
        Just(".".to_string()),
        Just("..".to_string()),
        Just(".git".to_string()),
        Just(".ssh".to_string()),
        Just("hooks".to_string()),
        Just("pre-commit".to_string()),
        Just("Library".to_string()),
        Just("LaunchAgents".to_string()),
        Just("bin".to_string()),
        Just("authorized_keys".to_string()),
        Just("*".to_string()),
        Just("$X".to_string()),
    ]
}

fn relative_path() -> impl Strategy<Value = String> {
    proptest::collection::vec(segment(), 1..6).prop_map(|s| s.join("/"))
}

proptest! {
    /// A placed path never climbs above the workspace, never names a sensitive place, and is one
    /// the level could see: whatever the placement says, it is a plain path below the root, or at
    /// `workspace` one hop into a plain sibling.
    #[test]
    fn a_placed_write_is_plain_and_stays_put(path in relative_path(), level in prop_oneof![Just(FolderLevel::Developer), Just(FolderLevel::Workspace)], use_ in prop_oneof![Just(Use::Write), Just(Use::Rebind)]) {
        if let Some(placed) = placement(&path, use_, level) {
            let below = placed.strip_prefix("../").unwrap_or(&placed);
            prop_assert!(placed == "." || of_path(below) == Anchor::RelativePlain, "{path} placed as {placed}");
            prop_assert!(!below.split('/').any(|s| s == ".." || s.starts_with('.') && s != "."), "{path} placed as {placed}");
            if placed.starts_with("../") {
                prop_assert_eq!(level, FolderLevel::Workspace);
            }
            if use_ == Use::Rebind {
                prop_assert_eq!(level, FolderLevel::Workspace);
                prop_assert!(placed != ".");
            }
        }
    }

    /// A placed read never climbs out and never names a secret.
    #[test]
    fn a_placed_read_stays_put_and_names_no_secret(path in relative_path(), level in prop_oneof![Just(FolderLevel::Reads), Just(FolderLevel::Developer), Just(FolderLevel::Workspace)]) {
        if let Some(placed) = placement(&path, Use::Read, level) {
            let segments: Vec<&str> = placed.split('/').filter(|s| !s.is_empty() && *s != ".").collect();
            prop_assert!(!segments.contains(&".."), "{path} placed as {placed}");
            prop_assert!(!read_is_secret(&segments), "{path} placed as {placed}");
        }
    }

    /// Looser levels place everything stricter ones do.
    #[test]
    fn placement_is_monotone_in_the_level(path in relative_path(), use_ in prop_oneof![Just(Use::Read), Just(Use::Write), Just(Use::Rebind)]) {
        let d = placement(&path, use_, FolderLevel::Developer);
        if d.is_some() {
            prop_assert_eq!(placement(&path, use_, FolderLevel::Workspace), d);
        }
    }

    /// A hidden segment anywhere below the folder is never plain, however it is reached.
    #[test]
    fn a_hidden_segment_is_never_plain(prefix in "[a-z]{1,6}(/[a-z]{1,6}){0,2}", hidden in "\\.[a-zA-Z0-9_-]{1,8}", suffix in "(/[a-z]{1,6}){0,2}") {
        let path = format!("{prefix}/{hidden}{suffix}");
        prop_assert_ne!(of_path(&path), Anchor::RelativePlain, "{}", path);
    }
}

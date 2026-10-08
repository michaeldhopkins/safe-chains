use super::edit::{self, Access};
use super::guard;
use super::*;
use proptest::prelude::*;

const COMMENTED: &str =
    "# My rules\nlevel = \"developer\"   # the default\n\n# Every project\n[[grant]]\npath = \"~/projects\"\nread = true\nwrite = true\n";

fn home() -> String {
    std::env::var("HOME").unwrap_or_default()
}

#[test]
fn a_grant_keeps_everything_else_in_the_file() {
    let (updated, message) = edit::grant(COMMENTED, "~/notes", Access::Read).unwrap();
    assert!(updated.starts_with(COMMENTED), "the existing text changed:\n{updated}");
    assert!(updated.contains("path = \"~/notes\"\nread = true\nwrite = false"), "{updated}");
    assert!(message.contains("Granted read access to `~/notes`"), "{message}");
    edit::check_loads(&updated).unwrap();
}

#[test]
fn a_grant_on_a_path_already_granted_merges_its_faces() {
    let (once, _) = edit::grant(COMMENTED, "~/notes", Access::Read).unwrap();
    let (twice, message) = edit::grant(&once, "~/notes", Access::Write).unwrap();
    assert_eq!(twice.matches("[[grant]]").count(), 2, "a second table was added:\n{twice}");
    assert!(twice.contains("path = \"~/notes\"\nread = true\nwrite = true"), "{twice}");
    assert!(message.contains("read and write"), "{message}");
    let (again, message) = edit::grant(&twice, "~/notes", Access::Read).unwrap();
    assert_eq!(again, twice);
    assert!(message.contains("already grants"), "{message}");
}

#[test]
fn a_grant_matches_the_other_spelling_of_a_home_path() {
    let absolute = format!("{}/projects", home());
    let source = format!("[[grant]]\npath = \"{absolute}\"\nread = true\n");
    let (updated, _) = edit::grant(&source, "~/projects", Access::Write).unwrap();
    assert_eq!(updated.matches("[[grant]]").count(), 1, "{updated}");
    let (removed, _) = edit::remove_grant(&updated, "~/projects").unwrap();
    assert!(!removed.contains("[[grant]]"), "{removed}");
}

#[test]
fn removing_the_only_grant_leaves_the_rest_as_it_was() {
    let (removed, message) = edit::remove_grant(COMMENTED, "~/projects").unwrap();
    assert!(removed.starts_with("# My rules\nlevel = \"developer\"   # the default\n"), "{removed}");
    assert!(!removed.contains("[[grant]]"), "{removed}");
    assert!(message.contains("Removed the grant on `~/projects`"), "{message}");
}

#[test]
fn removing_what_is_not_there_lists_what_is() {
    let err = edit::remove_grant(COMMENTED, "~/nope").unwrap_err();
    assert_eq!(err, EditError::NotFound { kind: "grant", key: "~/nope".into(), present: vec!["~/projects".into()] });
    assert!(err.to_string().contains("`~/projects`") && err.to_string().contains("Nothing was changed."), "{err}");
    let err = edit::remove_command(COMMENTED, "mytool").unwrap_err();
    assert!(err.to_string().contains("no command entry for `mytool`, and no command entries at all"), "{err}");
}

#[test]
fn an_added_entry_is_appended_and_approves_only_what_was_run() {
    let (updated, message) = edit::add_entry(COMMENTED, "zzdeploytool status --env prod", "SafeWrite").unwrap();
    assert!(updated.starts_with(COMMENTED));
    edit::check_loads(&updated).unwrap();
    let specs = crate::registry::load_toml(&updated, "custom-user").unwrap();
    let spec = specs.iter().find(|s| s.name == "zzdeploytool").expect("the entry loads");
    let tokens = |s: &str| s.split(' ').map(crate::parse::Token::from_test).collect::<Vec<_>>();
    assert!(crate::registry::dispatch_spec(&tokens("zzdeploytool status --env prod"), spec).is_allowed());
    assert!(!crate::registry::dispatch_spec(&tokens("zzdeploytool destroy"), spec).is_allowed());
    assert!(message.contains("It approves `zzdeploytool status` with the flag --env and one other argument"), "{message}");
    assert!(message.contains("remove command zzdeploytool"), "{message}");
}

#[test]
fn an_entry_cannot_replace_or_duplicate() {
    assert_eq!(edit::add_entry("", "gh pr merge 1", "SafeWrite").unwrap_err(), EditError::BuiltIn(vec!["gh".into()]));
    assert_eq!(edit::add_entry("", "ls", "SafeWrite").unwrap_err(), EditError::AlreadyApproved);
    assert_eq!(edit::add_entry("", "echo 'unterminated", "SafeWrite").unwrap_err(), EditError::Unparseable);
    let existing = "[[command]]\nname = \"zzmine\"\nmax_positional = 0\nlevel = \"Inert\"\n";
    assert_eq!(edit::add_entry(existing, "zzmine run", "SafeWrite").unwrap_err(), EditError::AlreadyHasEntry("zzmine".into()));
}

#[test]
fn an_entry_never_approves_an_agent() {
    for name in ["goose", "aider", "crush", "opencode"] {
        let err = edit::add_entry("", &format!("{name} run"), "SafeWrite").unwrap_err();
        assert!(matches!(err, EditError::RunsAnAgent(_) | EditError::BuiltIn(_)), "{name}: {err:?}");
    }
}

#[test]
fn a_file_that_would_not_load_is_never_written() {
    for broken in
        ["[[[", "[[command]]\nname = 12\n", "[[grant]]\nread = true\n", "grant = 3\n", "[[grant]]\npath = \"/x\"\nread = \"yes\"\n"]
    {
        assert!(matches!(edit::check_loads(broken), Err(EditError::DoesNotLoad(_))), "{broken:?}");
    }
    edit::check_loads("").unwrap();
    edit::check_loads(COMMENTED).unwrap();
}

#[test]
fn protected_paths_are_refused_whatever_the_spelling() {
    let h = home();
    for path in [
        "~/.ssh".to_string(),
        "~/.ssh/id_rsa".to_string(),
        format!("{h}/.ssh"),
        "~/.aws".to_string(),
        "~/.config".to_string(),
        "~/.config/safe-chains.toml".to_string(),
        "~/.claude/settings.json".to_string(),
        "/etc/sudoers".to_string(),
        "/dev/mem".to_string(),
    ] {
        let normalized = guard::normalize_path(&path).unwrap();
        assert!(matches!(guard::check_grant_path(&normalized), Err(EditError::Protected(..))), "{path} was grantable");
    }
}

#[test]
fn too_wide_and_wildcard_paths_are_refused() {
    let h = home();
    let parent = std::path::Path::new(&h).parent().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default();
    for path in ["/", "~", &h, &parent] {
        let normalized = guard::normalize_path(path).unwrap();
        assert!(matches!(guard::check_grant_path(&normalized), Err(EditError::TooBroad(_))), "{path}");
    }
    assert!(matches!(guard::check_grant_path("~/notes/*"), Err(EditError::Glob(_))));
    guard::check_grant_path("~/notes").unwrap();
    guard::check_grant_path("/opt/tools").unwrap();
}

#[test]
fn paths_are_normalized_to_one_spelling() {
    let h = home();
    assert_eq!(guard::normalize_path("~/notes/").unwrap(), "~/notes");
    assert_eq!(guard::normalize_path(&format!("{h}/notes/./a/..")).unwrap(), "~/notes");
    assert_eq!(guard::normalize_path("/opt//tools/").unwrap(), "/opt/tools");
    assert!(matches!(guard::normalize_path("~other/x"), Err(EditError::NotAbsolute(_))));
    assert!(matches!(guard::normalize_path("$HOME/x"), Err(EditError::NotAbsolute(_))));
    let cwd = std::env::current_dir().unwrap();
    let relative = guard::normalize_path("sub").unwrap();
    assert!(relative.ends_with("/sub") && !relative.contains(".."), "{relative} from {}", cwd.display());
}

#[test]
fn levels_have_one_spelling_each_way() {
    assert_eq!(entry_level("safe-read").unwrap(), "SafeRead");
    assert_eq!(entry_level("SafeWrite").unwrap(), "SafeWrite");
    assert_eq!(entry_level("inert").unwrap(), "Inert");
    assert_eq!(entry_level("").unwrap(), "SafeWrite");
    assert!(entry_level("yolo").is_err());
}

#[test]
fn every_error_says_nothing_changed() {
    let errors = [
        EditError::NoHome,
        EditError::Unreadable("x".into()),
        EditError::DoesNotLoad("x".into()),
        EditError::AlreadyApproved,
        EditError::Unparseable,
        EditError::BuiltIn(vec!["gh".into(), "git".into()]),
        EditError::RunsAnAgent("goose".into()),
        EditError::AlreadyHasEntry("x".into()),
        EditError::UnknownLevel("x".into()),
        EditError::NoAccess,
        EditError::NotAbsolute("x".into()),
        EditError::Glob("x".into()),
        EditError::TooBroad("/".into()),
        EditError::Protected("~/.ssh".into(), Protected::Credential),
        EditError::NotFound { kind: "grant", key: "x".into(), present: vec![] },
        EditError::WriteFailed("x".into()),
        EditError::CannotEditCleanly,
    ];
    for e in errors {
        let text = e.to_string();
        assert!(text.ends_with("Nothing was changed."), "{text}");
        assert!(!text.contains('—') && !text.contains(';'), "{text}");
    }
}

fn arb_comment() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9 .,]{0,30}"
}

fn arb_folder() -> impl Strategy<Value = String> {
    "[a-z][a-z0-9_-]{0,10}(/[a-z0-9_-]{1,8}){0,2}".prop_map(|p| format!("~/{p}"))
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// Granting then removing the same path gives back the file it started from, comments included.
    #[test]
    fn grant_then_remove_is_the_identity(comment in arb_comment(), folder in arb_folder(), read in any::<bool>()) {
        let original = format!("# {comment}\nlevel = \"developer\"\n");
        let access = if read { Access::Read } else { Access::Write };
        let (granted, _) = edit::grant(&original, &folder, access).unwrap();
        edit::check_loads(&granted).unwrap();
        let (removed, _) = edit::remove_grant(&granted, &folder).unwrap();
        prop_assert_eq!(removed, original);
    }

    /// An edit never touches a line it was not about: every line of the original survives a grant.
    #[test]
    fn a_grant_keeps_every_line(comments in prop::collection::vec(arb_comment(), 0..5), folder in arb_folder()) {
        let original: String = comments.iter().map(|c| format!("# {c}\n")).collect::<String>() + COMMENTED;
        let (granted, _) = edit::grant(&original, &folder, Access::ReadWrite).unwrap();
        prop_assert!(granted.starts_with(&original));
    }

    /// Granting is idempotent: the second grant of the same faces changes nothing.
    #[test]
    fn granting_twice_is_granting_once(folder in arb_folder()) {
        let (once, _) = edit::grant(COMMENTED, &folder, Access::Read).unwrap();
        let (twice, _) = edit::grant(&once, &folder, Access::Read).unwrap();
        prop_assert_eq!(once, twice);
    }

    /// Adding then removing an entry gives back the file it started from.
    #[test]
    fn add_then_remove_is_the_identity(sub in "[a-z][a-z0-9-]{1,8}", flag in "--[a-z]{2,6}") {
        let command = format!("zzproptool {sub} {flag}");
        let (added, _) = edit::add_entry(COMMENTED, &command, "SafeRead").unwrap();
        edit::check_loads(&added).unwrap();
        let (removed, _) = edit::remove_command(&added, "zzproptool").unwrap();
        prop_assert_eq!(removed.trim_end(), COMMENTED.trim_end());
    }
}

proptest! {
    /// One spelling per path: normalizing a normalized path changes nothing.
    #[test]
    fn normalizing_twice_is_normalizing_once(parts in prop::collection::vec("[a-z0-9._-]{1,6}|\\.\\.|\\.", 0..6), tilde in any::<bool>(), slash in any::<bool>()) {
        let joined = parts.join("/");
        let raw = format!("{}{joined}{}", if tilde { "~/" } else { "/" }, if slash { "/" } else { "" });
        let once = guard::normalize_path(&raw).unwrap();
        prop_assert_eq!(guard::normalize_path(&once).unwrap(), once.clone());
        prop_assert!(!once.split('/').any(|p| p == ".." || p == "."), "{}", once);
    }

    /// Nothing inside a credential store is grantable, however it is spelled.
    #[test]
    fn nothing_inside_a_credential_store_is_grantable(store in prop::sample::select(vec![".ssh", ".aws", ".gnupg", ".kube"]), rest in "(/[a-z0-9_]{1,8}){0,3}", absolute in any::<bool>()) {
        let prefix = if absolute { home() } else { "~".to_string() };
        let path = guard::normalize_path(&format!("{prefix}/{store}{rest}")).unwrap();
        prop_assert!(matches!(guard::check_grant_path(&path), Err(EditError::Protected(_, Protected::Credential))), "{}", path);
    }
}

#[test]
fn a_file_written_with_crlf_keeps_crlf() {
    let crlf = COMMENTED.replace('\n', "\r\n");
    let (granted, _) = edit::grant(&crlf, "~/notes", Access::Read).unwrap();
    assert!(!granted.replace("\r\n", "").contains('\n'), "a bare newline crept in:\n{granted:?}");
    let (removed, _) = edit::remove_grant(&granted, "~/notes").unwrap();
    assert_eq!(removed, crlf);
}

#[test]
fn text_the_edit_was_not_about_is_left_byte_for_byte() {
    let original = "a.b = 1\nz = \"last\"\n\n[tool]\nx.y = 2   # dotted\n\n[[grant]]\npath = \"/opt/one\"\nread = true # keep this note\n\n[[grant]]\npath = \"/opt/two\"\nwrite = true\n\n[other]\nk = 1\n";
    let (granted, _) = edit::grant(original, "/opt/three", Access::Read).unwrap();
    assert!(granted.starts_with(original), "{granted}");
    let (back, _) = edit::remove_grant(&granted, "/opt/three").unwrap();
    assert_eq!(back, original);

    let (merged, _) = edit::grant(original, "/opt/one", Access::Write).unwrap();
    assert_eq!(merged, original.replace("read = true # keep this note\n", "read = true # keep this note\nwrite = true\n"));
    let (removed, _) = edit::remove_grant(original, "/opt/one").unwrap();
    assert_eq!(removed, original.replace("[[grant]]\npath = \"/opt/one\"\nread = true # keep this note\n\n", ""));
}

#[test]
fn removing_an_entry_takes_its_subcommands_and_comments_with_it() {
    let original = "level = \"developer\"\n\n# deploys\n[[command]]\nname = \"zzone\"\n\n[[command.sub]]\nname = \"status\"\nmax_positional = 0\n\n[[command]]\nname = \"zztwo\"\nmax_positional = 0\n";
    let (removed, _) = edit::remove_command(original, "zzone").unwrap();
    assert_eq!(removed, "level = \"developer\"\n\n[[command]]\nname = \"zztwo\"\nmax_positional = 0\n");
}

/// Found by the `config_edit` fuzz target: two entries with the same name, each claiming the blank
/// line between them, overlapped and panicked the cut.
#[test]
fn removing_two_neighbouring_entries_of_one_name() {
    let original = "# head\n\n[[command]]\nname = \"zzdup\"\nmax_positional = 1\n\n[[command]]\nname = \"zzdup\"\nmax_positional = 1\n";
    let (removed, _) = edit::remove_command(original, "zzdup").unwrap();
    assert_eq!(removed, "# head\n\n");
}

/// Found in review: each of these was granted, and a write there then auto-approved. Every one
/// holds settings or programs that run later: agents' settings, shell and login startup, git's
/// config, launchd, and folders on PATH. A folder above one is refused too, since the grant
/// would sweep it in.
#[test]
fn places_that_run_later_are_refused() {
    for path in [
        "~/.claude.json", "~/.codex", "~/.codex/config.toml", "~/.gemini", "~/.cursor", "~/.zshrc", "~/.bashrc", "~/.gitconfig",
        "~/Library/LaunchAgents", "~/Library", "/Library/LaunchDaemons", "/usr/local/bin", "/usr/local", "/opt/homebrew/bin", "/opt",
        "~/.local/bin", "~/.cargo", "~/bin",
    ] {
        let normalized = guard::normalize_path(path).unwrap();
        let refused = guard::check_grant_path(&normalized);
        assert!(matches!(refused, Err(EditError::Protected(..))), "{path} was grantable: {refused:?}");
    }
    for fine in ["~/notes", "~/projects", "/opt/tools", "~/Library/Mobile Documents/notes", "/srv/data"] {
        let normalized = guard::normalize_path(fine).unwrap();
        assert!(guard::check_grant_path(&normalized).is_ok(), "{fine} was refused");
    }
}

/// Every harness safe-chains installs into runs an agent, so none may get an entry.
#[test]
fn every_supported_harness_is_an_agent_command() {
    for target in crate::targets::registry() {
        let name = target.name();
        let binary = match name {
            "claude" | "codex" | "grok" | "qwen" | "droid" | "copilot" | "opencode" | "agy" | "gemini" => name,
            "cursor" => "cursor-agent",
            "antigravity" => "agy",
            other => other,
        };
        assert!(super::guard::AGENT_COMMANDS.contains(&binary), "{name} ({binary}) is missing from AGENT_COMMANDS");
    }
}

/// Found in review: each of these was written, and then approved more than the command shown.
#[test]
fn an_entry_approves_no_more_than_the_command_shown() {
    for (command, expected) in [
        ("Bash -c x", "BuiltIn"),
        ("RM -rf x", "BuiltIn"),
        ("Claude -p hi", "agent"),
        ("CODEX exec x", "agent"),
        ("q chat", "agent"),
        ("/tmp/zzevil build", "PathSpelled"),
        ("./zzevil.sh build", "PathSpelled"),
        ("zzmytool -o out.txt run", "SubAfterFlags"),
        ("zzmytool --config x.toml sync", "SubAfterFlags"),
        ("", "Unparseable"),
    ] {
        let err = edit::add_entry("", command, "SafeWrite").unwrap_err();
        let kind = match &err {
            EditError::BuiltIn(_) => "BuiltIn",
            EditError::RunsAnAgent(_) => "agent",
            EditError::PathSpelled(_) => "PathSpelled",
            EditError::SubAfterFlags(..) => "SubAfterFlags",
            EditError::Unparseable => "Unparseable",
            other => panic!("{command}: {other:?}"),
        };
        let either = expected == "agent" && kind == "BuiltIn";
        assert!(kind == expected || either, "{command}: {err}");
    }
    assert!(edit::add_entry("", "zzmytool --verbose", "SafeWrite").is_ok(), "a flag-only command is fine");
    assert!(edit::add_entry("", "zzmytool run -o out.txt", "SafeWrite").is_ok(), "the subcommand first is fine");
}

/// Found in review: removing a grant cut away a commented-out block above it, which the semantic
/// check cannot see because comments carry no meaning.
#[test]
fn commented_out_rules_above_a_removed_table_stay() {
    let original = "[[grant]]\npath = \"~/a\"\nread = true\n# disabled for now:\n# [[grant]]\n# path = \"~/old\"\n[[grant]]\npath = \"~/notes\"\nread = true\n";
    let (removed, _) = edit::remove_grant(original, "~/notes").unwrap();
    assert!(removed.contains("# [[grant]]\n# path = \"~/old\""), "{removed}");
    let described = "level = \"developer\"\n\n# notes for the agent\n[[grant]]\npath = \"~/notes\"\nread = true\n";
    let (removed, _) = edit::remove_grant(described, "~/notes").unwrap();
    assert_eq!(removed, "level = \"developer\"\n", "a comment describing the table goes with it");
}

/// Found in review: grants written as an inline array loaded fine, but `config grant` called the
/// file broken.
#[test]
fn grants_written_inline_are_left_to_a_person_not_called_broken() {
    let inline = "grant = [ { path = \"~/a\", read = true } ]\n";
    edit::check_loads(inline).unwrap();
    assert_eq!(edit::grant(inline, "~/b", Access::Read).unwrap_err(), EditError::CannotEditCleanly);
}

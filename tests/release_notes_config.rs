#[test]
fn release_notes_leave_out_todo_commits() {
    let text = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/cliff.toml")).expect("cliff.toml");
    let config: toml::Value = toml::from_str(&text).expect("cliff.toml is valid TOML");
    let git = config.get("git").expect("[git]");
    let excluded: Vec<&str> = git
        .get("exclude_paths")
        .and_then(toml::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(toml::Value::as_str)
        .collect();
    assert!(excluded.contains(&"TODO.md"), "a commit touching only TODO.md must not reach the release notes");
    let parsers = git.get("commit_parsers").and_then(toml::Value::as_array).expect("commit_parsers");
    let first = &parsers[0];
    assert_eq!(first.get("message").and_then(toml::Value::as_str), Some("^[a-z]+\\(todo\\)"), "the todo scope is skipped first");
    assert_eq!(first.get("skip").and_then(toml::Value::as_bool), Some(true));
}

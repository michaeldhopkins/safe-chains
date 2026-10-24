//! `mise x` is mise's own short name for `mise exec`, so every line spelled with it gets the verdict
//! the long spelling gets.

fn approved(command: &str) -> bool {
    safe_chains::is_safe_command(command)
}

#[test]
fn mise_x_is_judged_as_mise_exec() {
    for tail in [
        "node -- true", "node@20 -- ls", "'node[postinstall=touch pwned]' -- true", "node -- rm -rf /", "", "-- ls",
        "'cargo:x@path:/tmp/evil' -- ls",
    ] {
        let long = approved(&format!("mise exec {tail}"));
        assert_eq!(approved(&format!("mise x {tail}")), long, "`mise x {tail}` differs from `mise exec {tail}` ({long})");
    }
}

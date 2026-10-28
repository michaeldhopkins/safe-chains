//! Forms that stay approved after a tightening whose command-reference examples are held for the
//! owner's approval of the exact text. They are pinned here, where nothing renders them, until the
//! reference lists them.

fn approved(command: &str) -> bool {
    safe_chains::is_safe_command(command)
}

#[test]
fn approved_forms_of_borg_task_printf_and_mkdir_stay_approved() {
    for command in [
        "borg --bypass-lock list repo", "borg --lock-wait 5 list repo", "borg list --first 3 repo", "borg list --format '{name}' repo",
        "borg check --archives-only repo", "borg config repo append_only",
        "task -l -d ./sub", "task -l --taskfile ./Taskfile.yml", "task --list --json",
        "printf hello", "printf '%s\\n' one two", "printf '%5d|%-5s|\\n' 42 ab", "printf -- '%s\\n' -x", "printf '%%n is a literal\\n'",
        "gprintf '%b' 'a\\tb'", "printf --version", "printf --help",
        "mkdir build", "mkdir -p out/logs/archive", "mkdir -pv ./a ./b", "mkdir -m 755 ./dist", "mkdir --mode=700 ./private",
        "gmkdir --context ./labelled", "mkdir --version",
    ] {
        assert!(approved(command), "{command} must stay approved");
    }
}

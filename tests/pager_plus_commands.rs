//! A pager that runs a `+` operand as a command (`less '+!id'`) is refused that operand only when
//! it is seen as an operand. A flag listed as taking a value hides the next word from that check,
//! so a flag the pager treats as a switch must never be listed as valued: `less --LINE-NUMBERS
//! '+!id' a` runs `id`. Every less-family entry is checked against less's own option table.

use std::path::Path;

const LESS_TAKES_AN_ARGUMENT: &[&str] = &[
    "-b", "--buffers", "-h", "--max-back-scroll", "-j", "--jump-target", "-k", "--lesskey-file", "-o", "--log-file", "-O", "--LOG-FILE",
    "-p", "--pattern", "-P", "--prompt", "-t", "--tag", "-T", "--tag-file", "-x", "--tabs", "-y", "--max-forw-scroll", "-z", "--window",
    "-#", "--shift", "-D", "--color", "--header", "--rscroll", "--intr", "--line-num-width", "--status-col-width", "--wheel-lines",
];

fn toml_files(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("readable commands directory") {
        let path = entry.expect("readable entry").path();
        if path.is_dir() {
            toml_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "toml") {
            out.push(path);
        }
    }
}

fn strings(table: &toml::Table, key: &str) -> Vec<String> {
    table
        .get(key)
        .and_then(toml::Value::as_array)
        .map(|a| a.iter().filter_map(|v| v.as_str().map(str::to_string)).collect())
        .unwrap_or_default()
}

fn less_family() -> Vec<(String, Vec<String>, Vec<String>)> {
    let mut files = Vec::new();
    toml_files(Path::new(env!("CARGO_MANIFEST_DIR")).join("commands").as_path(), &mut files);
    let mut found = Vec::new();
    for file in files {
        let text = std::fs::read_to_string(&file).expect("readable definition");
        let Ok(doc) = text.parse::<toml::Table>() else { continue };
        for command in doc.get("command").and_then(toml::Value::as_array).into_iter().flatten() {
            let Some(table) = command.as_table() else { continue };
            let plus = strings(table, "reads_commands_from").iter().any(|s| s == "plus");
            let valued = strings(table, "valued");
            if plus && valued.iter().any(|f| f == "--pattern") {
                let name = table.get("name").and_then(toml::Value::as_str).expect("named command").to_string();
                for spelling in std::iter::once(name).chain(strings(table, "aliases")) {
                    found.push((spelling, strings(table, "standalone"), valued.clone()));
                }
            }
        }
    }
    found
}

#[test]
fn the_less_family_is_found() {
    let names: Vec<String> = less_family().into_iter().map(|(name, ..)| name).collect();
    for expected in ["less", "zless", "zmore", "bzless", "bzmore", "xzless", "xzmore", "lzless", "lzmore", "zstdless"] {
        assert!(names.iter().any(|n| n == expected), "{expected} missing from {names:?}");
    }
}

#[test]
fn a_less_family_flag_listed_as_valued_takes_an_argument_in_less() {
    for (name, _, valued) in less_family() {
        for flag in valued {
            assert!(
                LESS_TAKES_AN_ARGUMENT.contains(&flag.as_str()),
                "{name}: {flag} is a switch in less, so it must not be listed as valued"
            );
        }
    }
}

#[test]
fn a_plus_command_after_any_switch_is_refused() {
    for (name, standalone, valued) in less_family() {
        for flag in standalone.iter().chain(&valued).filter(|f| !LESS_TAKES_AN_ARGUMENT.contains(&f.as_str())) {
            for command in [format!("{name} {flag} '+!id' ./a"), format!("{name} {flag}=+!id ./a")] {
                assert!(!safe_chains::is_safe_command(&command), "{command} must be refused: less runs the + operand");
            }
        }
    }
}

/// The more macOS ships is less in more mode, where `-p` takes a command to run at startup, as a
/// `+` operand does. `LESS_IS_MORE` puts any less-family pager in that mode.
#[test]
fn a_more_mode_startup_command_is_refused() {
    for command in ["more -p '!id' ./a", "more -p'!id' ./a", "more -p '!rm -rf ~' ./a", "LESS_IS_MORE=1 less -p '!id' ./a"] {
        assert!(!safe_chains::is_safe_command(command), "{command} must be refused: more mode runs the -p value");
    }
    for (name, ..) in less_family() {
        for command in [format!("LESS_IS_MORE=1 {name} -p '!id' ./a"), format!("LESS_IS_MORE=1 {name} ./a")] {
            assert!(!safe_chains::is_safe_command(&command), "{command} must be refused: more mode runs the -p value");
        }
    }
    for command in ["more ./a", "more -n 5 ./a", "more +42 ./a", "less -p pattern ./a"] {
        assert!(safe_chains::is_safe_command(command), "{command} must stay approved");
    }
}

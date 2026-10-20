//! Which binary a `cargo run` starts, as far as it decides whether that binary is safe-chains.
//!
//! The rule-change check only matters when the binary run is this one, so it applies when `--bin`
//! names it, or when no `--bin` or `--example` is given and the package run is safe-chains: the one
//! `-p` / `--package` names, or the package of the nearest `Cargo.toml` above the folder cargo runs
//! in. That last case reads the manifest from disk. Anything not known before the shell runs, an
//! unknown working directory, or a manifest that names no package of its own counts as safe-chains.
use crate::parse::Token;
use crate::registry;

const THIS: &str = "safe-chains";

pub(crate) fn runs_safe_chains(tokens: &[Token], arguments_at: usize) -> bool {
    let Some(target) = Target::of(&tokens[1..arguments_at.min(tokens.len())]) else {
        return true;
    };
    if let Some(bin) = target.bin {
        return bin.contains(THIS);
    }
    if target.example {
        return false;
    }
    if let Some(package) = target.package {
        return package.contains(THIS);
    }
    nearest_manifest_names_this(target.manifest_path, target.chdir)
}

#[derive(Default)]
struct Target<'a> {
    bin: Option<&'a str>,
    example: bool,
    package: Option<&'a str>,
    manifest_path: Option<&'a str>,
    chdir: Option<&'a str>,
}

impl<'a> Target<'a> {
    fn of(words: &'a [Token]) -> Option<Self> {
        let valued = registry::sub_valued_flags("cargo", "run").unwrap_or_default();
        let short_valued: Vec<char> = valued.iter().filter(|v| v.len() == 2).filter_map(|v| v.chars().nth(1)).chain(['C']).collect();
        let mut target = Target::default();
        let mut words = words.iter().map(Token::as_str);
        while let Some(word) = words.next() {
            let (flag, value) = if let Some((flag, value)) = word.split_once('=').filter(|_| word.starts_with("--")) {
                (flag, Some(value))
            } else if word.starts_with("--") {
                let value = if valued.iter().any(|v| v == word) { Some(words.next()?) } else { None };
                (word, value)
            } else if word.len() > 1 && word.starts_with('-') {
                let letters = &word[1..];
                let Some(at) = letters.find(|c| short_valued.contains(&c)) else { continue };
                let rest = &letters[at + 1..];
                let value = if rest.is_empty() { words.next()? } else { rest };
                (
                    if letters[at..].starts_with('p') {
                        "-p"
                    } else if letters[at..].starts_with('C') {
                        "-C"
                    } else {
                        "-"
                    },
                    Some(value),
                )
            } else {
                continue;
            };
            let Some(value) = value else { continue };
            if !known_before_it_runs(value) {
                return None;
            }
            match flag {
                "--bin" => target.bin = Some(value),
                "--example" => target.example = true,
                "--package" | "-p" => target.package = Some(value),
                "--manifest-path" => target.manifest_path = Some(value),
                "-C" => target.chdir = Some(value),
                _ => {}
            }
        }
        Some(target)
    }
}

fn known_before_it_runs(word: &str) -> bool {
    !word.is_empty()
        && !word.contains("__SAFE_CHAINS")
        && !word.contains("sc_item")
        && word
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-' | '/' | '+' | ':' | '@' | ','))
}

fn nearest_manifest_names_this(manifest_path: Option<&str>, chdir: Option<&str>) -> bool {
    let cwd = crate::pathctx::cwd().filter(|c| c.starts_with('/') && c != crate::pathctx::UNRESOLVED_CWD);
    let place = |path: &str| -> Option<std::path::PathBuf> {
        if path.starts_with('/') { Some(path.into()) } else { cwd.as_deref().map(|c| std::path::Path::new(c).join(path)) }
    };
    if let Some(manifest) = manifest_path {
        return place(manifest).is_none_or(|path| manifest_names_this(&path));
    }
    let Some(start) = (match chdir {
        Some(dir) => place(dir),
        None => cwd.as_deref().map(std::path::PathBuf::from),
    }) else {
        return true;
    };
    start
        .ancestors()
        .map(|dir| dir.join("Cargo.toml"))
        .find(|path| path.is_file())
        .is_some_and(|path| manifest_names_this(&path))
}

fn manifest_names_this(path: &std::path::Path) -> bool {
    let Ok(text) = std::fs::read_to_string(path) else {
        return true;
    };
    text.contains(THIS) || !text.lines().any(|line| line.trim() == "[package]")
}

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn in_project(name: Option<&str>, cmd: &str) -> bool {
        let dir = std::env::temp_dir().join(format!("sc-cargo-run-{}-{}", std::process::id(), name.unwrap_or("virtual")));
        std::fs::create_dir_all(dir.join("src")).expect("project dir");
        let manifest = match name {
            Some(name) => format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\n"),
            None => "[workspace]\nmembers = [\"crates/*\"]\n".to_string(),
        };
        std::fs::write(dir.join("Cargo.toml"), manifest).expect("manifest");
        let _cwd = crate::pathctx::enter_cwd(Some(dir.join("src").to_string_lossy().into_owned()));
        is_safe_command(cmd)
    }

    #[test]
    fn another_project_may_be_run_with_any_arguments() {
        for cmd in [
            "cargo run -- config show", "cargo run -- config add x", "cargo run -- mcp", "cargo run -- $HOME", "cargo run -- 'a*'",
            "cargo run -- {a,b}", "cargo run -q -- mcp", "cargo run --bin tool -- mcp", "cargo run --example demo -- mcp",
            "cargo run -p tool -- mcp",
        ] {
            assert!(in_project(Some("tool"), cmd), "{cmd} should be approved in another project");
        }
    }

    #[test]
    fn this_project_is_recognised_however_it_is_named() {
        for cmd in [
            "cargo run -- config add x", "cargo run -q -- mcp", "cargo run --bin safe-chains -- mcp", "cargo run --bin=safe-chains -- mcp",
            "cargo run -p safe-chains -- mcp", "cargo run -psafe-chains -- mcp", "cargo run -qp safe-chains -- mcp",
            "cargo run -qpsafe-chains -- mcp", "cargo run --package=safe-chains -- mcp", "cargo run --package safe-chains -- mcp",
            "cargo run -p $P -- mcp", "cargo run --bin $B -- mcp", "cargo run -- -- mcp",
        ] {
            assert!(!in_project(Some("safe-chains"), cmd), "{cmd} must not be approved in this project");
        }
        for cmd in [
            "cargo run -p safe-chains -- mcp",
            "cargo run -qpsafe-chains -- mcp",
            "cargo run --bin safe-chains -- mcp",
            "cargo run --manifest-path ../../elsewhere/Cargo.toml -- mcp",
        ] {
            assert!(!in_project(Some("tool"), cmd), "{cmd} must not be approved from another project");
        }
    }

    #[test]
    fn a_workspace_with_no_package_of_its_own_counts_as_this_one() {
        assert!(!in_project(None, "cargo run -- mcp"));
        assert!(in_project(None, "cargo run -p tool -- mcp"));
    }

    #[test]
    fn an_unknown_working_directory_counts_as_this_project() {
        let _cwd = crate::pathctx::enter_cwd(None);
        assert!(!is_safe_command("cargo run -- mcp"));
        assert!(is_safe_command("cargo run -p tool -- mcp"));
    }
}

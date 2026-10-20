//! Which binary a `cargo run` starts, as far as it decides whether that binary is safe-chains.
//!
//! The rule-change check only matters when the binary run is this one, so it applies when `--bin`
//! names it, when `-p` / `--package` names safe-chains (in any case, with `_` read as `-`) and no `--bin` or `--example` is given, and
//! otherwise when the package of the nearest `Cargo.toml` is safe-chains. A `--bin` or `--example`
//! naming another target still reads that manifest, since any target of a package that is or uses
//! safe-chains can reach its rule changes. The nearest manifest is the one above the folder cargo
//! runs in, found after symbolic links and `..` are resolved, read from disk:
//! it is safe-chains when its package, one of its binaries or one of its dependencies is named
//! safe-chains (with `_` read as `-`). Anything not known before the shell runs, an unknown or
//! missing working directory, or a manifest that does not parse or names no package of its own
//! counts as safe-chains.
use crate::parse::Token;
use crate::registry;

const THIS: &str = "safe-chains";

pub(crate) fn runs_safe_chains(tokens: &[Token], arguments_at: usize) -> bool {
    let Some(target) = Target::of(&tokens[1..arguments_at.min(tokens.len())]) else {
        return true;
    };
    if target.bin.is_some_and(names_this) {
        return true;
    }
    if target.bin.is_none()
        && !target.example
        && let Some(package) = target.package
    {
        return names_this(package);
    }
    nearest_manifest_names_this(target.manifest_path, target.chdir)
}

fn names_this(name: &str) -> bool {
    name.to_ascii_lowercase().replace('_', "-").contains(THIS)
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
    let Ok(start) = std::fs::canonicalize(start) else {
        return true;
    };
    start
        .ancestors()
        .map(|dir| dir.join("Cargo.toml"))
        .find(|path| path.is_file())
        .is_some_and(|path| manifest_names_this(&path))
}

fn manifest_names_this(path: &std::path::Path) -> bool {
    let Some(manifest) = std::fs::read_to_string(path).ok().and_then(|text| toml::from_str::<toml::Table>(&text).ok()) else {
        return true;
    };
    let Some(package) = manifest.get("package").and_then(toml::Value::as_table) else {
        return true;
    };
    let is_this = |name: &str| name.replace('_', "-") == THIS;
    let field_is_this =
        |table: Option<&toml::Table>, field: &str| table.and_then(|t| t.get(field)).and_then(toml::Value::as_str).is_some_and(is_this);
    let mut binaries = manifest.get("bin").and_then(toml::Value::as_array).into_iter().flatten();
    field_is_this(Some(package), "name")
        || binaries.any(|bin| field_is_this(bin.as_table(), "name"))
        || dependency_tables(&manifest)
            .any(|deps| deps.iter().any(|(name, spec)| is_this(name) || field_is_this(spec.as_table(), "package")))
}

fn dependency_tables(manifest: &toml::Table) -> impl Iterator<Item = &toml::Table> {
    const KINDS: [&str; 3] = ["dependencies", "dev-dependencies", "build-dependencies"];
    let targets = manifest.get("target").and_then(toml::Value::as_table).into_iter().flat_map(|t| t.values());
    std::iter::once(manifest)
        .chain(targets.filter_map(toml::Value::as_table))
        .flat_map(|table| KINDS.iter().filter_map(|kind| table.get(*kind).and_then(toml::Value::as_table)))
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
            "cargo run -p $P -- mcp", "cargo run --bin $B -- mcp", "cargo run -- -- mcp", "cargo run -p safe_chains -- mcp",
            "cargo run --package=Safe_Chains -- mcp", "cargo run --bin safe_chains -- mcp",
        ] {
            assert!(!in_project(Some("safe-chains"), cmd), "{cmd} must not be approved in this project");
        }
        for cmd in [
            "cargo run -p safe-chains -- mcp",
            "cargo run -qpsafe-chains -- mcp",
            "cargo run --bin safe-chains -- mcp",
            "cargo run -p safe_chains -- mcp",
            "cargo run --bin safe_chains -- mcp",
            "cargo run --manifest-path ../../elsewhere/Cargo.toml -- mcp",
        ] {
            assert!(!in_project(Some("tool"), cmd), "{cmd} must not be approved from another project");
        }
    }

    /// Any binary or example of a package that is or depends on safe-chains can call its rule
    /// changes, so `--bin` and `--example` are taken at face value only in another project. Telling
    /// which target's source reaches that code would mean reading Rust, which this does not do.
    #[test]
    fn a_named_target_of_this_project_counts_as_this_project() {
        for cmd in [
            "cargo run --bin gen_fuzz_corpus -- config add x",
            "cargo run --bin=tool -- config add x",
            "cargo run --example generate_assets -- config add x",
            "cargo run --example=demo -- mcp",
        ] {
            assert!(!in_manifest("this-target", "[package]\nname = \"safe-chains\"\n", cmd), "{cmd} must not be approved in this project");
            let dependent = "[package]\nname = \"tool\"\n[dependencies]\nsafe-chains = \"0.1\"\n";
            assert!(!in_manifest("dependent-target", dependent, cmd), "{cmd} must not be approved in a dependent");
            assert!(in_manifest("other-target", "[package]\nname = \"tool\"\n", cmd), "{cmd} should be approved in another project");
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

    fn in_manifest(label: &str, manifest: &str, cmd: &str) -> bool {
        let dir = std::env::temp_dir().join(format!("sc-cargo-run-{}-{label}", std::process::id()));
        std::fs::create_dir_all(dir.join("src")).expect("project dir");
        std::fs::write(dir.join("Cargo.toml"), manifest).expect("manifest");
        let _cwd = crate::pathctx::enter_cwd(Some(dir.join("src").to_string_lossy().into_owned()));
        is_safe_command(cmd)
    }

    #[test]
    fn a_manifest_is_this_project_by_its_names_not_its_text() {
        let package = |rest: &str| format!("[package]\nname = \"tool\"\nversion = \"0.1.0\"\n{rest}");
        for (label, manifest) in [
            ("comment", "[package] # the classifier\nname = \"safe-chains\"\nversion = \"0.1.0\"\n".to_string()),
            ("underscore", "[package]\nname = \"safe_chains\"\n".to_string()),
            ("dependency", package("[dependencies]\nsafe_chains = { path = \"..\" }\n")),
            ("renamed", package("[dependencies]\nclassifier = { package = \"safe-chains\", version = \"0.1\" }\n")),
            ("dev", package("[dev-dependencies]\nsafe-chains = \"0.1\"\n")),
            ("target", package("[target.'cfg(unix)'.dependencies]\nsafe-chains = \"0.1\"\n")),
            ("binary", package("[[bin]]\nname = \"safe-chains\"\npath = \"src/main.rs\"\n")),
            ("unparsed", "[package\nname = \"tool\"\n".to_string()),
        ] {
            assert!(!in_manifest(label, &manifest, "cargo run -- mcp"), "{label} must count as this project");
        }
        for (label, manifest) in [
            ("mention", package("description = \"a wrapper around safe-chains\"\n# safe-chains\n")),
            ("near", package("[dependencies]\nsafe-chains-extra = \"1\"\n")),
        ] {
            assert!(in_manifest(label, &manifest, "cargo run -- mcp"), "{label} is another project");
        }
    }

    #[test]
    fn a_folder_is_placed_after_its_links_are_followed() {
        let root = std::env::temp_dir().join(format!("sc-cargo-run-{}-links", std::process::id()));
        let this = root.join("this");
        let other = root.join("other");
        std::fs::create_dir_all(this.join("src")).expect("this project");
        std::fs::create_dir_all(other.join("sub")).expect("other project");
        std::fs::write(this.join("Cargo.toml"), "[package]\nname = \"safe-chains\"\n").expect("manifest");
        std::fs::write(other.join("Cargo.toml"), "[package]\nname = \"tool\"\n").expect("manifest");
        let link = other.join("link");
        if !link.exists() {
            std::os::unix::fs::symlink(this.join("src"), &link).expect("link");
        }
        let runs_from = |dir: &str| {
            let _cwd = crate::pathctx::enter_cwd(Some(other.join(dir).to_string_lossy().into_owned()));
            is_safe_command("cargo run -- mcp")
        };
        assert!(runs_from("."));
        assert!(runs_from("sub"));
        assert!(!runs_from("link"));
        assert!(!runs_from("link/../src"));
        assert!(!runs_from("missing"));
        let _cwd = crate::pathctx::enter_cwd(Some(other.to_string_lossy().into_owned()));
        assert!(!is_safe_command("cargo run --manifest-path link/../Cargo.toml -- mcp"));
        assert!(!is_safe_command("cargo run --manifest-path missing/Cargo.toml -- mcp"));
    }
}

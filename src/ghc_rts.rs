pub(crate) fn is_runtime_option(token: &str) -> bool {
    token.starts_with("+RTS") || matches!(token, "-RTS" | "--RTS")
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::is_runtime_option;
    use crate::parse::Token;
    use crate::registry::{dispatch_spec, load_toml};

    const SAFE_BASES: &[(&str, &str)] = &[
        ("agda", "agda --version"),
        ("cabal", "cabal list foo"),
        ("fourmolu", "fourmolu a.hs"),
        ("ghc", "ghc --version"),
        ("hadolint", "hadolint Dockerfile"),
        ("hlint", "hlint src"),
        ("ormolu", "ormolu a.hs"),
        ("pandoc", "pandoc README.md"),
        ("shellcheck", "shellcheck script.sh"),
        ("stack", "stack path"),
    ];

    const MARKERS: &[&str] = &["+RTS", "-RTS", "--RTS"];

    fn ghc_rts_command_names() -> Vec<String> {
        fn walk(dir: &std::path::Path, names: &mut Vec<String>) {
            let mut entries: Vec<_> = std::fs::read_dir(dir).expect("commands directory").map(|e| e.expect("entry").path()).collect();
            entries.sort();
            for path in entries {
                if path.is_dir() {
                    walk(&path, names);
                } else if path.extension().is_some_and(|e| e == "toml") {
                    let text = std::fs::read_to_string(&path).expect("readable");
                    let Ok(value) = text.parse::<toml::Table>() else { continue };
                    for command in value.get("command").and_then(toml::Value::as_array).into_iter().flatten() {
                        if command.get("ghc_rts").and_then(toml::Value::as_bool) == Some(true) {
                            names.push(command["name"].as_str().expect("name").to_string());
                        }
                    }
                }
            }
        }
        let mut names = Vec::new();
        walk(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("commands"), &mut names);
        names.sort();
        names
    }

    fn allowed(cmd: &str) -> bool {
        crate::is_safe_command(cmd)
    }

    #[test]
    fn every_runtime_option_spelling_is_exact() {
        for marker in MARKERS {
            assert!(is_runtime_option(marker));
        }
        for plain in ["x+RTS", "+rts", "RTS", "-RTSx", "--RTSx", "+RT", "RTS+", ""] {
            assert!(!is_runtime_option(plain), "{plain}");
        }
    }

    #[test]
    fn the_table_lists_every_ghc_rts_command() {
        let mut listed: Vec<String> = SAFE_BASES.iter().map(|(name, _)| (*name).to_string()).collect();
        listed.sort_unstable();
        assert_eq!(listed, ghc_rts_command_names());
    }

    #[test]
    fn every_listed_base_is_allowed_on_its_own() {
        for (_, base) in SAFE_BASES {
            assert!(allowed(base), "{base}");
        }
    }

    #[test]
    fn runtime_options_deny_in_every_position_for_every_ghc_built_tool() {
        for (_, base) in SAFE_BASES {
            for marker in MARKERS {
                for cmd in [
                    format!("{base} {marker}"),
                    format!("{base} {marker} -tpwned"),
                    format!("{base} '{marker}' -tpwned"),
                    format!("{base} \"{marker}\" -tpwned"),
                    format!("{base} -- {marker} -tpwned"),
                    format!("{base} {marker} -sout -RTS"),
                    format!(
                        "{} {marker} -tpwned {}",
                        base.split(' ').next().unwrap_or_default(),
                        base.split_once(' ').map_or("", |(_, rest)| rest)
                    ),
                ] {
                    assert!(!allowed(&cmd), "{cmd}");
                }
            }
        }
    }

    #[test]
    fn a_marker_after_the_tool_name_denies_before_anything_else_is_read() {
        for (name, _) in SAFE_BASES {
            for marker in MARKERS {
                let cmd = format!("{name} {marker} -tpwned");
                assert!(!allowed(&cmd), "{cmd}");
            }
        }
    }

    #[test]
    fn a_word_that_only_starts_like_a_marker_is_an_ordinary_argument() {
        for (name, base) in SAFE_BASES {
            let plain = format!("{base} RTS+x");
            let other = format!("{base} RTS+y");
            assert_eq!(allowed(&plain), allowed(&other), "{name}");
        }
        for cmd in [
            "hlint RTS+x", "ormolu RTS+x", "fourmolu RTS+x", "stack path RTS+x", "cabal list RTS+x", "hadolint RTS+x", "pandoc RTS+x",
            "shellcheck RTS+x", "hlint src -- RTS+x",
        ] {
            assert!(allowed(cmd), "{cmd}");
        }
    }

    #[test]
    fn a_synthetic_tool_shows_the_flag_is_what_denies() {
        let toml = |flag: &str| {
            format!(
                "[[command]]\nname = \"zzghc\"\n{flag}\nlevel = \"Inert\"\nbare = true\ntolerate_unknown_short = true\ntolerate_unknown_long = true\n"
            )
        };
        let tokens = |words: &[&str]| words.iter().map(|w| Token::from_test(w)).collect::<Vec<_>>();
        let with = load_toml(&toml("ghc_rts = true"), "test").expect("valid");
        let without = load_toml(&toml(""), "test").expect("valid");
        for marker in MARKERS {
            let argv = tokens(&["zzghc", "file", marker, "-tpwned"]);
            assert!(!dispatch_spec(&argv, &with[0]).is_allowed(), "{marker}");
            assert!(dispatch_spec(&argv, &without[0]).is_allowed(), "{marker} with no flag");
        }
        let plain = tokens(&["zzghc", "file", "RTS+x"]);
        assert!(dispatch_spec(&plain, &with[0]).is_allowed());
    }

    proptest! {
        #[test]
        fn a_runtime_option_anywhere_among_the_arguments_denies(
            index in 0usize..10,
            marker in prop::sample::select(MARKERS.to_vec()),
            before in prop::collection::vec("[a-zA-Z.]{1,6}", 0..3),
            after in prop::collection::vec("[a-zA-Z0-9.-]{1,8}", 0..3),
            quoted in any::<bool>(),
        ) {
            let (_, base) = SAFE_BASES[index];
            let marker = if quoted { format!("'{marker}'") } else { marker.to_string() };
            let cmd = format!("{base} {} {marker} {}", before.join(" "), after.join(" "));
            prop_assert!(!allowed(&cmd), "{}", cmd);
        }

    }
}

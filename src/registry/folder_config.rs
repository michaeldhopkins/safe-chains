//! The `[unknown_folder] writes = "…"` setting, read from the user config only
//! (`~/.config/safe-chains.toml`). A project's `.safe-chains.toml` is never consulted: the agent can
//! write that file, and a checkout must not be able to loosen how its own writes are judged.

use std::{env, fs};

use serde::Deserialize;

use crate::pathctx::anchor::FolderLevel;

#[derive(Deserialize)]
struct File {
    #[serde(default)]
    unknown_folder: Option<Section>,
}

#[derive(Deserialize)]
struct Section {
    #[serde(default)]
    writes: Option<String>,
}

/// What the user config says about writes in an unknown folder.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Setting {
    /// No config, or no `writes` in it: the default level applies.
    Unset,
    Level(FolderLevel),
    /// The file or the value could not be read. Treated as `reads`, the strictest level, since a
    /// typo must not be what loosens it.
    Unreadable(String),
}

impl Setting {
    pub fn level(&self) -> FolderLevel {
        match self {
            Setting::Unset => FolderLevel::DEFAULT,
            Setting::Level(l) => *l,
            Setting::Unreadable(_) => FolderLevel::Reads,
        }
    }
}

/// The setting in the user config. `SAFE_CHAINS_NO_LOCAL` switches it off with the rest of the
/// local config.
pub fn user_setting() -> Setting {
    if env::var_os("SAFE_CHAINS_NO_LOCAL").is_some() {
        return Setting::Unset;
    }
    let Some(path) = super::custom::find_user_custom() else {
        return Setting::Unset;
    };
    match fs::read_to_string(&path) {
        Ok(source) => parse(&source),
        Err(e) => Setting::Unreadable(format!("{}: {e}", path.display())),
    }
}

pub(crate) fn parse(source: &str) -> Setting {
    let file: File = match toml::from_str(source) {
        Ok(f) => f,
        Err(e) => return Setting::Unreadable(e.message().to_string()),
    };
    match file.unknown_folder.and_then(|s| s.writes) {
        None => Setting::Unset,
        Some(name) => FolderLevel::parse(&name).map_or_else(
            || Setting::Unreadable(format!("unknown_folder.writes = \"{name}\" is not one of {}", FolderLevel::NAMES.join(", "))),
            Setting::Level,
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_level_reads_back() {
        for name in FolderLevel::NAMES {
            let level = FolderLevel::parse(name).expect("a listed name parses");
            assert_eq!(parse(&format!("[unknown_folder]\nwrites = \"{name}\"\n")), Setting::Level(level));
        }
    }

    #[test]
    fn absent_is_the_default_and_unreadable_is_reads() {
        assert_eq!(parse(""), Setting::Unset);
        assert_eq!(parse("level = \"editor\"\n[[trusted]]\npath = \"/a\"\nsha256 = \"x\"\n"), Setting::Unset);
        assert_eq!(parse("[unknown_folder]\n"), Setting::Unset);
        assert_eq!(Setting::Unset.level(), FolderLevel::Developer);
        for bad in
            ["[unknown_folder]\nwrites = \"devloper\"\n", "[unknown_folder]\nwrites = 3\n", "not toml {{{", "unknown_folder = \"reads\"\n"]
        {
            let got = parse(bad);
            assert!(matches!(got, Setting::Unreadable(_)), "{bad}: {got:?}");
            assert_eq!(got.level(), FolderLevel::Reads, "{bad}");
        }
    }
}

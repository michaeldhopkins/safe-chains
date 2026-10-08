//! What `safe-chains config` refuses to write, whoever asks.
//!
//! A grant or an entry may widen what safe-chains approves. It may not undo a built-in rule: the
//! credential shield, the freeze on safe-chains' own config and the system's login files, a built-in
//! command's grammar, or the refusal to approve an agent that can rewrite its own approvals. Those
//! are the rules a hand edit has broken before, so the commands that edit the file hold them.

use super::EditError;
use crate::engine::facet::LocalLocus;
use crate::engine::resolve::regions::Frozen;
use crate::engine::resolve::regions::hypothetical::ungranted_role;

/// Why a path cannot be granted from a command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Protected {
    Credential,
    TrustFile,
    SystemFile,
    Device,
    RunsLater,
}

impl Protected {
    pub(super) fn explain(self) -> &'static str {
        match self {
            Protected::Credential => {
                "holds credentials, and safe-chains will not grant a credential store from a command. If \
                 you want commands there approved, write that grant into ~/.config/safe-chains.toml \
                 yourself"
            }
            Protected::TrustFile => {
                "holds a file safe-chains or your agent reads its approvals from. A grant there would let \
                 an agent approve itself, so no grant can cover it"
            }
            Protected::SystemFile => "decides who may log in and what they may do, so no grant can cover it",
            Protected::Device => "is a raw device, not a folder, so safe-chains will not grant it from a command",
            Protected::RunsLater => {
                "is, or holds, settings or programs that run later without anyone asking: a coding agent's \
                 settings, a shell or login startup file, or a folder on PATH. A grant there would let an agent \
                 change what runs next, so safe-chains will not make it from a command"
            }
        }
    }
}

/// Commands that run an AI coding agent. Each can rewrite its own permission settings, so an entry
/// approving one would let an agent approve whatever it likes next. Best effort: a new agent is not
/// on it until someone adds it, which is why each harness `src/targets` supports must be (tested).
pub(crate) const AGENT_COMMANDS: &[&str] = &[
    "agy", "aider", "amazon-q", "amp", "antigravity", "claude", "cline", "codex", "continue", "copilot", "crush", "cursor", "cursor-agent",
    "devin", "droid", "gemini", "goose", "gptme", "grok", "kiro", "kiro-cli", "openhands", "opencode", "plandex", "q", "qwen", "roo",
];

pub(super) fn check_entry_name(name: &str) -> Result<(), EditError> {
    if AGENT_COMMANDS.contains(&name) {
        return Err(EditError::RunsAnAgent(name.to_string()));
    }
    Ok(())
}

/// The path as it will be written: absolute, `~/` for anything under the home directory, with no
/// `.`, `..` or trailing `/`. A relative path is taken from the current directory.
pub(super) fn normalize_path(raw: &str) -> Result<String, EditError> {
    let raw = raw.trim();
    let home = std::env::var("HOME").map_err(|_| EditError::NoHome)?;
    let home = home.trim_end_matches('/');
    let absolute = if raw == "~" {
        home.to_string()
    } else if let Some(rest) = raw.strip_prefix("~/") {
        format!("{home}/{rest}")
    } else if raw.starts_with('/') {
        raw.to_string()
    } else if raw.is_empty() || raw.starts_with('~') || raw.starts_with('$') {
        return Err(EditError::NotAbsolute(raw.to_string()));
    } else {
        let cwd = std::env::current_dir().map_err(|e| EditError::Unreadable(e.to_string()))?;
        format!("{}/{raw}", cwd.to_string_lossy())
    };
    let mut parts: Vec<&str> = Vec::new();
    for part in absolute.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            other => parts.push(other),
        }
    }
    let joined = format!("/{}", parts.join("/"));
    Ok(match joined.strip_prefix(home) {
        Some("") => "~".to_string(),
        Some(rest) if rest.starts_with('/') => format!("~{rest}"),
        _ => joined,
    })
}

/// Refuse a grant that a built-in rule forbids, or that is too wide to mean anything.
pub(super) fn check_grant_path(path: &str) -> Result<(), EditError> {
    if path.contains(['*', '?', '[']) {
        return Err(EditError::Glob(path.to_string()));
    }
    let home = std::env::var("HOME").unwrap_or_default();
    if path == "/" || path == "~" || home.starts_with(&format!("{path}/")) {
        return Err(EditError::TooBroad(path.to_string()));
    }
    let role = ungranted_role(path);
    let protected = if role.reads_secret {
        Some(Protected::Credential)
    } else if role.frozen != Frozen::Nothing {
        Some(Protected::TrustFile)
    } else if role.write_locus >= LocalLocus::Device {
        Some(Protected::Device)
    } else if role.write_locus >= LocalLocus::SystemIntegrity {
        Some(Protected::SystemFile)
    } else if runs_later(path) {
        Some(Protected::RunsLater)
    } else {
        None
    };
    match protected {
        Some(why) => Err(EditError::Protected(path.to_string(), why)),
        None => Ok(()),
    }
}

/// Places whose contents run later without anyone being asked: coding agents' settings (an agent
/// that can write its own settings approves itself), shell and login startup, git's config (it
/// can name programs git runs), launchd and autostart, and folders on PATH (a bare command name
/// is trusted wherever PATH finds it). Home entries are relative to the home folder.
const HOME_RUNS_LATER: &[&str] = &[
    ".claude", ".claude.json", ".codex", ".gemini", ".cursor", ".qwen", ".factory", ".copilot", ".grok", ".aider.conf.yml", ".continue",
    ".config/opencode", ".config/goose", ".kiro", ".aws/amazonq", ".zshrc", ".zshenv", ".zprofile", ".zlogin", ".bashrc", ".bash_profile",
    ".bash_login", ".profile", ".config/fish", ".gitconfig", ".config/git", ".npmrc", ".cargo", ".local/bin", "bin", ".config/systemd",
    ".config/autostart", "Library/LaunchAgents",
];
const SYSTEM_RUNS_LATER: &[&str] = &[
    "/Library/LaunchAgents", "/Library/LaunchDaemons", "/usr/local/bin", "/usr/local/sbin", "/opt/homebrew", "/usr/bin", "/bin",
    "/usr/sbin", "/sbin", "/etc/profile", "/etc/zshrc", "/etc/zprofile", "/etc/bashrc", "/etc/paths", "/etc/paths.d", "/etc/cron.d",
    "/etc/crontab", "/etc/systemd", "/etc/xdg/autostart", "/etc/launchd.conf",
];

/// Whether a grant on `path` (normalized, `~/` for home) would reach a place that runs later: it is
/// one, sits inside one, or is a folder above one that a grant would sweep in (a grant does not
/// reach through a dot directory, so `~/` does not sweep in `~/.zshrc`).
fn runs_later(path: &str) -> bool {
    let home = std::env::var("HOME").unwrap_or_default();
    let absolute = |p: &str| match p.strip_prefix('~') {
        Some(rest) => format!("{}{rest}", home.trim_end_matches('/')),
        None => p.to_string(),
    };
    let granted = absolute(path).to_ascii_lowercase();
    let on_path = std::env::var("PATH").unwrap_or_default();
    let places = HOME_RUNS_LATER
        .iter()
        .map(|p| format!("{}/{p}", home.trim_end_matches('/')))
        .chain(SYSTEM_RUNS_LATER.iter().map(|p| (*p).to_string()))
        .chain(on_path.split(':').filter(|p| p.starts_with('/')).map(String::from));
    places.map(|p| p.trim_end_matches('/').to_ascii_lowercase()).filter(|p| !p.is_empty()).any(|place| {
        let inside = granted == place || granted.starts_with(&format!("{place}/"));
        let above = place
            .strip_prefix(&format!("{granted}/"))
            .is_some_and(|below| !below.split('/').any(|c| c.starts_with('.')));
        inside || above
    })
}

/// Refuse an entry whose name or shape would approve more than the command written.
///
/// Found in review: `config add 'Bash -c x'` slipped past the built-in check (names are matched
/// case-sensitively, and macOS finds `bash` as `Bash`); `config add '/tmp/evil build'` recorded
/// `evil`, approving every program of that name wherever it sits; and in `mytool -o out.txt run`
/// the flag hid the subcommand, so the entry approved `mytool` with any words at all.
pub(super) fn check_spelling(command: &str, entry: &crate::suggest::GeneratedEntry) -> Result<(), EditError> {
    let lower = entry.name.to_ascii_lowercase();
    let lowered = command.replacen(&entry.name, &lower, 1);
    let unknown_in_lower_case = match crate::suggest::analyze(&lowered) {
        crate::suggest::Outcome::Generated { entries, .. } => entries.iter().any(|e| e.name == lower),
        _ => false,
    };
    if lower != entry.name && !unknown_in_lower_case || crate::suggest::is_known(&lower) {
        return Err(EditError::BuiltIn(vec![entry.name.clone()]));
    }
    check_entry_name(&lower)?;
    let plain = entry.name.starts_with(|c: char| c.is_ascii_alphanumeric())
        && entry.name.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '+' | '-'));
    if !plain {
        return Err(EditError::OddName(entry.name.clone()));
    }
    let words = shell_words::split(command).unwrap_or_default();
    for (at, word) in words.iter().enumerate() {
        let names_it = word == &entry.name || word.ends_with(&format!("/{}", entry.name));
        if !names_it || at > 0 && !matches!(words[at - 1].as_str(), "|" | "&&" | "||" | ";") {
            continue;
        }
        if word.contains('/') {
            return Err(EditError::PathSpelled(word.clone()));
        }
        let rest = &words[at + 1..];
        let hidden = rest.first().is_some_and(|w| w.starts_with('-'))
            && rest
                .iter()
                .take_while(|w| !matches!(w.as_str(), "|" | "&&" | "||" | ";"))
                .any(|w| crate::suggest::is_sub_word(w));
        if hidden && entry.subs.is_empty() {
            let sub = rest.iter().find(|w| crate::suggest::is_sub_word(w)).cloned().unwrap_or_default();
            return Err(EditError::SubAfterFlags(entry.name.clone(), sub));
        }
    }
    Ok(())
}

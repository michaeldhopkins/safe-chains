pub mod bun;
mod bunx;
mod npx;

use crate::parse::{Token, WordSet};
use crate::verdict::Verdict;

pub(super) static BUNX_FLAGS_NO_ARG: WordSet =
    WordSet::new(&["--bun", "--no-install", "--silent", "--verbose"]);

pub(super) fn find_runner_package_index(
    tokens: &[Token],
    start: usize,
    flags: &WordSet,
) -> Option<usize> {
    let mut i = start;
    while i < tokens.len() {
        if tokens[i] == "--package" || tokens[i] == "-p" {
            i += 2;
            continue;
        }
        if flags.contains(&tokens[i]) {
            i += 1;
            continue;
        }
        if tokens[i] == "--" {
            return Some(i + 1);
        }
        if tokens[i].starts_with("-") {
            return None;
        }
        return Some(i);
    }
    None
}

pub(super) fn runner_dispatch(tokens: &[Token], flags: &WordSet) -> crate::verdict::Verdict {
    use crate::verdict::{SafetyLevel, Verdict};

    if tokens.len() < 2 {
        return Verdict::Denied;
    }
    if tokens.len() == 2 && tokens[1] == "--version" {
        return Verdict::Allowed(SafetyLevel::Inert);
    }
    match find_runner_package_index(tokens, 1, flags) {
        Some(idx) => runner_verdict(tokens, idx),
        None => Verdict::Denied,
    }
}

fn strip_version(pkg: &str) -> &str {
    if let Some(at) = pkg.rfind('@').filter(|&at| at > 0) {
        return &pkg[..at];
    }
    pkg
}

pub(super) fn runner_verdict(tokens: &[Token], pkg_idx: usize) -> crate::verdict::Verdict {
    use crate::verdict::Verdict;

    if pkg_idx >= tokens.len() {
        return Verdict::Denied;
    }
    let pkg = strip_version(tokens[pkg_idx].as_str());
    let args: Vec<&str> = std::iter::once(pkg)
        .chain(tokens[pkg_idx + 1..].iter().map(|t| t.as_str()))
        .collect();
    let inner = shell_words::join(args);
    crate::command_verdict(&inner)
}

pub(crate) fn dispatch(cmd: &str, tokens: &[Token]) -> Option<Verdict> {
    npx::dispatch(cmd, tokens)
        .or_else(|| bunx::dispatch(cmd, tokens))
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    let mut docs = Vec::new();
    docs.extend(bunx::command_docs());
    docs.extend(npx::command_docs());
    docs
}

#[cfg(test)]
pub(super) fn full_registry() -> Vec<&'static super::CommandEntry> {
    let mut v = Vec::new();
    v.extend(npx::REGISTRY);
    v.extend(bunx::REGISTRY);
    v
}

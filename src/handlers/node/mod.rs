pub mod bun;
mod bunx;
mod npx;

use crate::parse::{Token, WordSet};
use crate::verdict::Verdict;

pub(super) static BUNX_FLAGS_NO_ARG: WordSet = WordSet::new(&["--bun", "--no-install", "--silent", "--verbose"]);

pub(super) fn find_runner_package_index(tokens: &[Token], start: usize, flags: &WordSet) -> Option<usize> {
    let mut specs: Vec<&str> = Vec::new();
    let mut i = start;
    while i < tokens.len() {
        if tokens[i] == "--package" || tokens[i] == "-p" {
            let spec = tokens.get(i + 1)?.as_str();
            package_name(spec)?;
            specs.push(spec);
            i += 2;
            continue;
        }
        if flags.contains(&tokens[i]) {
            i += 1;
            continue;
        }
        if tokens[i] == "--" {
            // An empty tail still yields the index after `--`. A spec is checked only when one
            // was given, against the command that follows.
            if !specs.is_empty() {
                let command = tokens.get(i + 1)?.as_str();
                if !specs_match(&specs, command) {
                    return None;
                }
            }
            return Some(i + 1);
        }
        if tokens[i].starts_with("-") {
            return None;
        }
        if !specs.is_empty() && !specs_match(&specs, tokens[i].as_str()) {
            return None;
        }
        return Some(i);
    }
    None
}

/// A `--package` spec is the package the command runs. Its name, after a version, has to be that
/// command. A version that carries a path or a URL is a different package.
fn specs_match(specs: &[&str], command: &str) -> bool {
    match package_name(command) {
        Some(command) => specs.iter().all(|spec| package_name(spec) == Some(command)),
        None => false,
    }
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

/// The package a runner will execute, without a version suffix.
///
/// A leading `@scope/name` keeps its slash: that is the package, not a version. The version is the
/// text after the last `@` once a name has started. `None` when that version, or an unscoped name,
/// contains `/`, `:`, or `..`, or when the spec is empty.
fn package_name(pkg: &str) -> Option<&str> {
    if pkg.is_empty() {
        return None;
    }
    if let Some(rest) = pkg.strip_prefix('@') {
        let (body, version) = split_version(rest);
        let (scope, name) = body.split_once('/')?;
        if !plain_segment(scope) || !plain_segment(name) || body.matches('/').count() != 1 {
            return None;
        }
        if let Some(version) = version {
            if !safe_version(version) {
                return None;
            }
            return Some(&pkg[..pkg.len() - version.len() - 1]);
        }
        return Some(pkg);
    }
    let (name, version) = split_version(pkg);
    if !plain_segment(name) {
        return None;
    }
    if let Some(version) = version
        && !safe_version(version)
    {
        return None;
    }
    Some(name)
}

fn split_version(pkg: &str) -> (&str, Option<&str>) {
    match pkg.rfind('@') {
        Some(at) if at > 0 => (&pkg[..at], Some(&pkg[at + 1..])),
        _ => (pkg, None),
    }
}

fn plain_segment(segment: &str) -> bool {
    !segment.is_empty() && !segment.contains(['/', ':', '@']) && !segment.contains("..") && !segment.starts_with('-')
}

fn safe_version(version: &str) -> bool {
    !version.is_empty() && !version.contains(['/', ':']) && !version.contains("..")
}

pub(super) fn runner_verdict(tokens: &[Token], pkg_idx: usize) -> crate::verdict::Verdict {
    use crate::verdict::Verdict;

    if pkg_idx >= tokens.len() {
        return Verdict::Denied;
    }
    let Some(pkg) = package_name(tokens[pkg_idx].as_str()) else {
        return Verdict::Denied;
    };
    let args: Vec<&str> = std::iter::once(pkg).chain(tokens[pkg_idx + 1..].iter().map(|t| t.as_str())).collect();
    let inner = shell_words::join(args);
    crate::command_verdict(&inner)
}

pub(crate) fn dispatch(cmd: &str, tokens: &[Token]) -> Option<Verdict> {
    npx::dispatch(cmd, tokens).or_else(|| bunx::dispatch(cmd, tokens))
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    let mut docs = Vec::new();
    docs.extend(bunx::command_docs());
    docs.extend(npx::command_docs());
    docs
}

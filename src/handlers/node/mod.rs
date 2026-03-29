mod bun;
mod bunx;
mod npm;
mod npx;
mod yarn;

use crate::parse::{Token, WordSet, has_flag};
use crate::verdict::Verdict;
use crate::policy::{self, FlagPolicy, FlagStyle};

pub(crate) use bun::BUN;
pub(crate) use npm::NPM;

pub(super) static NPX_SAFE: WordSet =
    WordSet::new(&["@herb-tools/linter", "eslint", "karma"]);

pub(super) static BUNX_FLAGS_NO_ARG: WordSet =
    WordSet::new(&["--bun", "--no-install", "--silent", "--verbose"]);

pub(super) static TSC_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--allowJs", "--checkJs", "--esModuleInterop",
        "--forceConsistentCasingInFileNames", "--help", "--incremental",
        "--isolatedModules", "--noEmit", "--noFallthroughCasesInSwitch",
        "--noImplicitAny", "--noImplicitReturns", "--noUnusedLocals",
        "--noUnusedParameters", "--pretty", "--resolveJsonModule",
        "--skipLibCheck", "--strict", "--strictNullChecks",
        "-h",
    ]),
    valued: WordSet::flags(&[
        "--baseUrl", "--jsx", "--lib", "--module",
        "--moduleResolution", "--project",
        "--rootDir", "--target",
    ]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

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

pub(super) fn is_safe_runner_package(tokens: &[Token], pkg_idx: usize) -> bool {
    if pkg_idx >= tokens.len() {
        return false;
    }
    if NPX_SAFE.contains(&tokens[pkg_idx]) {
        return true;
    }
    if tokens[pkg_idx] == "tsc" {
        return has_flag(&tokens[pkg_idx..], None, Some("--noEmit"))
            && policy::check(&tokens[pkg_idx..], &TSC_POLICY);
    }
    false
}

pub(crate) fn dispatch(cmd: &str, tokens: &[Token]) -> Option<Verdict> {
    NPM.dispatch(cmd, tokens)
        .or_else(|| yarn::dispatch(cmd, tokens))
        .or_else(|| BUN.dispatch(cmd, tokens))
        .or_else(|| npx::dispatch(cmd, tokens))
        .or_else(|| bunx::dispatch(cmd, tokens))
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    let mut docs = Vec::new();
    docs.push(NPM.to_doc());
    docs.extend(yarn::command_docs());
    docs.push(BUN.to_doc());
    docs.extend(bunx::command_docs());
    docs.extend(npx::command_docs());
    docs
}

#[cfg(test)]
pub(super) fn full_registry() -> Vec<&'static super::CommandEntry> {
    let mut v = Vec::new();
    v.extend(yarn::REGISTRY);
    v.extend(npx::REGISTRY);
    v.extend(bunx::REGISTRY);
    v
}

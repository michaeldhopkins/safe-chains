mod bun;
mod bunx;
mod deno;
mod fnm;
mod npm;
mod npx;
mod nvm;
mod pnpm;
mod volta;
mod yarn;

use crate::parse::{Segment, Token, WordSet, has_flag};
use crate::policy::{self, FlagPolicy, FlagStyle};

pub(crate) use bun::BUN;
pub(crate) use deno::DENO;
pub(crate) use fnm::FNM;
pub(crate) use npm::NPM;
pub(crate) use nvm::NVM;
pub(crate) use pnpm::PNPM;
pub(crate) use volta::VOLTA;

pub(super) static NPX_SAFE: WordSet =
    WordSet::new(&["@herb-tools/linter", "eslint", "karma"]);

pub(super) static BUNX_FLAGS_NO_ARG: WordSet =
    WordSet::new(&["--bun", "--no-install", "--silent", "--verbose"]);

pub(super) static TSC_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--allowJs", "--checkJs", "--esModuleInterop",
        "--forceConsistentCasingInFileNames", "--incremental",
        "--isolatedModules", "--noEmit", "--noFallthroughCasesInSwitch",
        "--noImplicitAny", "--noImplicitReturns", "--noUnusedLocals",
        "--noUnusedParameters", "--pretty", "--resolveJsonModule",
        "--skipLibCheck", "--strict", "--strictNullChecks",
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

pub(crate) fn dispatch(cmd: &str, tokens: &[Token], is_safe: &dyn Fn(&Segment) -> bool) -> Option<bool> {
    NPM.dispatch(cmd, tokens, is_safe)
        .or_else(|| yarn::dispatch(cmd, tokens, is_safe))
        .or_else(|| PNPM.dispatch(cmd, tokens, is_safe))
        .or_else(|| BUN.dispatch(cmd, tokens, is_safe))
        .or_else(|| DENO.dispatch(cmd, tokens, is_safe))
        .or_else(|| npx::dispatch(cmd, tokens, is_safe))
        .or_else(|| bunx::dispatch(cmd, tokens, is_safe))
        .or_else(|| NVM.dispatch(cmd, tokens, is_safe))
        .or_else(|| FNM.dispatch(cmd, tokens, is_safe))
        .or_else(|| VOLTA.dispatch(cmd, tokens, is_safe))
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    let mut docs = Vec::new();
    docs.push(NPM.to_doc());
    docs.extend(yarn::command_docs());
    docs.push(PNPM.to_doc());
    docs.push(BUN.to_doc());
    docs.extend(bunx::command_docs());
    docs.push(DENO.to_doc());
    docs.extend(npx::command_docs());
    docs.push(NVM.to_doc());
    docs.push(FNM.to_doc());
    docs.push(VOLTA.to_doc());
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

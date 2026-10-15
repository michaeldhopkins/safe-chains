use crate::engine::facet::LocalLocus;
use crate::engine::resolve::locus::{is_unpinnable, read_locus, write_locus};
use crate::verdict::{SafetyLevel, Verdict};

pub(super) fn config_dir_verdict(value: &str) -> Verdict {
    if in_scratch(value) || !in_a_normal_folder(value) { Verdict::Denied } else { Verdict::Allowed(SafetyLevel::Inert) }
}

pub(super) fn config_file_verdict(value: &str) -> Verdict {
    if in_scratch(value) {
        return Verdict::Denied;
    }
    crate::engine::resolve::execute_file_verdict(value)
}

fn in_a_normal_folder(value: &str) -> bool {
    let Some(abs) = crate::pathctx::join_cwd(None, value) else {
        return false;
    };
    if std::env::var("HOME")
        .ok()
        .filter(|h| h.starts_with('/') && h.len() > 1)
        .is_some_and(|h| abs == h || abs.starts_with(&format!("{}/", h.trim_end_matches('/'))))
    {
        return true;
    }
    [abs.clone(), format!("{}/x", abs.trim_end_matches('/'))]
        .iter()
        .all(|p| matches!(read_locus(p), LocalLocus::Worktree | LocalLocus::WorktreeTrusted | LocalLocus::User))
}

const TEMP_ROOTS: &[&str] = &["/tmp/", "/private/tmp/", "/var/tmp/", "/private/var/tmp/", "/var/folders/", "/private/var/folders/"];

pub(super) fn in_scratch(value: &str) -> bool {
    if !value.starts_with('/') && value != "~" && !value.starts_with("~/") {
        return true;
    }
    let Some(abs) = crate::pathctx::join_cwd(None, value) else {
        return true;
    };
    let dir = format!("{}/", abs.trim_end_matches('/'));
    if is_unpinnable(&abs) || crate::pathctx::under_temp_root(&dir) || under_temp_root_any_case(&dir) {
        return true;
    }
    let mut spellings = vec![abs.clone()];
    spellings.extend(spelled_as_the_projects_parent(&abs));
    spellings
        .iter()
        .flat_map(|p| [p.clone(), format!("{}/x", p.trim_end_matches('/'))])
        .any(|p| matches!(write_locus(&p), LocalLocus::Temp | LocalLocus::SandboxScope | LocalLocus::Adjacent))
}

fn under_temp_root_any_case(dir: &str) -> bool {
    let lower = dir.to_ascii_lowercase();
    let tmpdir = std::env::var("TMPDIR")
        .ok()
        .map(|t| t.trim_end_matches('/').to_ascii_lowercase())
        .filter(|t| t.len() > 1);
    TEMP_ROOTS.iter().any(|r| lower.starts_with(r)) || tmpdir.is_some_and(|t| t.starts_with('/') && lower.starts_with(&format!("{t}/")))
}

fn spelled_as_the_projects_parent(abs: &str) -> Option<String> {
    let root = crate::pathctx::join_cwd(None, &crate::pathctx::root()?)?;
    let (parent, _) = root.trim_end_matches('/').rsplit_once('/')?;
    let head = abs.get(..parent.len())?;
    let rest = abs.get(parent.len()..)?;
    (head.eq_ignore_ascii_case(parent) && (rest.is_empty() || rest.starts_with('/'))).then(|| format!("{parent}{rest}"))
}

#[cfg(test)]
#[path = "config_dir_tests.rs"]
mod tests;

//! What a `$(…)` stage is known to print, and what that makes of the whole substitution.

use crate::engine::facet::LocalLocus;

pub(super) enum StageOutput {
    Locus(LocalLocus),
    /// The stage prints the folder it runs in, which is known: its path, and that path's rung.
    Here(String, LocalLocus),
    /// Every word of this stage's stdout is separator-free, so no word can BE a path.
    Atom,
    /// This stage only filters; ask the stage before it.
    PassThrough,
}

/// What a `$(…)` is known to yield. Two different kinds of claim, which is why this is not an
/// `Option<LocalLocus>`: a locus says the value NAMES something at a rung, an atom says the value
/// names nothing at all and cannot traverse. The second is the weaker claim and the more useful
/// one — it is what lets a literal prefix survive around an interpolated leaf.
pub(crate) enum SubClaim {
    Locus(LocalLocus),
    /// The value is this exact path: `$(pwd)` in a folder on disk.
    Literal(String),
    Atom,
}

/// A working directory that the shell would hand on as one plain word: absolute, a real place rather
/// than the unknown folder or a placeholder, and free of anything that splits, globs or expands.
fn plain_folder_word(dir: &str) -> bool {
    dir.starts_with('/')
        && !crate::pathctx::folder::is_unknown(dir)
        && !dir.contains("__SAFE_CHAINS_")
        && !dir.chars().any(|c| c.is_whitespace() || c.is_control() || "*?[]{}$`\\'\"~!#&;|<>()".contains(c))
}

/// The stage prints the folder it runs in. That folder is a worktree path only while it is one: at
/// `user` or above the claim names a directory, not a file, and a suffix such as `/shadow` would
/// never be shown to the shield. Same bound as `Operands`.
pub(super) fn of_cwd(locus: LocalLocus) -> Option<StageOutput> {
    if locus >= LocalLocus::User {
        return None;
    }
    Some(
        crate::pathctx::cwd()
            .filter(|dir| plain_folder_word(dir))
            .map_or(StageOutput::Locus(locus), |dir| StageOutput::Here(dir, locus)),
    )
}

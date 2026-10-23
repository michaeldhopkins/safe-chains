//! Whether `*` matches names that start with a dot, for the line being judged.

thread_local! {
    static DOT_NAMES: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// While held, `*` also matches names that start with a dot. A line that turns on bash's `dotglob`,
/// sets `GLOBIGNORE` (which turns `dotglob` on too) or turns on zsh's `GLOB_DOTS` anywhere is judged
/// so throughout: the words are matched without reading the shell, so `shopt -s dotglob; rm -rf *`
/// reaches `.git`. Entering never turns it off for a line an outer guard already marked.
pub(crate) struct DotNamesGuard(bool);

impl Drop for DotNamesGuard {
    fn drop(&mut self) {
        DOT_NAMES.with(|cell| cell.set(self.0));
    }
}

pub(crate) fn enter_dot_names(line: &str) -> DotNamesGuard {
    let folded: String = line.chars().filter(|c| *c != '_').flat_map(char::to_lowercase).collect();
    let on = ["dotglob", "globdots", "globignore"].iter().any(|option| folded.contains(option));
    DOT_NAMES.with(|cell| DotNamesGuard(cell.replace(cell.get() || on)))
}

pub(crate) fn dot_names_matched() -> bool {
    DOT_NAMES.with(std::cell::Cell::get)
}

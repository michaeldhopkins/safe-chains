//! Placing a path argument, once: joined onto the working directory, followed through links on disk,
//! and written relative to the project root when it lands inside it. [`place`] is the only
//! constructor of [`Placed`], so a judge that takes `&Placed` cannot be handed text nobody placed.
//!
//! A placed path keeps two readings: where the kernel takes it (`physical`) and how it was spelled
//! (`spelled`, joined lexically). A name rule is asked of both and the worse answer stands, so a link
//! can only add a refusal for the name it was given, never remove one for the place it leads.

use std::cell::{Cell, RefCell};

use super::home::home_path_inside_root;
use super::lexical::{expand_home, express_relative_to_root, lexical_join};
use super::{CURRENT, anchor, folder};

pub(super) mod physical;

thread_local! {
    static ON_THE_LINK: Cell<bool> = const { Cell::new(false) };
    static HERE: RefCell<Option<Here>> = const { RefCell::new(None) };
}

/// A path as the kernel will take it and as it was written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Placed(Inner);

#[derive(Clone, Debug, PartialEq, Eq)]
enum Inner {
    Real { physical: String, spelled: String },
    Unknown { text: String },
}

impl Placed {
    fn real(physical: String, spelled: String) -> Placed {
        Placed(Inner::Real { physical, spelled })
    }

    fn unknown(text: &str) -> Placed {
        Placed(Inner::Unknown { text: text.to_string() })
    }

    /// The place to judge: root-relative inside the project, absolute outside it. For a path that
    /// names no place (`$VAR`) it is the text itself, which every judge refuses to pin.
    pub(crate) fn judged(&self) -> &str {
        match &self.0 {
            Inner::Real { physical, .. } => physical,
            Inner::Unknown { text } => text,
        }
    }

    /// The path as it was written, joined onto the working directory without following a link.
    pub(crate) fn spelled(&self) -> &str {
        match &self.0 {
            Inner::Real { spelled, .. } => spelled,
            Inner::Unknown { text } => text,
        }
    }

    pub(crate) fn into_judged(self) -> String {
        match self.0 {
            Inner::Real { physical, .. } => physical,
            Inner::Unknown { text } => text,
        }
    }
}

/// The folder the harness reported, placed once at hook entry in both forms: `logical` as the shell
/// spells it (`cd` joins on it, as `cd -L` does) and `physical` as the kernel takes it (`git -C`,
/// `tmux -c` and `find -execdir` call `chdir(2)`, and so do file operands).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Here {
    pub(crate) logical: String,
    pub(crate) physical: String,
}

pub(super) fn note_here(reported: Option<&str>) {
    let here = reported
        .filter(|c| {
            c.starts_with('/') && !folder::is_unknown(c) && *c != super::UNRESOLVED_CWD && !c.contains('$') && !c.contains("__SAFE_CHAINS_")
        })
        .map(|logical| Here {
            physical: physical::resolve(logical, true).unwrap_or_else(|| lexical_join("/", logical)),
            logical: logical.to_string(),
        });
    HERE.with(|h| *h.borrow_mut() = here);
}

pub(super) fn swap_here(here: Option<Here>) -> Option<Here> {
    HERE.with(|h| h.replace(here))
}

pub(crate) fn here() -> Option<Here> {
    HERE.with(|h| h.borrow().clone())
}

/// The folder a command in the unknown folder runs in on disk: the reported one with any `cd` below
/// the unknown folder applied.
pub(super) fn here_below(cwd: &str) -> Option<String> {
    let below = cwd.strip_prefix(crate::targets::UNKNOWN_WORKDIR)?;
    if !(below.is_empty() || below.starts_with('/')) {
        return None;
    }
    Some(format!("{}{below}", here()?.logical.trim_end_matches('/')))
}

/// Runs `judge` with the last component of every path kept, not followed: the command acts on a link
/// named there (`cp -R link x`, `chmod -h`), not on what it leads to. A path ending in `/` or `/.`
/// is still followed, as the kernel follows it.
pub(crate) fn on_the_link<T>(judge: impl FnOnce() -> T) -> T {
    struct Restore(bool);
    impl Drop for Restore {
        fn drop(&mut self) {
            ON_THE_LINK.set(self.0);
        }
    }
    let _restore = Restore(ON_THE_LINK.replace(true));
    judge()
}

/// Where a relative `path` read in the unknown folder lands when it passes through a link in the
/// reported folder, or `None` when it meets no link there.
///
/// The unknown folder names no place on disk, so a file committed as a link to `~/.ssh/id_rsa` would
/// otherwise be judged as a file of the folder. The command still runs in the reported folder, where
/// the kernel follows the link, so the path is judged where it lands, as it is in a known
/// workspace. Only reads come here: the real place is never judged more leniently than a file of the
/// unknown folder for a read, and writes there are judged by name.
fn through_a_link_in_here(cwd: &str, path: &str, follow_last: bool) -> Option<String> {
    let folder = here_below(cwd)?;
    physical::resolve(&format!("{}/{path}", folder.trim_end_matches('/')), follow_last)
}

/// Place `path` for `use_` against the ambient folder and project root. The result is judged as
/// [`Placed::judged`] says; with no context at all it is the path as written.
pub(crate) fn place(path: &str, use_: Option<anchor::Use>) -> Placed {
    if path.contains('$') {
        return Placed::unknown(path);
    }
    if path.is_empty() {
        return Placed::real(String::new(), String::new());
    }
    let follow_last = use_ != Some(anchor::Use::Rebind) && !ON_THE_LINK.get();
    let root = CURRENT.with(|c| c.borrow().root.clone()).filter(|r| r.starts_with('/') && !folder::is_unknown(r));
    if path.starts_with('~') {
        let spelled = home_path_inside_root(path).unwrap_or_else(|| path.to_string());
        let real = expand_home(path)
            .and_then(|abs| physical::resolve(&abs, follow_last))
            .map(|real| {
                root.as_deref()
                    .map_or_else(|| real.clone(), |root| express_relative_to_root(&real, &physical::root(root)))
            })
            .filter(|placed| placed.starts_with('/'));
        return Placed::real(real.unwrap_or_else(|| spelled.clone()), spelled);
    }
    CURRENT.with(|c| {
        let ctx = c.borrow();
        match (ctx.cwd.as_deref(), ctx.root.as_deref()) {
            (Some(cwd), Some(root)) if cwd.starts_with('/') && root.starts_with('/') => {
                if !path.starts_with('/')
                    && !use_.is_some_and(anchor::Use::mutates)
                    && let Some(real) = through_a_link_in_here(cwd, path, follow_last)
                {
                    return Placed::real(real, path.to_string());
                }
                if folder::is_unknown(root)
                    && !path.starts_with('/')
                    && !use_.is_some_and(anchor::Use::mutates)
                    && !super::reported_is_secret()
                    && let Some(placed) = folder::place(cwd, path, use_)
                {
                    return Placed::real(placed.clone(), placed);
                }
                let raw = if path.starts_with('/') {
                    path.to_string()
                } else {
                    folder::place(cwd, path, use_).map_or_else(|| format!("{cwd}/{path}"), |placed| format!("{root}/{placed}"))
                };
                let lexical = express_relative_to_root(&lexical_join("/", &raw), root);
                match physical::resolve(&raw, follow_last) {
                    Some(real) => Placed::real(express_relative_to_root(&real, &physical::root(root)), lexical),
                    None => Placed::real(lexical.clone(), lexical),
                }
            }
            _ if path.starts_with('/') => {
                Placed::real(physical::resolve(path, follow_last).unwrap_or_else(|| path.to_string()), path.to_string())
            }
            _ => Placed::real(path.to_string(), path.to_string()),
        }
    })
}

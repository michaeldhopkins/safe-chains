//! Listing one directory of a walk, telling an unreadable folder from a missing path or a file.

pub(super) enum Opened {
    Entries(std::fs::ReadDir),
    Unlisted,
    Absent,
}

/// [`Opened::Unlisted`] when a directory cannot be listed. [`Opened::Absent`] for a missing path or a file.
pub(super) fn open_dir(path: &str) -> Opened {
    match std::fs::read_dir(path) {
        Ok(entries) => Opened::Entries(entries),
        Err(err) if dir_unreadable(&err, path) => Opened::Unlisted,
        Err(_) => Opened::Absent,
    }
}

fn dir_unreadable(err: &std::io::Error, path: &str) -> bool {
    !matches!(err.kind(), std::io::ErrorKind::NotFound | std::io::ErrorKind::NotADirectory)
        && match std::fs::metadata(path) {
            Ok(meta) => meta.is_dir(),
            Err(stat_err) => !matches!(stat_err.kind(), std::io::ErrorKind::NotFound | std::io::ErrorKind::NotADirectory),
        }
}

pub(super) fn same_place(walked: &str, listed: &std::path::Path) -> bool {
    listed.to_str() == Some(walked)
}

pub(super) fn link_dir(listed: &std::path::Path) -> Option<String> {
    let target = std::fs::read_link(listed).ok()?;
    let joined = if target.is_absolute() { target } else { listed.parent()?.join(target) };
    joined.is_dir().then(|| joined.to_str().map(str::to_string)).flatten()
}

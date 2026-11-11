//! Whether ack's own configuration turns on `--follow`. Unless told `--noenv`, ack reads options
//! from `/etc/ackrc`, the user's `$ACKRC` or `~/.ackrc`, a project `.ackrc` found from the working
//! directory upward, and `ACK_OPTIONS`, all before its command line.

use std::path::{Path, PathBuf};

pub(super) fn follows() -> bool {
    let env = |name: &str| std::env::var_os(name).filter(|value| !value.is_empty());
    if env("ACK_OPTIONS").is_some_and(|options| options.to_string_lossy().split_whitespace().any(says_follow)) {
        return true;
    }
    let mut files = vec![PathBuf::from("/etc/ackrc")];
    files.extend(env("ACKRC").map(PathBuf::from));
    files.extend(env("HOME").map(|home| Path::new(&home).join(".ackrc")));
    let Some(cwd) = crate::pathctx::cwd() else { return true };
    files.extend(Path::new(&cwd).ancestors().flat_map(|dir| [dir.join(".ackrc"), dir.join("_ackrc")]));
    files.iter().any(|file| file_follows(file))
}

/// A file that exists but cannot be read may say anything, so it counts as following.
fn file_follows(file: &Path) -> bool {
    match std::fs::read_to_string(file) {
        Ok(text) => text.lines().map(str::trim).any(says_follow),
        Err(error) => error.kind() != std::io::ErrorKind::NotFound,
    }
}

/// ack takes any unambiguous abbreviation of a long option, so `--fo` and `--foll` are `--follow`.
fn says_follow(word: &str) -> bool {
    let name = word.strip_prefix("--").map(|body| body.split('=').next().unwrap_or(body));
    name.is_some_and(|name| name.len() >= 2 && "follow".starts_with(name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_follow_option_in_a_readable_file_follows() {
        let dir = tempfile::tempdir().expect("scratch");
        let write = |name: &str, text: &str| {
            let path = dir.path().join(name);
            std::fs::write(&path, text).expect("rc");
            path
        };
        assert!(file_follows(&write("long", "--follow\n")));
        assert!(file_follows(&write("abbreviated", "# c\n  --fol\n")));
        assert!(!file_follows(&write("plain", "--smart-case\n--nofollow\n--fx\n")));
        assert!(!file_follows(&dir.path().join("missing")));
        assert!(file_follows(dir.path()));
        assert!(!says_follow("-f") && !says_follow("--f") && !says_follow("--follows"));
    }
}

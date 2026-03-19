use crate::parse::{Token, WordSet};

static TAR_DANGEROUS_LONG: WordSet = WordSet::new(&[
    "--append", "--concatenate", "--create", "--delete",
    "--extract", "--get", "--update",
]);

const TAR_DANGEROUS_SHORT: &[u8] = b"Acrux";

const TAR_SAFE_SHORT: &[u8] = b"JfjtvzO";

static TAR_SAFE_LONG: WordSet = WordSet::new(&[
    "--bzip2", "--file", "--gzip", "--list", "--verbose", "--xz", "--zstd",
]);

fn is_old_style_flags(s: &str) -> bool {
    !s.starts_with('-') && !s.is_empty() && s.bytes().all(|b| b.is_ascii_alphabetic())
}

fn has_list_mode(tokens: &[Token]) -> bool {
    for (idx, t) in tokens[1..].iter().enumerate() {
        if *t == "--list" {
            return true;
        }
        let s = t.as_str();
        if s.starts_with('-') && !s.starts_with("--")
            && s.bytes().skip(1).any(|b| b == b't')
        {
            return true;
        }
        if idx == 0 && is_old_style_flags(s) && s.contains('t') {
            return true;
        }
    }
    false
}

fn has_dangerous_char(s: &str) -> bool {
    s.bytes().skip(1).any(|b| TAR_DANGEROUS_SHORT.contains(&b))
}

fn all_chars_safe(s: &str) -> bool {
    s.bytes().skip(1).all(|b| TAR_SAFE_SHORT.contains(&b) || b == b't')
}

fn check_short_bundle(s: &str) -> Option<usize> {
    if has_dangerous_char(s) {
        return None;
    }
    if !all_chars_safe(s) {
        if s.contains('f') {
            return Some(2);
        }
        return None;
    }
    if s.contains('f') { Some(2) } else { Some(1) }
}

fn is_safe_tar(tokens: &[Token]) -> bool {
    if !has_list_mode(tokens) {
        return false;
    }
    let mut i = 1;
    while i < tokens.len() {
        let t = &tokens[i];
        let s = t.as_str();
        if TAR_DANGEROUS_LONG.contains(t) {
            return false;
        }
        if TAR_SAFE_LONG.contains(t) {
            i += 1;
            continue;
        }
        if s == "--file" || s == "-f" {
            i += 2;
            continue;
        }
        if s.starts_with('-') && !s.starts_with("--") && s.len() > 1 {
            match check_short_bundle(s) {
                Some(advance) => { i += advance; continue; }
                None => return false,
            }
        }
        if i == 1 && is_old_style_flags(s) {
            let dashed = format!("-{s}");
            match check_short_bundle(&dashed) {
                Some(advance) => { i += advance; continue; }
                None => return false,
            }
        }
        if s.starts_with("--") {
            return false;
        }
        i += 1;
    }
    true
}

pub(in crate::handlers::coreutils) fn dispatch(cmd: &str, tokens: &[Token]) -> Option<bool> {
    match cmd {
        "tar" => Some(is_safe_tar(tokens)),
        _ => None,
    }
}

pub(in crate::handlers::coreutils) fn command_docs() -> Vec<crate::docs::CommandDoc> {
    vec![
        crate::docs::CommandDoc::handler("tar",
            "https://man7.org/linux/man-pages/man1/tar.1.html",
            "Listing mode only (requires -t or --list). Old-style flags accepted (e.g. tar tf, tar tzf).\n\
             Flags: -f, -j, -J, -v, -z, -O, --bzip2, --file, --gzip, --xz, --zstd."),
    ]
}

#[cfg(test)]
pub(in crate::handlers::coreutils) const REGISTRY: &[crate::handlers::CommandEntry] = &[
    crate::handlers::CommandEntry::Custom { cmd: "tar", valid_prefix: Some("tar -tf archive.tar") },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        tar_list: "tar -tf archive.tar",
        tar_list_verbose: "tar -tvf archive.tar",
        tar_list_gz: "tar -tzf archive.tar.gz",
        tar_list_long: "tar --list --file archive.tar",
        tar_list_bz2: "tar -tjf archive.tar.bz2",
        tar_list_xz: "tar -tJf archive.tar.xz",
        tar_list_separate: "tar -t -f archive.tar",
        tar_list_v_separate: "tar -t -v -f archive.tar",
        tar_old_style_tz: "tar tz",
        tar_old_style_tf: "tar tf archive.tar",
        tar_old_style_tvf: "tar tvf archive.tar",
        tar_old_style_tzf: "tar tzf archive.tar.gz",
        tar_old_style_tjf: "tar tjf archive.tar.bz2",
    }

    denied! {
        tar_create: "tar -cf archive.tar files/",
        tar_extract: "tar -xf archive.tar",
        tar_append: "tar -rf archive.tar newfile",
        tar_update: "tar -uf archive.tar newfile",
        tar_bare: "tar",
        tar_no_list: "tar -f archive.tar",
        tar_bundled_extract: "tar -txf archive.tar",
        tar_bundled_create: "tar -tcf archive.tar",
        tar_old_style_xf: "tar xf archive.tar",
        tar_old_style_cf: "tar cf archive.tar files/",
    }
}

use crate::parse::{Token, WordSet};
use crate::verdict::{SafetyLevel, Verdict};

static GZIP_SAFE_MODES: WordSet = WordSet::new(&["--list", "--test", "-l", "-t"]);

static GZIP_SAFE_FLAGS: WordSet = WordSet::new(&[
    "--list", "--test", "--verbose",
    "-l", "-t", "-v",
]);

fn is_safe_gzip(tokens: &[Token]) -> Verdict {
    let mut has_mode = false;
    for t in &tokens[1..] {
        if GZIP_SAFE_MODES.contains(t) {
            has_mode = true;
        }
    }
    if !has_mode {
        return Verdict::Denied;
    }
    let mut i = 1;
    while i < tokens.len() {
        let t = &tokens[i];
        if GZIP_SAFE_FLAGS.contains(t) {
            i += 1;
            continue;
        }
        if !t.as_str().starts_with('-') {
            i += 1;
            continue;
        }
        return Verdict::Denied;
    }
    Verdict::Allowed(SafetyLevel::Inert)

}

pub(in crate::handlers::coreutils) fn dispatch(cmd: &str, tokens: &[Token]) -> Option<Verdict> {
    match cmd {
        "gzip" => Some(is_safe_gzip(tokens)),
        _ => None,
    }
}

pub(in crate::handlers::coreutils) fn command_docs() -> Vec<crate::docs::CommandDoc> {
    vec![
        crate::docs::CommandDoc::handler("gzip",
            "https://man7.org/linux/man-pages/man1/gzip.1.html",
            "Info/test modes only (requires -l/--list or -t/--test).\n\
             Flags: -v/--verbose."),
    ]
}

#[cfg(test)]
pub(in crate::handlers::coreutils) const REGISTRY: &[crate::handlers::CommandEntry] = &[
    crate::handlers::CommandEntry::Custom { cmd: "gzip", valid_prefix: Some("gzip -l") },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        gzip_list: "gzip -l file.gz",
        gzip_list_long: "gzip --list file.gz",
        gzip_test: "gzip -t file.gz",
        gzip_test_long: "gzip --test file.gz",
        gzip_list_verbose: "gzip -l -v file.gz",
    }

    denied! {
        gzip_compress: "gzip file.txt",
        gzip_bare: "gzip",
        gzip_decompress: "gzip -d file.gz",
        gzip_force: "gzip -f file.txt",
    }
}

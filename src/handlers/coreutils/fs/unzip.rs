use crate::parse::{Token, WordSet};
use crate::verdict::{SafetyLevel, Verdict};

static UNZIP_SAFE_MODES: WordSet = WordSet::new(&["-Z", "-l", "-t"]);

static UNZIP_SAFE_FLAGS: WordSet = WordSet::new(&[
    "-1", "-2", "-C", "-M", "-T", "-Z",
    "-h", "-l", "-m", "-q", "-s", "-t", "-v", "-z",
]);

fn is_safe_unzip(tokens: &[Token]) -> Verdict {
    if tokens.len() == 2 && (tokens[1] == "--help" || tokens[1] == "-h" || tokens[1] == "--version" || tokens[1] == "-V") {
        return Verdict::Allowed(SafetyLevel::Inert);
    }
    let mut has_mode = false;
    for t in &tokens[1..] {
        if UNZIP_SAFE_MODES.contains(t) {
            has_mode = true;
        }
    }
    if !has_mode {
        return Verdict::Denied;
    }
    let mut i = 1;
    while i < tokens.len() {
        let t = &tokens[i];
        if UNZIP_SAFE_FLAGS.contains(t) {
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
        "unzip" => Some(is_safe_unzip(tokens)),
        _ => None,
    }
}

pub(in crate::handlers::coreutils) fn command_docs() -> Vec<crate::docs::CommandDoc> {
    vec![
        crate::docs::CommandDoc::handler("unzip",
            "https://linux.die.net/man/1/unzip",
            "List/test modes only (requires -l, -t, or -Z).\n\
             Flags: -1, -2, -C, -M, -T, -h, -m, -q, -s, -v, -z."),
    ]
}

#[cfg(test)]
pub(in crate::handlers::coreutils) const REGISTRY: &[crate::handlers::CommandEntry] = &[
    crate::handlers::CommandEntry::Custom { cmd: "unzip", valid_prefix: Some("unzip -l") },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        unzip_list: "unzip -l archive.zip",
        unzip_test: "unzip -t archive.zip",
        unzip_zipinfo: "unzip -Z archive.zip",
        unzip_list_quiet: "unzip -l -q archive.zip",
    }

    denied! {
        unzip_extract: "unzip archive.zip",
        unzip_bare: "unzip",
        unzip_dest: "unzip -d /tmp archive.zip",
        unzip_verbose_only: "unzip -v archive.zip",
    }
}

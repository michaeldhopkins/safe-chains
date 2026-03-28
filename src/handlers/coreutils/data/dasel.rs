use crate::parse::{Token, WordSet};
use crate::verdict::{SafetyLevel, Verdict};
use crate::policy::{self, FlagPolicy, FlagStyle};

static DASEL_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--colour", "--compact", "--help", "--length",
        "--multiple", "--no-colour", "--null", "--plain",
        "--version",
        "-c", "-h", "-m", "-n", "-v",
    ]),
    valued: WordSet::flags(&[
        "--file", "--parser", "--read", "--selector", "--write",
        "-f", "-p", "-r", "-s", "-w",
    ]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static MUTATING_SUBS: WordSet = WordSet::new(&["delete", "put"]);

pub fn is_safe_dasel(tokens: &[Token]) -> Verdict {
    if tokens.len() == 1 {
        return Verdict::Allowed(SafetyLevel::Inert);
    }
    if tokens.len() == 2 && matches!(tokens[1].as_str(), "--help" | "-h" | "--version" | "-v") {
        return Verdict::Allowed(SafetyLevel::Inert);
    }
    for t in &tokens[1..] {
        if !t.starts_with("-") && MUTATING_SUBS.contains(t) {
            return Verdict::Denied;
        }
    }
    if policy::check(tokens, &DASEL_POLICY) { Verdict::Allowed(SafetyLevel::Inert) } else { Verdict::Denied }
}

pub(super) fn dispatch(cmd: &str, tokens: &[Token]) -> Option<Verdict> {
    match cmd {
        "dasel" => Some(is_safe_dasel(tokens)),
        _ => None,
    }
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    vec![
        crate::docs::CommandDoc::handler("dasel",
            "https://github.com/TomWright/dasel",
            "Read-only queries allowed. select subcommand allowed."),
    ]
}

#[cfg(test)]
pub(in crate::handlers::coreutils) const REGISTRY: &[crate::handlers::CommandEntry] = &[
    crate::handlers::CommandEntry::Custom { cmd: "dasel", valid_prefix: None },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        dasel_bare: "dasel",
        dasel_help: "dasel --help",
        dasel_select: "dasel select -f data.json '.name'",
        dasel_query: "dasel -f data.json '.name'",
        dasel_file_parser: "dasel -f data.yaml -p yaml '.key'",
    }

    denied! {
        dasel_put_denied: "dasel put -f data.json -t string -v hello '.name'",
        dasel_delete_denied: "dasel delete -f data.json '.name'",
    }
}

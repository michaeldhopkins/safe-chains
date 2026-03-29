use crate::parse::{Token, WordSet};
use crate::policy::{self, FlagPolicy, FlagStyle};
use crate::verdict::{SafetyLevel, Verdict};

static SK_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--ansi", "--border", "--cycle", "--exact", "--exit-0",
        "--help", "--inline-info", "--keep-right", "--layout",
        "--multi", "--no-bold", "--no-clear", "--no-hscroll", "--no-mouse",
        "--no-sort", "--print-query", "--print-score", "--print0",
        "--read0", "--regex", "--reverse", "--select-1",
        "--tac", "--version",
        "-0", "-1", "-V", "-e", "-h", "-m",
    ]),
    valued: WordSet::flags(&[
        "--algo", "--border-label", "--color", "--delimiter",
        "--expect", "--header", "--header-lines", "--height",
        "--history", "--history-size", "--margin", "--min-height",
        "--nth", "--output-format", "--padding",
        "--pre-select-file", "--pre-select-items", "--pre-select-n",
        "--prompt", "--query", "--scrollbar", "--skip-to-pattern",
        "--tabstop", "--tiebreak", "--with-nth",
        "-d", "-n", "-p", "-q",
    ]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static HISTORY_FLAGS: WordSet = WordSet::flags(&["--history"]);

fn has_history(tokens: &[Token]) -> bool {
    tokens[1..].iter().any(|t| {
        HISTORY_FLAGS.contains(t)
            || t.as_str().starts_with("--history=")
    })
}

pub fn is_safe_sk(tokens: &[Token]) -> Verdict {
    if !policy::check(tokens, &SK_POLICY) {
        return Verdict::Denied;
    }
    if tokens.len() > 1 && has_history(tokens) {
        Verdict::Allowed(SafetyLevel::SafeWrite)
    } else {
        Verdict::Allowed(SafetyLevel::Inert)
    }
}

pub(super) fn dispatch(cmd: &str, tokens: &[Token]) -> Option<Verdict> {
    match cmd {
        "sk" => Some(is_safe_sk(tokens)),
        _ => None,
    }
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    vec![
        crate::docs::CommandDoc::handler(
            "sk",
            "https://github.com/lotabout/skim",
            "Fuzzy finder. Display, filtering, and layout flags allowed. --history allowed (SafeWrite).",
        ),
    ]
}

#[cfg(test)]
pub(super) const REGISTRY: &[crate::handlers::CommandEntry] = &[
    crate::handlers::CommandEntry::Custom { cmd: "sk", valid_prefix: None },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    use crate::command_verdict;
    use crate::verdict::{SafetyLevel, Verdict};
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        sk_bare: "sk",
        sk_help: "sk --help",
        sk_version: "sk --version",
        sk_multi: "sk -m",
        sk_exact: "sk --exact",
        sk_query: "sk -q pattern",
        sk_height: "sk --height 40%",
        sk_ansi: "sk --ansi",
        sk_prompt: "sk --prompt '> '",
        sk_header: "sk --header 'Pick one'",
        sk_color: "sk --color=dark",
        sk_delimiter: "sk -d : -n 2",
        sk_select_1: "sk --select-1 --exit-0",
        sk_history: "sk --history /tmp/sk-history",
        sk_history_eq: "sk --history=/tmp/sk-history",
    }

    safe_write! {
        sk_history_level: "sk --history /tmp/sk-history",
    }

    denied! {
        sk_preview: "sk --preview 'cat {}'",
        sk_cmd: "sk -c 'find .'",
        sk_interactive: "sk -i -c 'grep {}'",
        sk_bind: "sk --bind 'ctrl-j:accept'",
    }

    #[test]
    fn sk_bare_is_inert() {
        assert_eq!(command_verdict("sk"), Verdict::Allowed(SafetyLevel::Inert));
    }
}

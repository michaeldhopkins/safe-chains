use crate::command::FlatDef;
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static HTMLQ_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--detect-base", "--help", "--ignore-whitespace",
        "--pretty", "--remove-nodes", "--text", "--version",
        "-B", "-V", "-h", "-p", "-t", "-w",
    ]),
    valued: WordSet::flags(&[
        "--attribute", "--base", "--filename", "--output",
        "-a", "-b", "-f", "-o",
    ]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "htmlq", policy: &HTMLQ_POLICY, level: SafetyLevel::Inert, url: "https://github.com/mgdm/htmlq", aliases: &[] },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        htmlq_selector: "htmlq 'div.container'",
        htmlq_text: "htmlq --text 'h1'",
        htmlq_attr: "htmlq -a href 'a.link'",
        htmlq_file: "htmlq -f page.html 'title'",
        htmlq_pretty: "htmlq -p 'body'",
    }

    denied! {
        htmlq_bare_denied: "htmlq",
    }
}

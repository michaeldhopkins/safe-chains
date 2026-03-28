use crate::parse::Token;
use crate::verdict::{SafetyLevel, Verdict};
use crate::parse::WordSet;
use crate::policy::{self, FlagPolicy, FlagStyle};

static MLR_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--csv", "--csvlite", "--dkvp", "--from",
        "--headerless-csv-output", "--help",
        "--implicit-csv-header", "--json", "--jsonl",
        "--markdown", "--nidx", "--no-auto",
        "--pprint", "--ragged-or-trim", "--tsv",
        "--version", "--xtab",
        "-h",
    ]),
    valued: WordSet::flags(&[
        "--ifs", "--ips", "--irs", "--nr-progress-mod",
        "--ofs", "--ops", "--ors", "--prepipe", "--seed",
    ]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Positional,
};

fn is_safe_mlr(tokens: &[Token]) -> Verdict {
    for t in &tokens[1..] {
        if t == "-I" || t == "--in-place" {
            return Verdict::Denied;
        }
    }
    if policy::check(tokens, &MLR_POLICY) { Verdict::Allowed(SafetyLevel::Inert) } else { Verdict::Denied }
}

pub(super) fn dispatch(cmd: &str, tokens: &[Token]) -> Option<Verdict> {
    match cmd {
        "mlr" => Some(is_safe_mlr(tokens)),
        _ => None,
    }
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    vec![
        crate::docs::CommandDoc::handler("mlr",
            "https://miller.readthedocs.io/",
            "Data processing allowed. Verbs and file arguments accepted."),
    ]
}

#[cfg(test)]
pub(in crate::handlers::coreutils) const REGISTRY: &[crate::handlers::CommandEntry] = &[
    crate::handlers::CommandEntry::Positional { cmd: "mlr" },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        mlr_csv_head: "mlr --csv head -n 10 data.csv",
        mlr_json_filter: "mlr --json filter '$age > 30' data.json",
        mlr_tsv_cut: "mlr --tsv cut -f name,age data.tsv",
    }

    denied! {
        mlr_bare_denied: "mlr",
        mlr_inplace_denied: "mlr -I --csv head -n 10 data.csv",
        mlr_inplace_long_denied: "mlr --in-place --csv head -n 10 data.csv",
    }
}

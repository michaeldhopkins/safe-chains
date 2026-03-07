use crate::command::FlatDef;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static YQ_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--colors", "--exit-status",
        "--no-colors", "--no-doc", "--null-input",
        "--prettyPrint",
        "-C", "-M", "-N", "-P", "-e", "-r",
    ]),
    valued: WordSet::flags(&[
        "--arg", "--argjson", "--expression",
        "--front-matter", "--indent", "--input-format",
        "--output-format",
        "-I", "-p",
    ]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "yq", policy: &YQ_POLICY, help_eligible: true, url: "https://mikefarah.gitbook.io/yq" },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        yq_read: "yq '.key' file.yaml",
        yq_eval: "yq eval '.metadata.name' deployment.yaml",
    }

    denied! {
        yq_inplace_denied: "yq -i '.key = \"value\"' file.yaml",
        yq_inplace_long_denied: "yq --inplace '.key = \"value\"' file.yaml",
    }
}

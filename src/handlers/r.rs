use crate::parse::{Segment, Token, WordSet};
use crate::policy::{self, FlagPolicy, FlagStyle};

static R_CMD_CHECK_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--as-cran", "--no-build-vignettes", "--no-examples",
        "--no-manual", "--no-tests", "--no-vignettes",
    ]),
    standalone_short: b"",
    valued: WordSet::flags(&["--output"]),
    valued_short: b"o",
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static R_CMD_CONFIG_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[]),
    standalone_short: b"",
    valued: WordSet::flags(&[]),
    valued_short: b"",
    bare: false,
    max_positional: Some(1),
    flag_style: FlagStyle::Strict,
};

fn is_safe_r(tokens: &[Token]) -> bool {
    if tokens.len() < 3 {
        return false;
    }
    if tokens[1] != "CMD" {
        return false;
    }
    match tokens[2].as_str() {
        "check" => policy::check(&tokens[2..], &R_CMD_CHECK_POLICY),
        "config" => policy::check(&tokens[2..], &R_CMD_CONFIG_POLICY),
        _ => false,
    }
}

fn is_safe_rscript(tokens: &[Token]) -> bool {
    tokens.len() == 2 && matches!(tokens[1].as_str(), "--help" | "-h" | "--version" | "-V")
}

pub(crate) fn dispatch(cmd: &str, tokens: &[Token], _is_safe: &dyn Fn(&Segment) -> bool) -> Option<bool> {
    match cmd {
        "R" => Some(is_safe_r(tokens)),
        "Rscript" => Some(is_safe_rscript(tokens)),
        _ => None,
    }
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    use crate::docs::CommandDoc;
    vec![
        CommandDoc::handler("R",
            "https://cran.r-project.org/manuals.html",
            "- CMD check <package> (with --as-cran, --no-tests, --no-examples, --no-vignettes, --no-build-vignettes, --no-manual, --output)\n- CMD config <var>"),
        CommandDoc::handler("Rscript",
            "https://cran.r-project.org/manuals.html",
            "- --version\n- --help"),
    ]
}

#[cfg(test)]
pub(crate) const REGISTRY: &[super::CommandEntry] = &[
    super::CommandEntry::Custom { cmd: "R", valid_prefix: Some("R CMD check pkg") },
    super::CommandEntry::Custom { cmd: "Rscript", valid_prefix: Some("Rscript --version") },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        r_cmd_check: "R CMD check mypackage",
        r_cmd_check_as_cran: "R CMD check --as-cran mypackage",
        r_cmd_check_no_tests: "R CMD check --no-tests mypackage",
        r_cmd_check_no_vignettes: "R CMD check --no-vignettes mypackage",
        r_cmd_check_no_manual: "R CMD check --no-manual mypackage",
        r_cmd_check_output: "R CMD check --output /tmp mypackage",
        r_cmd_config: "R CMD config CC",
        r_cmd_config_cflags: "R CMD config CFLAGS",
        rscript_version: "Rscript --version",
        rscript_help: "Rscript --help",
    }

    denied! {
        r_bare_denied: "R",
        r_cmd_install_denied: "R CMD INSTALL mypackage",
        r_cmd_remove_denied: "R CMD REMOVE mypackage",
        r_cmd_build_denied: "R CMD build mypackage",
        r_cmd_batch_denied: "R CMD BATCH script.R",
        r_no_save_denied: "R --no-save -e 'system(\"rm -rf /\")'",
        r_e_denied: "R -e 'print(1)'",
        rscript_e_denied: "Rscript -e 'print(1)'",
        rscript_file_denied: "Rscript script.R",
        rscript_bare_denied: "Rscript",
        r_cmd_unknown_denied: "R CMD xyzzy",
        r_cmd_config_bare_denied: "R CMD config",
        r_cmd_check_bare_denied: "R CMD check",
    }
}

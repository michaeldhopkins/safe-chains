use crate::command::{CommandDef, SubDef};
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static TERRAFORM_SHOW_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "--json", "--no-color", "-h"]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static TERRAFORM_STATE_LIST_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "-h"]),
    valued: WordSet::flags(&["--id", "--state"]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static TERRAFORM_STATE_SHOW_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "-h"]),
    valued: WordSet::flags(&["--state"]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static TERRAFORM_OUTPUT_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "--json", "--no-color", "--raw", "-h"]),
    valued: WordSet::flags(&["--state"]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static TERRAFORM_VALIDATE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "--json", "--no-color", "-h"]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static TERRAFORM_GRAPH_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--draw-cycles", "--help", "-h"]),
    valued: WordSet::flags(&["--plan", "--type"]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static TERRAFORM_BARE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "-h"]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static TERRAFORM_VERSION_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "--json", "-h"]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static TERRAFORM_FMT_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--check", "--diff", "--help", "--no-color", "--recursive", "-h"]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(crate) static TERRAFORM: CommandDef = CommandDef {
    name: "terraform",
    subs: &[
        SubDef::Guarded {
            name: "fmt",
            guard_short: None,
            guard_long: "--check",
            policy: &TERRAFORM_FMT_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "graph", policy: &TERRAFORM_GRAPH_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "output", policy: &TERRAFORM_OUTPUT_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "providers", policy: &TERRAFORM_BARE_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "show", policy: &TERRAFORM_SHOW_POLICY, level: SafetyLevel::Inert },
        SubDef::Nested { name: "state", subs: &[
            SubDef::Policy { name: "list", policy: &TERRAFORM_STATE_LIST_POLICY, level: SafetyLevel::Inert },
            SubDef::Policy { name: "show", policy: &TERRAFORM_STATE_SHOW_POLICY, level: SafetyLevel::Inert },
        ]},
        SubDef::Policy { name: "validate", policy: &TERRAFORM_VALIDATE_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "version", policy: &TERRAFORM_VERSION_POLICY, level: SafetyLevel::Inert },
    ],
    bare_flags: &["--help", "--version", "-V", "-h"],
    url: "https://developer.hashicorp.com/terraform/cli/commands",
    aliases: &[],
};

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        terraform_show: "terraform show",
        terraform_show_json: "terraform show --json",
        terraform_state_list: "terraform state list",
        terraform_state_show: "terraform state show aws_instance.web",
        terraform_output: "terraform output",
        terraform_output_json: "terraform output --json",
        terraform_output_name: "terraform output my_output",
        terraform_validate: "terraform validate",
        terraform_validate_json: "terraform validate --json",
        terraform_graph: "terraform graph",
        terraform_graph_cycles: "terraform graph --draw-cycles",
        terraform_providers: "terraform providers",
        terraform_version: "terraform version",
        terraform_version_json: "terraform version --json",
        terraform_fmt_check: "terraform fmt --check",
        terraform_fmt_check_diff: "terraform fmt --check --diff",
        terraform_fmt_check_recursive: "terraform fmt --check --recursive",
        terraform_help: "terraform --help",
    }

    denied! {
        terraform_apply: "terraform apply",
        terraform_destroy: "terraform destroy",
        terraform_init: "terraform init",
        terraform_plan: "terraform plan",
        terraform_fmt_no_check: "terraform fmt",
        terraform_bare: "terraform",
    }
}

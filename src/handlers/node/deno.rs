use crate::command::{CommandDef, SubDef};
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static DENO_SAFE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--help", "--json", "--no-lock", "--quiet", "--unstable",
        "-h", "-q",
    ]),
    valued: WordSet::flags(&["--config", "--import-map", "-c"]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static DENO_FMT_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--check", "--help", "--no-semicolons", "--single-quote",
        "--unstable",
        "-h", "-q",
    ]),
    valued: WordSet::flags(&[
        "--config", "--ext", "--ignore", "--indent-width",
        "--line-width", "--log-level", "--prose-wrap",
        "-c",
    ]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(crate) static DENO: CommandDef = CommandDef {
    name: "deno",
    subs: &[
        SubDef::Policy { name: "check", policy: &DENO_SAFE_POLICY, level: SafetyLevel::SafeRead },
        SubDef::Policy { name: "doc", policy: &DENO_SAFE_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "info", policy: &DENO_SAFE_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "lint", policy: &DENO_SAFE_POLICY, level: SafetyLevel::SafeRead },
        SubDef::Policy { name: "test", policy: &DENO_SAFE_POLICY, level: SafetyLevel::SafeRead },
        SubDef::Guarded { name: "fmt", guard_short: None, guard_long: "--check", policy: &DENO_FMT_POLICY, level: SafetyLevel::Inert },
    ],
    bare_flags: &["--help", "--version", "-V", "-h"],
    url: "https://docs.deno.com/runtime/reference/cli/",
    aliases: &[],
};

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        deno_version: "deno --version",
        deno_info: "deno info",
        deno_info_json: "deno info --json",
        deno_doc: "deno doc mod.ts",
        deno_lint: "deno lint",
        deno_check: "deno check main.ts",
        deno_test: "deno test",
        deno_test_quiet: "deno test --quiet",
        deno_fmt_check: "deno fmt --check",
    }

    denied! {
        deno_fmt_denied: "deno fmt",
    }
}

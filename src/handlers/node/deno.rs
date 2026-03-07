use crate::command::{CommandDef, SubDef};
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static DENO_SAFE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--json", "--no-lock", "--quiet", "--unstable",
    ]),
    standalone_short: b"q",
    valued: WordSet::flags(&["--config", "--import-map"]),
    valued_short: b"c",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static DENO_FMT_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--check", "--no-semicolons", "--single-quote",
        "--unstable",
    ]),
    standalone_short: b"q",
    valued: WordSet::flags(&[
        "--config", "--ext", "--ignore", "--indent-width",
        "--line-width", "--log-level", "--prose-wrap",
    ]),
    valued_short: b"c",
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(crate) static DENO: CommandDef = CommandDef {
    name: "deno",
    subs: &[
        SubDef::Policy { name: "check", policy: &DENO_SAFE_POLICY },
        SubDef::Policy { name: "doc", policy: &DENO_SAFE_POLICY },
        SubDef::Policy { name: "info", policy: &DENO_SAFE_POLICY },
        SubDef::Policy { name: "lint", policy: &DENO_SAFE_POLICY },
        SubDef::Policy { name: "test", policy: &DENO_SAFE_POLICY },
        SubDef::Guarded { name: "fmt", guard_short: None, guard_long: "--check", policy: &DENO_FMT_POLICY },
    ],
    bare_flags: &[],
    help_eligible: true,
    url: "https://docs.deno.com/runtime/reference/cli/",
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

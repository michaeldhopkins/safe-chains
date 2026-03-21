use crate::command::{CommandDef, SubDef};
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static DUMP_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "--no-values", "-h", "-v"]),
    valued: WordSet::flags(&["--config", "--file"]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static VERSION_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "-h"]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: Some(0),
    flag_style: FlagStyle::Strict,
};

pub(crate) static AAPT2: CommandDef = CommandDef {
    name: "aapt2",
    subs: &[
        SubDef::Nested { name: "dump", subs: &[
            SubDef::Policy { name: "badging", policy: &DUMP_POLICY, level: SafetyLevel::Inert },
            SubDef::Policy { name: "configurations", policy: &DUMP_POLICY, level: SafetyLevel::Inert },
            SubDef::Policy { name: "permissions", policy: &DUMP_POLICY, level: SafetyLevel::Inert },
            SubDef::Policy { name: "resources", policy: &DUMP_POLICY, level: SafetyLevel::Inert },
            SubDef::Policy { name: "strings", policy: &DUMP_POLICY, level: SafetyLevel::Inert },
            SubDef::Policy { name: "styleparents", policy: &DUMP_POLICY, level: SafetyLevel::Inert },
            SubDef::Policy { name: "xmlstrings", policy: &DUMP_POLICY, level: SafetyLevel::Inert },
            SubDef::Policy { name: "xmltree", policy: &DUMP_POLICY, level: SafetyLevel::Inert },
        ]},
        SubDef::Policy { name: "version", policy: &VERSION_POLICY, level: SafetyLevel::Inert },
    ],
    bare_flags: &["--help", "--version", "-V", "-h"],
    url: "https://developer.android.com/tools/aapt2",
    aliases: &[],
};

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        aapt2_dump_badging: "aapt2 dump badging app.apk",
        aapt2_dump_permissions: "aapt2 dump permissions app.apk",
        aapt2_dump_resources: "aapt2 dump resources app.apk",
        aapt2_dump_configurations: "aapt2 dump configurations app.apk",
        aapt2_dump_xmltree: "aapt2 dump xmltree app.apk --file AndroidManifest.xml",
        aapt2_dump_xmlstrings: "aapt2 dump xmlstrings app.apk --file res/layout/main.xml",
        aapt2_dump_strings: "aapt2 dump strings app.apk",
        aapt2_dump_styleparents: "aapt2 dump styleparents app.apk",
        aapt2_dump_verbose: "aapt2 dump resources -v app.apk",
        aapt2_version: "aapt2 version",
        aapt2_help: "aapt2 --help",
    }

    denied! {
        aapt2_bare_denied: "aapt2",
        aapt2_compile_denied: "aapt2 compile -o compiled/ res/values/strings.xml",
        aapt2_link_denied: "aapt2 link -o output.apk compiled/",
        aapt2_optimize_denied: "aapt2 optimize -o optimized.apk app.apk",
        aapt2_dump_bare_denied: "aapt2 dump",
        aapt2_dump_unknown_denied: "aapt2 dump badging --unknown-flag",
    }
}

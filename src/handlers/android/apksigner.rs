use crate::command::{CommandDef, SubDef};
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static VERIFY_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--print-certs", "--verbose", "-v"]),
    valued: WordSet::flags(&["--in", "--max-sdk-version", "--min-sdk-version"]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static BARE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: Some(0),
    flag_style: FlagStyle::Strict,
};

pub(crate) static APKSIGNER: CommandDef = CommandDef {
    name: "apksigner",
    subs: &[
        SubDef::Policy { name: "help", policy: &BARE_POLICY },
        SubDef::Policy { name: "verify", policy: &VERIFY_POLICY },
        SubDef::Policy { name: "version", policy: &BARE_POLICY },
    ],
    bare_flags: &[],
    help_eligible: true,
    url: "https://developer.android.com/tools/apksigner",
};

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        apksigner_verify: "apksigner verify app.apk",
        apksigner_verify_verbose: "apksigner verify --verbose app.apk",
        apksigner_verify_v: "apksigner verify -v app.apk",
        apksigner_verify_print_certs: "apksigner verify --print-certs app.apk",
        apksigner_verify_min_sdk: "apksigner verify --min-sdk-version 21 app.apk",
        apksigner_version: "apksigner version",
        apksigner_help_sub: "apksigner help",
        apksigner_help: "apksigner --help",
    }

    denied! {
        apksigner_bare_denied: "apksigner",
        apksigner_sign_denied: "apksigner sign --ks debug.keystore app.apk",
        apksigner_rotate_denied: "apksigner rotate --out lineage",
        apksigner_verify_unknown_denied: "apksigner verify --unknown app.apk",
    }
}

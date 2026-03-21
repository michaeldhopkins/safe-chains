use crate::command::{CommandDef, SubDef};
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static KEYTOOL_LIST_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "-h", "-rfc", "-v"]),
    valued: WordSet::flags(&["-alias", "-keystore", "-storepass", "-storetype"]),
    bare: true,
    max_positional: Some(0),
    flag_style: FlagStyle::Strict,
};

static KEYTOOL_PRINTCERT_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "-h", "-rfc", "-v"]),
    valued: WordSet::flags(&["-file", "-jarfile"]),
    bare: true,
    max_positional: Some(0),
    flag_style: FlagStyle::Strict,
};

pub(crate) static KEYTOOL: CommandDef = CommandDef {
    name: "keytool",
    subs: &[
        SubDef::Policy { name: "-list", policy: &KEYTOOL_LIST_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "-printcert", policy: &KEYTOOL_PRINTCERT_POLICY, level: SafetyLevel::Inert },
    ],
    bare_flags: &["--help", "--version", "-V", "-h"],
    url: "https://docs.oracle.com/en/java/javase/21/docs/specs/man/keytool.html",
    aliases: &[],
};

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        keytool_list: "keytool -list",
        keytool_list_keystore: "keytool -list -keystore debug.keystore",
        keytool_list_v: "keytool -list -v -keystore debug.keystore",
        keytool_list_rfc: "keytool -list -rfc -keystore debug.keystore",
        keytool_list_alias: "keytool -list -alias mykey -keystore debug.keystore",
        keytool_printcert: "keytool -printcert -file cert.pem",
        keytool_printcert_v: "keytool -printcert -v -file cert.pem",
        keytool_printcert_jarfile: "keytool -printcert -jarfile app.jar",
        keytool_help: "keytool --help",
        keytool_version: "keytool --version",
    }

    denied! {
        keytool_bare_denied: "keytool",
        keytool_genkey_denied: "keytool -genkey",
        keytool_genkeypair_denied: "keytool -genkeypair",
        keytool_delete_denied: "keytool -delete -alias mykey",
        keytool_import_denied: "keytool -importcert -file cert.pem",
        keytool_export_denied: "keytool -exportcert -alias mykey",
        keytool_storepasswd_denied: "keytool -storepasswd",
        keytool_list_unknown_denied: "keytool -list --unknown-flag",
    }
}

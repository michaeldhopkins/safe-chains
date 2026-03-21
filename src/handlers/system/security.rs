use crate::command::{CommandDef, SubDef};
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static SECURITY_FIND_CERT_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "-Z", "-a", "-h", "-p"]),
    valued: WordSet::flags(&["-c", "-e"]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static SECURITY_FIND_IDENTITY_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "-h", "-v"]),
    valued: WordSet::flags(&["-p", "-s"]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static SECURITY_FIND_PASSWORD_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "-h"]),
    valued: WordSet::flags(&[
        "-D", "-a", "-c", "-d", "-j", "-l", "-r", "-s",
        "-t",
    ]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static SECURITY_LIST_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "-d", "-h"]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static SECURITY_DUMP_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "-h"]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static SECURITY_VERIFY_CERT_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "-L", "-h", "-l", "-q"]),
    valued: WordSet::flags(&["-c", "-k", "-n", "-p", "-r"]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static SECURITY_SIMPLE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "-h"]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(crate) static SECURITY: CommandDef = CommandDef {
    name: "security",
    subs: &[
        SubDef::Policy { name: "find-certificate", policy: &SECURITY_FIND_CERT_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "find-identity", policy: &SECURITY_FIND_IDENTITY_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "find-generic-password", policy: &SECURITY_FIND_PASSWORD_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "find-internet-password", policy: &SECURITY_FIND_PASSWORD_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "list-keychains", policy: &SECURITY_LIST_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "dump-keychain", policy: &SECURITY_DUMP_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "dump-trust-settings", policy: &SECURITY_DUMP_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "verify-cert", policy: &SECURITY_VERIFY_CERT_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "cms", policy: &SECURITY_SIMPLE_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "show-keychain-info", policy: &SECURITY_SIMPLE_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "smartcard", policy: &SECURITY_SIMPLE_POLICY, level: SafetyLevel::Inert },
    ],
    bare_flags: &["--help", "--version", "-V", "-h"],
    url: "https://ss64.com/mac/security.html",
    aliases: &[],
};

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        security_find_identity: "security find-identity -v -p codesigning",
        security_find_certificate: "security find-certificate -a",
        security_list_keychains: "security list-keychains",
        security_verify_cert: "security verify-cert -c cert.pem",
        security_dump_keychain: "security dump-keychain",
        security_dump_trust: "security dump-trust-settings",
    }

    denied! {
        security_dump_keychain_decrypt_denied: "security dump-keychain -d",
        security_find_password_g_denied: "security find-generic-password -g",
        security_find_password_w_denied: "security find-internet-password -w pass",
    }
}

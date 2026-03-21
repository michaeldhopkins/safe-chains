use crate::command::{CommandDef, SubDef};
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static BARE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "-h"]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: Some(0),
    flag_style: FlagStyle::Strict,
};

static LOG_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "-h"]),
    valued: WordSet::flags(&["--only"]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(crate) static FIREBASE: CommandDef = CommandDef {
    name: "firebase",
    subs: &[
        SubDef::Policy { name: "apps:list", policy: &BARE_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "functions:log", policy: &LOG_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "login:list", policy: &BARE_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "projects:list", policy: &BARE_POLICY, level: SafetyLevel::Inert },
    ],
    bare_flags: &["--help", "--version", "-V", "-h"],
    url: "https://firebase.google.com/docs/cli",
    aliases: &[],
};

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        firebase_projects_list: "firebase projects:list",
        firebase_apps_list: "firebase apps:list",
        firebase_login_list: "firebase login:list",
        firebase_functions_log: "firebase functions:log",
        firebase_functions_log_only: "firebase functions:log --only myFunction",
        firebase_help: "firebase --help",
        firebase_version: "firebase --version",
    }

    denied! {
        firebase_bare_denied: "firebase",
        firebase_deploy_denied: "firebase deploy",
        firebase_init_denied: "firebase init",
        firebase_login_denied: "firebase login",
        firebase_delete_denied: "firebase firestore:delete users/123",
    }
}

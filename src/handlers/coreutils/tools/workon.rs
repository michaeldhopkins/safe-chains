use crate::command::FlatDef;
use crate::verdict::SafetyLevel;

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "workon", policy: &super::super::BARE_ONLY, level: SafetyLevel::Inert, help_eligible: true, url: "https://github.com/michaeldhopkins/workon", aliases: &[] },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        workon_bare: "workon",
        workon_help: "workon --help",
        workon_help_short: "workon -h",
        workon_version: "workon --version",
        workon_version_short: "workon -V",
    }

    denied! {
        workon_project: "workon myproject",
        workon_force_new: "workon -n myproject",
        workon_workspace: "workon -w myproject",
        workon_unknown: "workon --unknown",
    }
}

use crate::command::FlatDef;
use crate::verdict::SafetyLevel;

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "whoami", policy: &super::super::BARE_ONLY, level: SafetyLevel::Inert, url: "https://www.gnu.org/software/coreutils/manual/coreutils.html#whoami-invocation", aliases: &[] },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        whoami_bare: "whoami",
    }
}

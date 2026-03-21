use crate::command::FlatDef;
use crate::verdict::SafetyLevel;

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "false", policy: &super::super::BARE_ONLY, level: SafetyLevel::Inert, url: "https://www.gnu.org/software/coreutils/manual/coreutils.html#false-invocation", aliases: &[] },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        false_bare: "false",
    }

    denied! {
        false_with_args_denied: "false something",
    }
}

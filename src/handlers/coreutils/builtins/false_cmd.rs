use crate::command::FlatDef;

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "false", policy: &super::super::BARE_ONLY, help_eligible: false },
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

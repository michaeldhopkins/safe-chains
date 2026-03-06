use crate::command::FlatDef;

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "branchdiff", policy: &super::super::BARE_ONLY, help_eligible: false, url: "https://github.com/nickolasburr/branchdiff#readme" },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        branchdiff_bare: "branchdiff",
    }
}

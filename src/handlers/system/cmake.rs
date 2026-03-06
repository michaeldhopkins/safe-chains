use crate::command::CommandDef;

pub(crate) static CMAKE: CommandDef = CommandDef {
    name: "cmake",
    subs: &[],
    bare_flags: &["--system-information", "--version"],
    help_eligible: true,
};

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        cmake_version: "cmake --version",
        cmake_system_information: "cmake --system-information",
    }

    denied! {
        cmake_build_denied: "cmake --build .",
        cmake_generate_denied: "cmake .",
    }
}

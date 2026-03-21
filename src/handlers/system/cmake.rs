use crate::command::CommandDef;

pub(crate) static CMAKE: CommandDef = CommandDef {
    name: "cmake",
    subs: &[],
    bare_flags: &["--help", "--system-information", "--version", "-V", "-h"],
    url: "https://cmake.org/cmake/help/latest/manual/cmake.1.html",
    aliases: &[],
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

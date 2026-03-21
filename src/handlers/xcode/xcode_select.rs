use crate::command::CommandDef;

pub(crate) static XCODE_SELECT: CommandDef = CommandDef {
    name: "xcode-select",
    subs: &[],
    bare_flags: &["--help", "--print-path", "--version", "-V", "-h", "-p", "-v"],
    url: "https://ss64.com/mac/xcode-select.html",
    aliases: &[],
};

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        xcode_select_print_path: "xcode-select -p",
        xcode_select_print_path_long: "xcode-select --print-path",
        xcode_select_version: "xcode-select -v",
    }

    denied! {
        xcode_select_switch_denied: "xcode-select -s /Applications/Xcode.app",
        xcode_select_install_denied: "xcode-select --install",
        xcode_select_reset_denied: "xcode-select --reset",
        xcode_select_no_args_denied: "xcode-select",
    }
}

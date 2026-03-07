use crate::command::FlatDef;
use crate::policy::{FlagPolicy, FlagStyle};
use crate::parse::WordSet;

static READ_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["-r", "-s"]),
    standalone_short: b"rs",
    valued: WordSet::flags(&["-a", "-d", "-n", "-p", "-t", "-u"]),
    valued_short: b"adnptu",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Positional,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "read", policy: &READ_POLICY, help_eligible: false, url: "https://pubs.opengroup.org/onlinepubs/9799919799/utilities/read.html" },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        read_bare: "read",
        read_var: "read line",
        read_multiple_vars: "read first second rest",
        read_raw: "read -r line",
        read_prompt: "read -p 'Enter: ' name",
        read_timeout: "read -t 5 input",
        read_delimiter: "read -d '' content",
        read_nchars: "read -n 1 char",
        read_silent: "read -s password",
        read_array: "read -a arr",
        read_in_while: "while read line; do echo $line; done",
    }
}

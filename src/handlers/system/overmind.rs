use crate::command::{CommandDef, SubDef};
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static OVERMIND_BARE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "-h"]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: Some(0),
    flag_style: FlagStyle::Strict,
};

static OVERMIND_CONNECT_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "-h"]),
    valued: WordSet::flags(&[]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static OVERMIND_START_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--daemonize", "--help", "--no-port",
        "-D", "-N", "-h",
    ]),
    valued: WordSet::flags(&[
        "--colors", "--formation", "--port", "--port-step",
        "--procfile", "--root", "--socket", "--title",
        "-T", "-c", "-f", "-l", "-p", "-r", "-s",
    ]),
    bare: true,
    max_positional: Some(0),
    flag_style: FlagStyle::Strict,
};

static OVERMIND_PROCESS_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "-h"]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(crate) static OVERMIND: CommandDef = CommandDef {
    name: "overmind",
    subs: &[
        SubDef::Policy { name: "connect", policy: &OVERMIND_CONNECT_POLICY, level: SafetyLevel::SafeWrite },
        SubDef::Policy { name: "echo", policy: &OVERMIND_BARE_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "kill", policy: &OVERMIND_BARE_POLICY, level: SafetyLevel::SafeWrite },
        SubDef::Policy { name: "quit", policy: &OVERMIND_BARE_POLICY, level: SafetyLevel::SafeWrite },
        SubDef::Policy { name: "restart", policy: &OVERMIND_PROCESS_POLICY, level: SafetyLevel::SafeWrite },
        SubDef::Policy { name: "start", policy: &OVERMIND_START_POLICY, level: SafetyLevel::SafeWrite },
        SubDef::Policy { name: "status", policy: &OVERMIND_BARE_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "stop", policy: &OVERMIND_PROCESS_POLICY, level: SafetyLevel::SafeWrite },
    ],
    bare_flags: &["--help", "--version", "-h", "-v"],
    url: "https://github.com/DarthSim/overmind",
    aliases: &[],
};

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        overmind_help: "overmind --help",
        overmind_version: "overmind --version",
        overmind_echo: "overmind echo",
        overmind_status: "overmind status",
        overmind_start: "overmind start",
        overmind_start_formation: "overmind start -l web,worker",
        overmind_start_port: "overmind start -p 3000",
        overmind_start_daemonize: "overmind start -D",
        overmind_restart: "overmind restart web",
        overmind_restart_all: "overmind restart",
        overmind_stop: "overmind stop web",
        overmind_stop_all: "overmind stop",
        overmind_connect: "overmind connect web",
        overmind_quit: "overmind quit",
        overmind_kill: "overmind kill",
    }

    denied! {
        overmind_bare_denied: "overmind",
        overmind_run_denied: "overmind run web echo hello",
    }

    inert! {
        level_overmind_echo: "overmind echo",
        level_overmind_status: "overmind status",
    }

    safe_write! {
        level_overmind_start: "overmind start",
        level_overmind_restart: "overmind restart web",
        level_overmind_stop: "overmind stop web",
        level_overmind_quit: "overmind quit",
        level_overmind_kill: "overmind kill",
    }
}

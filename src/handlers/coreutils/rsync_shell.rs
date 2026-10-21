use crate::parse::Token;
use crate::verdict::{SafetyLevel, Verdict};

const VALUED_SHORT: &[char] = &['B', 'e', 'f', 'M', 'T'];
const SSH_FLAGS: &[&str] = &["-4", "-6", "-a", "-C", "-k", "-q", "-T", "-v", "-vv", "-vvv", "-x"];
const SSH_OPTIONS: &[&str] = &[
    "batchmode", "compression", "connectionattempts", "connecttimeout", "loglevel", "port", "serveralivecountmax", "serveraliveinterval",
    "stricthostkeychecking", "user",
];

/// Whether the line names a remote shell (`-e`, `--rsh`) that is not ssh or rsh with plain options,
/// or an `--rsync-path` that is not a plain path to rsync. rsync starts the first itself, so `-e sh`
/// with `x:foo` runs `./x` here, and the remote shell runs the second as a command line. A local
/// `--server` is refused too: it is the far side's half of a transfer, reading its options and
/// file list from standard input, so nothing on the line says what it will write.
pub(crate) fn names_a_program_to_run(tokens: &[Token]) -> bool {
    let mut words = tokens.iter().skip(1).map(Token::as_str);
    while let Some(word) = words.next() {
        if word == "--" {
            break;
        }
        if word == "--server" {
            return true;
        }
        let (is_shell, value) = if let Some(long) = word.strip_prefix("--") {
            let (name, glued) = long.split_once('=').map_or((long, None), |(name, value)| (name, Some(value)));
            if name != "rsh" && name != "rsync-path" {
                continue;
            }
            (name == "rsh", if glued.is_some() { glued } else { words.next() })
        } else if let Some(cluster) = word.strip_prefix('-') {
            let Some(at) = cluster.find(VALUED_SHORT) else {
                continue;
            };
            let glued = &cluster[at + 1..];
            if !cluster[at..].starts_with('e') {
                if glued.is_empty() {
                    words.next();
                }
                continue;
            }
            (true, if glued.is_empty() { words.next() } else { Some(glued) })
        } else {
            continue;
        };
        let plain = match value {
            Some(value) if is_shell => remote_shell_verdict(value).is_allowed(),
            Some(value) => rsync_path_is_plain(value),
            None => false,
        };
        if !plain {
            return true;
        }
    }
    false
}

/// ssh or rsh, bare or in a standard bin directory, or a program named by a relative path (which
/// the executor path gate then places), followed only by options that pick the port, user,
/// verbosity and a few connection settings. ssh's `-o ProxyCommand`, `LocalCommand`, `-F` and `-S`
/// start or read something else, so they are not among them; nor is a quote, which rsync parses
/// itself.
pub(crate) fn remote_shell_is_plain(value: &str) -> bool {
    if !value
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c.is_ascii_whitespace() || matches!(c, '-' | '_' | '.' | '=' | '/' | '@'))
    {
        return false;
    }
    let mut words = value.split_ascii_whitespace();
    let Some(program) = words.next() else {
        return false;
    };
    let name = ["/bin/", "/usr/bin/", "/usr/local/bin/", "/opt/homebrew/bin/"]
        .iter()
        .find_map(|dir| program.strip_prefix(dir))
        .unwrap_or(program);
    let in_the_project = program.contains('/') && !program.starts_with('/');
    if !in_the_project && name != "ssh" && name != "rsh" {
        return false;
    }
    while let Some(word) = words.next() {
        if SSH_FLAGS.contains(&word) {
            continue;
        }
        let Some(flag) = word.get(..2) else {
            return false;
        };
        let glued = &word[2..];
        let Some(argument) = (if glued.is_empty() { words.next() } else { Some(glued) }) else {
            return false;
        };
        let fits = match flag {
            "-p" => argument.chars().all(|c| c.is_ascii_digit()),
            "-l" => argument.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.')),
            "-o" => argument.split_once('=').is_some_and(|(key, setting)| {
                SSH_OPTIONS.contains(&key.to_ascii_lowercase().as_str())
                    && !setting.is_empty()
                    && setting.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
            }),
            _ => false,
        };
        if !fits {
            return false;
        }
    }
    true
}

/// The verdict on a remote shell rsync would start: ssh or rsh with plain options is allowed, and a
/// program named by a relative path is judged as the file it runs. Shared by `-e`, `--rsh` and
/// `RSYNC_RSH`, so the flag and the variable cannot disagree.
pub(crate) fn remote_shell_verdict(value: &str) -> Verdict {
    if !remote_shell_is_plain(value) {
        return Verdict::Denied;
    }
    let program = value.split_ascii_whitespace().next().unwrap_or_default();
    if program.contains('/') && !program.starts_with('/') {
        crate::engine::resolve::execute_file_verdict(program)
    } else {
        Verdict::Allowed(SafetyLevel::Inert)
    }
}

/// A path whose last part is rsync, with nothing a shell would read as more than one word.
fn rsync_path_is_plain(value: &str) -> bool {
    value.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '/' | '.' | '_' | '-' | '+'))
        && value.rsplit('/').next() == Some("rsync")
}

#[cfg(test)]
mod tests {
    use super::remote_shell_is_plain;
    use crate::is_safe_command;
    use proptest::prelude::*;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        rsync_shell_ssh: "rsync -e ssh x:foo dst/",
        rsync_shell_long_ssh: "rsync --rsh=ssh x:foo dst/",
        rsync_shell_cluster_ssh: "rsync -avessh x:foo dst/",
        rsync_shell_env_ssh: "RSYNC_RSH=ssh rsync x:foo dst/",
        rsync_path_plain: "rsync --rsync-path=/usr/local/bin/rsync host:src/ dst/",
        rsync_path_bare: "rsync --rsync-path rsync host:src/ dst/",
        rsync_shell_ssh_with_a_port: "rsync -e 'ssh -p 22' x:foo dst/",
        rsync_shell_ssh_with_a_port_locally: "rsync -e 'ssh -p 22' src/ dst/",
        rsync_shell_ssh_in_usr_bin: "rsync -e /usr/bin/ssh x:foo dst/",
        rsync_shell_ssh_in_usr_bin_with_a_user: "rsync -e '/usr/bin/ssh -l x' x:foo dst/",
        rsync_shell_long_ssh_with_a_port: "rsync --rsh='ssh -p 22 -o BatchMode=yes' x:foo dst/",
        rsync_shell_env_ssh_with_a_port: "RSYNC_RSH='ssh -p 22' rsync x:foo dst/",
    }

    denied! {
        rsync_shell_sh: "rsync -e sh x:foo dst/",
        rsync_shell_bash: "rsync -e bash x:foo dst/",
        rsync_shell_make: "rsync -e make x:foo dst/",
        rsync_shell_long_sh: "rsync --rsh=sh x:foo dst/",
        rsync_shell_long_spaced_sh: "rsync --rsh sh x:foo dst/",
        rsync_shell_cluster_sh: "rsync -avesh x:foo dst/",
        rsync_shell_cluster_spaced_sh: "rsync -ave sh x:foo dst/",
        rsync_shell_env_sh: "RSYNC_RSH=sh rsync x:foo dst/",
        rsync_shell_env_make: "env RSYNC_RSH=make rsync x:foo dst/",
        rsync_shell_proxy_command: "rsync -e 'ssh -o ProxyCommand=id' x:foo dst/",
        rsync_shell_remote_sh: "rsync -a -M-esh src/ dst/",
        rsync_path_shell: "rsync --rsync-path='sh -c id' host:src/ dst/",
        rsync_path_spaced_shell: "rsync --rsync-path 'sudo rsync' host:src/ dst/",
        rsync_path_other_program: "rsync --rsync-path=/bin/sh host:src/ dst/",
        rsync_path_metacharacter: "rsync --rsync-path='rsync;id' host:src/ dst/",
        rsync_path_remote_option: "rsync -a -M--rsync-path=sh src/ dst/",
        rsync_local_server: "rsync --server src/ dst/",
        rsync_local_server_sender: "rsync --server --sender -vlogDtpre.iLsfxC . src/",
        rsync_local_server_after_options: "rsync -a --server src/ dst/",
        rsync_shell_ssh_with_a_config_file: "rsync -e 'ssh -p 22 -F ./cfg' x:foo dst/",
        rsync_shell_ssh_in_tmp: "rsync -e /tmp/ssh x:foo dst/",
        rsync_shell_project_program_outside: "rsync -e ../../x/ssh x:foo dst/",
        rsync_shell_project_program_with_options: "rsync -e '/tmp/x/tool -p 22' x:foo dst/",
    }

    #[test]
    fn only_ssh_or_rsh_with_plain_options_is_a_plain_remote_shell() {
        for value in [
            "ssh -o StrictHostKeyChecking=accept-new", "ssh -o User=a-b", "ssh", "rsh", "ssh -p 22", "ssh -p22 -l me", "ssh -4 -q",
            "ssh -o BatchMode=yes", "ssh -oPort=22", "/usr/bin/ssh", "./bin/tool",
        ] {
            assert!(remote_shell_is_plain(value), "{value}");
        }
        for value in [
            "sh", "bash", "make", "just", "", "ssh -o ProxyCommand=id", "ssh -oProxyCommand=id", "ssh -o LocalCommand=id", "ssh -F ./cfg",
            "ssh -S /tmp/s", "ssh 'x'", "ssh \"-p\" 2", "ssh;id", "ssh -p", "/tmp/ssh", "sshd", "ssh -o", "ssh x id",
            "ssh -o PermitLocalCommand=yes", "ssh -o User=a=b", "ssh -o User=a.b", "ssh -o User=a_b", "ssh -o User=a/b", "ssh -o User=a@b",
        ] {
            assert!(!remote_shell_is_plain(value), "{value}");
        }
    }

    proptest! {
        /// A program rsync would start to reach a daemon is a command line in every spelling, so
        /// none of them is approved. The remote shell's spellings agree with `-e`.
        #[test]
        fn a_connect_program_is_refused_in_every_spelling(
            value in prop::sample::select(vec!["id", "cat", "echo", "sh", "make", "ssh", "rsh", "./evil"]),
            spelling in 0usize..3,
        ) {
            let line = match spelling {
                0 => format!("RSYNC_CONNECT_PROG={value} rsync rsync://x/foo dst/"),
                1 => format!("env RSYNC_CONNECT_PROG={value} rsync rsync://x/foo dst/"),
                _ => format!("export RSYNC_CONNECT_PROG={value}; rsync rsync://x/foo dst/"),
            };
            prop_assert!(!is_safe_command(&line), "{}", line);
        }

        /// However RSYNC_RSH is set, it is the same choice as `-e`.
        #[test]
        fn a_remote_shell_variable_agrees_with_its_flag_in_every_spelling(
            value in prop::sample::select(vec!["id", "cat", "echo", "sh", "make", "ssh", "rsh", "./evil", "./bin/tool"]),
            spelling in 0usize..3,
        ) {
            let via_flag = format!("rsync -e {value} ./src/ ./dst/");
            let line = match spelling {
                0 => format!("RSYNC_RSH={value} rsync ./src/ ./dst/"),
                1 => format!("env RSYNC_RSH={value} rsync ./src/ ./dst/"),
                _ => format!("export RSYNC_RSH={value}; rsync ./src/ ./dst/"),
            };
            prop_assert_eq!(is_safe_command(&line), is_safe_command(&via_flag), "{} vs {}", line, via_flag);
        }

        /// However the remote shell is spelled on the line, only ssh or rsh is approved.
        #[test]
        fn a_remote_shell_other_than_ssh_is_never_approved(
            shell in prop::sample::select(vec!["sh", "bash", "zsh", "make", "just", "python3", "x"]),
            spelling in 0usize..7,
        ) {
            let line = match spelling {
                0 => format!("rsync -e {shell} x:foo dst/"),
                1 => format!("rsync -e{shell} x:foo dst/"),
                2 => format!("rsync -ave {shell} x:foo dst/"),
                3 => format!("rsync -ave{shell} x:foo dst/"),
                4 => format!("rsync --rsh={shell} x:foo dst/"),
                5 => format!("rsync --rsh {shell} x:foo dst/"),
                _ => format!("RSYNC_RSH={shell} rsync x:foo dst/"),
            };
            prop_assert!(!is_safe_command(&line), "{}", line);
        }
    }
}

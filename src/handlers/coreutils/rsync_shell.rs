use crate::parse::Token;

const VALUED_SHORT: &[char] = &['B', 'e', 'f', 'M', 'T'];
const SSH_FLAGS: &[&str] = &["-4", "-6", "-a", "-C", "-k", "-q", "-T", "-v", "-vv", "-vvv", "-x"];
const SSH_OPTIONS: &[&str] = &[
    "batchmode", "compression", "connectionattempts", "connecttimeout", "loglevel", "port", "serveralivecountmax", "serveraliveinterval",
    "stricthostkeychecking", "user",
];

/// Whether the line names a remote shell (`-e`, `--rsh`) that is not ssh or rsh with plain options.
/// rsync starts it itself, so `-e sh` with `x:foo` runs `./x` here, and `-e make` reads a Makefile.
pub(crate) fn names_a_program_to_run(tokens: &[Token]) -> bool {
    let mut words = tokens.iter().skip(1).map(Token::as_str);
    while let Some(word) = words.next() {
        if word == "--" {
            break;
        }
        let value = if let Some(long) = word.strip_prefix("--") {
            let (name, glued) = long.split_once('=').map_or((long, None), |(name, value)| (name, Some(value)));
            if name != "rsh" {
                continue;
            }
            if glued.is_some() { glued } else { words.next() }
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
            if glued.is_empty() { words.next() } else { Some(glued) }
        } else {
            continue;
        };
        if !value.is_some_and(remote_shell_is_plain) {
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
    }

    #[test]
    fn only_ssh_or_rsh_with_plain_options_is_a_plain_remote_shell() {
        for value in [
            "ssh", "rsh", "ssh -p 22", "ssh -p22 -l me", "ssh -4 -q", "ssh -o BatchMode=yes", "ssh -oPort=22", "/usr/bin/ssh", "./bin/tool",
        ] {
            assert!(remote_shell_is_plain(value), "{value}");
        }
        for value in [
            "sh", "bash", "make", "just", "", "ssh -o ProxyCommand=id", "ssh -oProxyCommand=id", "ssh -o LocalCommand=id", "ssh -F ./cfg",
            "ssh -S /tmp/s", "ssh 'x'", "ssh \"-p\" 2", "ssh;id", "ssh -p", "/tmp/ssh", "sshd", "ssh -o", "ssh x id",
            "ssh -o PermitLocalCommand=yes",
        ] {
            assert!(!remote_shell_is_plain(value), "{value}");
        }
    }

    proptest! {
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

use crate::parse::Token;
use crate::registry;
use crate::verdict::{SafetyLevel, Verdict};

/// Recognises the auth-probe pattern: `ssh -T -o BatchMode=yes <host>`
/// plus a small set of harmless connection-shaping flags (-l, -p, -i,
/// -F, -4, -6, -q, -v). The probe opens a TCP connection and exchanges
/// auth packets, but with -T (no PTY) and BatchMode=yes (no interactive
/// prompts) and no remote command, nothing is executed on the remote
/// host -- it succeeds or fails depending on whether auth works, and
/// exits.
///
/// Additional -o KEY=VALUE pairs are accepted, but a small set of
/// dangerous options (port forwarding, proxy commands, X11/agent
/// forwarding, local-command execution) are rejected so they can't be
/// smuggled in via -o.
fn is_auth_probe(tokens: &[Token]) -> bool {
    let mut i = 1;
    let mut has_t = false;
    let mut has_batchmode = false;
    let mut positional_count = 0;
    while i < tokens.len() {
        let t = tokens[i].as_str();
        match t {
            "-T" => {
                has_t = true;
                i += 1;
            }
            "-q" | "-v" | "-4" | "-6" => {
                i += 1;
            }
            "-o" => {
                let Some(next) = tokens.get(i + 1) else { return false; };
                let lower = next.as_str().to_ascii_lowercase();
                if lower == "batchmode=yes" || lower == "batchmode=true" {
                    has_batchmode = true;
                }
                if is_dangerous_ssh_option(&lower) {
                    return false;
                }
                i += 2;
            }
            "-l" | "-p" | "-i" | "-F" => {
                if i + 1 >= tokens.len() {
                    return false;
                }
                i += 2;
            }
            _ if t.starts_with('-') => return false,
            _ => {
                positional_count += 1;
                if positional_count > 1 {
                    return false;
                }
                i += 1;
            }
        }
    }
    has_t && has_batchmode && positional_count == 1
}

fn is_dangerous_ssh_option(lower: &str) -> bool {
    const DANGEROUS_PREFIXES: &[&str] = &[
        "dynamicforward=",
        "forwardagent=yes",
        "forwardx11=yes",
        "forwardx11trusted=yes",
        "localcommand=",
        "localforward=",
        "permitlocalcommand=yes",
        "permitremoteopen=",
        "proxycommand=",
        "proxyjump=",
        "remoteforward=",
        "remotecommand=",
        "tunnel=",
    ];
    DANGEROUS_PREFIXES.iter().any(|p| lower.starts_with(p))
}

pub(crate) fn check_ssh(tokens: &[Token]) -> Verdict {
    if is_auth_probe(tokens) {
        return Verdict::Allowed(SafetyLevel::Inert);
    }
    let has_inspect_flag = tokens[1..]
        .iter()
        .any(|t| matches!(t.as_str(), "-V" | "-G" | "-Q"));
    if !has_inspect_flag {
        return Verdict::Denied;
    }
    registry::try_fallback_grammar("ssh", tokens).unwrap_or(Verdict::Denied)
}

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    use crate::verdict::{SafetyLevel, Verdict};

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    fn verdict(cmd: &str) -> Verdict {
        crate::command_verdict(cmd)
    }

    safe! {
        // Fallback grammar — read-only inspection flags
        version: "ssh -V",
        config_dump_no_host: "ssh -G",
        config_dump_for_host: "ssh -G user@example.com",
        query_cipher: "ssh -Q cipher",
        query_mac: "ssh -Q mac",
        query_kex: "ssh -Q kex",
        query_key: "ssh -Q key",
        query_sig: "ssh -Q sig",

        // Auth-probe pattern
        probe_basic: "ssh -T -o BatchMode=yes git@github.com",
        probe_short_first: "ssh -o BatchMode=yes -T git@github.com",
        probe_batchmode_true: "ssh -T -o BatchMode=true git@github.com",
        probe_with_connect_timeout: "ssh -T -o BatchMode=yes -o ConnectTimeout=3 git@github.com",
        probe_with_connect_timeout_first: "ssh -o ConnectTimeout=3 -T -o BatchMode=yes git@github.com",
        probe_with_port: "ssh -T -o BatchMode=yes -p 2222 git@example.com",
        probe_with_identity: "ssh -T -o BatchMode=yes -i ~/.ssh/id_ed25519 git@example.com",
        probe_with_login: "ssh -T -o BatchMode=yes -l git github.com",
        probe_with_config: "ssh -T -o BatchMode=yes -F /tmp/sshconf git@example.com",
        probe_v4: "ssh -4 -T -o BatchMode=yes git@github.com",
        probe_quiet: "ssh -q -T -o BatchMode=yes git@github.com",
        probe_case_insensitive: "ssh -T -o batchmode=YES git@github.com",
    }

    denied! {
        bare: "ssh",
        bare_host: "ssh user@example.com",
        with_port_no_probe: "ssh -p 22 user@example.com",
        remote_command: "ssh user@example.com ls /",
        local_forward: "ssh -L 8080:localhost:80 user@example.com",
        remote_forward: "ssh -R 8080:localhost:80 user@example.com",
        dynamic_forward: "ssh -D 1080 user@example.com",
        agent_forwarding: "ssh -A user@example.com",
        x11_forwarding: "ssh -X user@example.com",
        background_no_command: "ssh -f -N user@example.com",
        probe_missing_batchmode: "ssh -T user@example.com",
        probe_missing_t: "ssh -o BatchMode=yes user@example.com",
        probe_with_remote_command: "ssh -T -o BatchMode=yes user@example.com ls /",
        probe_with_local_forward: "ssh -T -o BatchMode=yes -L 8080:localhost:80 user@example.com",
        probe_dangerous_o_localcommand: "ssh -T -o BatchMode=yes -o LocalCommand=evil user@example.com",
        probe_dangerous_o_proxycommand: "ssh -T -o BatchMode=yes -o ProxyCommand=evil user@example.com",
        probe_dangerous_o_forwardx11: "ssh -T -o BatchMode=yes -o ForwardX11=yes user@example.com",
        probe_no_host: "ssh -T -o BatchMode=yes",
        probe_two_hosts: "ssh -T -o BatchMode=yes user@a user@b",
        unknown_flag: "ssh --evil",
        unknown_short: "ssh -z",
    }

    #[test]
    fn auth_probe_returns_inert() {
        assert_eq!(
            verdict("ssh -T -o BatchMode=yes git@github.com"),
            Verdict::Allowed(SafetyLevel::Inert),
        );
    }

    #[test]
    fn config_dump_returns_inert() {
        assert_eq!(
            verdict("ssh -G user@example.com"),
            Verdict::Allowed(SafetyLevel::Inert),
        );
    }
}

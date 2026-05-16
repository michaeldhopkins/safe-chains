use crate::parse::Token;
use crate::registry;
use crate::verdict::{SafetyLevel, Verdict};

/// Recognises the auth-probe pattern: `ssh -T -o BatchMode=yes <host>`
/// plus a small set of harmless connection-shaping flags (-l, -p, -i,
/// -F, -4, -6, -q, -v). The probe opens a TCP connection and exchanges
/// auth packets, but with -T (no PTY), BatchMode=yes (no interactive
/// prompts), and no remote command, nothing is executed on the remote
/// host -- it succeeds or fails depending on whether auth works, then
/// exits.
///
/// Additional `-o KEY=VALUE` pairs are accepted only if KEY is on the
/// `SAFE_O_KEYS` allowlist. Keys that enable forwarding, tunneling,
/// proxying, X11/agent forwarding, local-command execution, or override
/// the PTY/session-type decision are absent from that list and are
/// therefore rejected.
///
/// Combined short forms are accepted: `-Tq -o BatchMode=yes` works,
/// `-oBatchMode=yes` works, `-p2222` works.
fn is_auth_probe(tokens: &[Token]) -> bool {
    let mut i = 1;
    let mut has_t = false;
    let mut has_batchmode = false;
    let mut positional_count = 0;
    while i < tokens.len() {
        let t = tokens[i].as_str();
        if let Some(rest) = t.strip_prefix('-')
            && !rest.starts_with('-')
            && !rest.is_empty()
        {
            let chars: Vec<char> = rest.chars().collect();
            let mut ci = 0;
            while ci < chars.len() {
                let c = chars[ci];
                match c {
                    'T' => {
                        has_t = true;
                        ci += 1;
                    }
                    'q' | 'v' | '4' | '6' => {
                        ci += 1;
                    }
                    'o' => {
                        let value: String = if ci + 1 < chars.len() {
                            chars[ci + 1..].iter().collect()
                        } else {
                            let Some(next) = tokens.get(i + 1) else {
                                return false;
                            };
                            i += 1;
                            next.as_str().to_string()
                        };
                        if !check_o_option(&value, &mut has_batchmode) {
                            return false;
                        }
                        ci = chars.len();
                    }
                    'l' | 'p' | 'i' | 'F' => {
                        if ci + 1 < chars.len() {
                            // value attached to the flag chain: -p22, -iFILE, etc.
                        } else if tokens.get(i + 1).is_some() {
                            i += 1;
                        } else {
                            return false;
                        }
                        ci = chars.len();
                    }
                    _ => return false,
                }
            }
            i += 1;
        } else if t.starts_with('-') {
            return false;
        } else {
            positional_count += 1;
            if positional_count > 1 {
                return false;
            }
            i += 1;
        }
    }
    has_t && has_batchmode && positional_count == 1
}

/// `-o KEY=VALUE` option keys safe to pass through. These configure the
/// connection without enabling execution, forwarding, or tunneling.
/// Sorted for `binary_search`.
const SAFE_O_KEYS: &[&str] = &[
    "addressfamily",
    "batchmode",
    "bindaddress",
    "bindinterface",
    "checkhostip",
    "ciphers",
    "compression",
    "connectionattempts",
    "connecttimeout",
    "gssapiauthentication",
    "hashknownhosts",
    "hostkeyalgorithms",
    "hostname",
    "identitiesonly",
    "identityfile",
    "kbdinteractiveauthentication",
    "kexalgorithms",
    "loglevel",
    "macs",
    "numberofpasswordprompts",
    "passwordauthentication",
    "port",
    "preferredauthentications",
    "pubkeyauthentication",
    "serveralivecountmax",
    "serveraliveinterval",
    "stricthostkeychecking",
    "tcpkeepalive",
    "user",
    "userknownhostsfile",
    "verifyhostkeydns",
    "visualhostkey",
];

fn check_o_option(option: &str, has_batchmode: &mut bool) -> bool {
    let lower = option.to_ascii_lowercase();
    let key = lower.split(['=', ' ']).next().unwrap_or("");
    if SAFE_O_KEYS.binary_search(&key).is_err() {
        return false;
    }
    if lower == "batchmode=yes" || lower == "batchmode=true" {
        *has_batchmode = true;
    }
    true
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

        // Combined short flags
        probe_combined_short_tq: "ssh -Tq -o BatchMode=yes git@github.com",
        probe_combined_short_qt: "ssh -qT -o BatchMode=yes git@github.com",
        probe_combined_short_t46q: "ssh -T4q -o BatchMode=yes git@github.com",

        // `-oKEY=VALUE` no-space form
        probe_o_no_space: "ssh -T -oBatchMode=yes git@github.com",
        probe_o_no_space_with_other: "ssh -T -oBatchMode=yes -oConnectTimeout=3 git@github.com",

        // `-pPORT` / `-lUSER` / `-iFILE` no-space form
        probe_p_no_space: "ssh -T -o BatchMode=yes -p2222 git@example.com",
        probe_l_no_space: "ssh -T -o BatchMode=yes -lgit github.com",
        probe_i_no_space: "ssh -T -o BatchMode=yes -i/tmp/key github.com",

        // Combined short chain ending in a valued flag
        probe_combined_tq_then_p: "ssh -Tqp2222 -o BatchMode=yes git@example.com",

        // Safe -o options on the allowlist
        probe_o_stricthostkey: "ssh -T -o BatchMode=yes -o StrictHostKeyChecking=no git@example.com",
        probe_o_user_known_hosts: "ssh -T -o BatchMode=yes -o UserKnownHostsFile=/tmp/kh github.com",
        probe_o_log_level: "ssh -T -o BatchMode=yes -o LogLevel=ERROR github.com",
        probe_o_server_alive: "ssh -T -o BatchMode=yes -o ServerAliveInterval=60 github.com",
        probe_o_preferred_auth: "ssh -T -o BatchMode=yes -o PreferredAuthentications=publickey github.com",
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

        // -o options NOT on the allowlist
        probe_o_localcommand: "ssh -T -o BatchMode=yes -o LocalCommand=evil user@example.com",
        probe_o_proxycommand: "ssh -T -o BatchMode=yes -o ProxyCommand=evil user@example.com",
        probe_o_forwardx11: "ssh -T -o BatchMode=yes -o ForwardX11=yes user@example.com",
        probe_o_forwardagent: "ssh -T -o BatchMode=yes -o ForwardAgent=yes user@example.com",
        probe_o_localforward: "ssh -T -o BatchMode=yes -o LocalForward=8080:host:80 user@example.com",
        probe_o_remoteforward: "ssh -T -o BatchMode=yes -o RemoteForward=8080:host:80 user@example.com",
        probe_o_dynamicforward: "ssh -T -o BatchMode=yes -o DynamicForward=1080 user@example.com",
        probe_o_proxyjump: "ssh -T -o BatchMode=yes -o ProxyJump=jumphost user@example.com",
        probe_o_remotecommand: "ssh -T -o BatchMode=yes -o RemoteCommand=evil user@example.com",
        probe_o_requesttty: "ssh -T -o BatchMode=yes -o RequestTTY=yes user@example.com",
        probe_o_sessiontype: "ssh -T -o BatchMode=yes -o SessionType=subsystem user@example.com",
        probe_o_tunnel: "ssh -T -o BatchMode=yes -o Tunnel=point-to-point user@example.com",
        probe_o_controlmaster: "ssh -T -o BatchMode=yes -o ControlMaster=yes user@example.com",
        probe_o_include: "ssh -T -o BatchMode=yes -o Include=/tmp/evil user@example.com",
        probe_o_match: "ssh -T -o BatchMode=yes -o Match=all user@example.com",
        probe_o_permitlocalcommand: "ssh -T -o BatchMode=yes -o PermitLocalCommand=yes user@example.com",
        probe_o_no_space_unsafe: "ssh -T -oProxyCommand=evil -o BatchMode=yes user@example.com",

        probe_no_host: "ssh -T -o BatchMode=yes",
        probe_two_hosts: "ssh -T -o BatchMode=yes user@a user@b",
        unknown_flag: "ssh --evil",
        unknown_short: "ssh -z",
        unknown_combined_short: "ssh -Tqz -o BatchMode=yes git@example.com",
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

    #[test]
    fn combined_short_returns_inert() {
        assert_eq!(
            verdict("ssh -Tq -o BatchMode=yes git@github.com"),
            Verdict::Allowed(SafetyLevel::Inert),
        );
    }
}

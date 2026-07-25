//! Recognizing a network location that never leaves the machine.
//!
//! The behavioral taxonomy has always had the `loopback` rung on `network.direction` (v1.4 §2.5),
//! and `reader` admits up to it — but nothing derived it from an actual host, so every endpoint
//! read as remote. That forced an all-or-nothing choice on flags like `aws --endpoint-url`: admit
//! it and an authenticated call can be redirected to an attacker's host, or withhold it and deny
//! the developer running DynamoDB Local.
//!
//! This module decides that one question — does this value name THIS machine? — and it is the only
//! place that decides it.
//!
//! # Why it is written as a positive recognizer
//!
//! Being wrong in one direction is an inconvenience and in the other a redirected authenticated
//! request, so this is an allowlist over a tiny vetted set of spellings, and everything it does not
//! positively recognize is remote. `http://2130706433/` really is loopback and this returns `false`
//! for it; that is the intended behavior, not a gap to close. Adding a spelling is cheap; missing a
//! bypass is not.
//!
//! # The bypasses it is built to survive
//!
//! A host string is attacker-controlled, and the attack is always to make our parse disagree with
//! the tool's. The cases that drive this implementation:
//!
//! - `http://localhost@evil.com/` — the authority's host is `evil.com`; `localhost` is userinfo.
//! - `http://evil.com\@localhost/` — WHATWG treats `\` as a path separator, so the host is
//!   `evil.com`; splitting on `@` first would read it as `localhost`.
//! - `http://evil.com#@localhost` and `?@localhost` — the fragment/query start ends the authority.
//! - `http://localhost.evil.com/` — a prefix, not the host.
//! - `http://127.0.0.1.evil.com/` — likewise.
//! - `http://0177.0.0.1/` — leading zeros are octal to some resolvers and decimal to others; an
//!   ambiguous spelling is not recognized at all.
//! - Percent-encoding and non-ASCII (`%6c`, IDN homographs) — the host charset is enforced after
//!   extraction, so no decoding step exists to disagree about.

/// The vetted loopback names. Anything not here, or not an address form below, is remote.
const LOOPBACK_NAMES: &[&str] = &["localhost"];

/// Whether `value` names a network location on this machine.
///
/// Accepts a URL (`http://localhost:8000/path`) or a bare authority (`127.0.0.1:5432`). Recognized:
/// `localhost` and any `*.localhost` subdomain (RFC 6761 §6.3 reserves the TLD to loopback),
/// `127.0.0.0/8` in dotted-quad form (RFC 1122 §3.2.1.3), and the IPv6 loopback `::1`.
pub fn is_loopback(value: &str) -> bool {
    host_of(value).is_some_and(|h| is_loopback_host(&h))
}

/// The host component, lowercased with a trailing root dot removed; `None` when the value cannot be
/// parsed into a host we are confident about. Every `None` is a deny, so ambiguity resolves here.
fn host_of(value: &str) -> Option<String> {
    if value.is_empty() || value.len() > 512 {
        return None;
    }
    // Strip the scheme. A value may also arrive as a bare authority (`localhost:8000`), so the
    // absence of `://` is not itself disqualifying — but a lone `:` then has to be a port, which
    // the host/port split below enforces.
    let after_scheme = match value.find("://") {
        Some(i) => {
            let scheme = &value[..i];
            if scheme.is_empty()
                || !scheme.starts_with(|c: char| c.is_ascii_alphabetic())
                || !scheme.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'+' | b'-' | b'.'))
            {
                return None;
            }
            &value[i + 3..]
        }
        None => value,
    };

    // The authority ends at the first delimiter. `\` is included because WHATWG URL parsing treats
    // it as `/`, so `evil.com\@localhost` has host `evil.com` — cutting here is what keeps the `@`
    // split below from reading the tail as the host.
    let authority = match after_scheme.find(['/', '\\', '?', '#']) {
        Some(i) => &after_scheme[..i],
        None => after_scheme,
    };

    // Userinfo is everything before the LAST `@` (browsers and curl agree; `a@b@host` has host
    // `host`). This is the `localhost@evil.com` case.
    let hostport = match authority.rfind('@') {
        Some(i) => &authority[i + 1..],
        None => authority,
    };
    if hostport.is_empty() {
        return None;
    }

    let host = if let Some(rest) = hostport.strip_prefix('[') {
        // Bracketed IPv6. Anything after the closing bracket must be a port and nothing else.
        let (inner, after) = rest.split_once(']')?;
        if !after.is_empty() && !after.strip_prefix(':').is_some_and(is_port) {
            return None;
        }
        inner
    } else {
        match hostport.split_once(':') {
            // A bare IPv6 without brackets is ambiguous against host:port, so a non-numeric tail is
            // not something to guess at.
            Some((h, port)) if is_port(port) => h,
            Some(_) => return None,
            None => hostport,
        }
    };
    if host.is_empty() {
        return None;
    }

    // Enforced AFTER extraction so no decoding step exists for us and the tool to disagree about:
    // a percent-encoded or non-ASCII host is simply not recognized.
    if !host.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b':')) {
        return None;
    }

    let host = host.to_ascii_lowercase();
    // One trailing root dot is the FQDN spelling of the same name; more than one is malformed.
    let host = host.strip_suffix('.').unwrap_or(&host).to_string();
    if host.is_empty() || host.ends_with('.') {
        return None;
    }
    Some(host)
}

fn is_port(s: &str) -> bool {
    !s.is_empty() && s.len() <= 5 && s.bytes().all(|b| b.is_ascii_digit())
}

fn is_loopback_host(host: &str) -> bool {
    if LOOPBACK_NAMES.contains(&host) {
        return true;
    }
    // RFC 6761 §6.3: names under the `localhost` TLD resolve to loopback. The label before it must
    // be non-empty, so a bare `.localhost` does not qualify.
    if let Some(label) = host.strip_suffix(".localhost")
        && !label.is_empty()
    {
        return true;
    }
    if host == "::1" || host == "0:0:0:0:0:0:0:1" {
        return true;
    }
    is_loopback_v4(host)
}

/// Dotted-quad inside `127.0.0.0/8`. Requires all four octets, plain decimal, no leading zeros —
/// a leading zero is octal to `inet_aton` and decimal to other parsers, and a spelling whose value
/// depends on who reads it is not one to recognize.
fn is_loopback_v4(host: &str) -> bool {
    let mut octets = host.split('.');
    let mut parsed = [0u16; 4];
    for slot in &mut parsed {
        let Some(o) = octets.next() else { return false };
        if o.is_empty() || o.len() > 3 || !o.bytes().all(|b| b.is_ascii_digit()) {
            return false;
        }
        if o.len() > 1 && o.starts_with('0') {
            return false;
        }
        let Ok(v) = o.parse::<u16>() else { return false };
        if v > 255 {
            return false;
        }
        *slot = v;
    }
    octets.next().is_none() && parsed[0] == 127
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    /// Hosts this module is meant to recognize, in the spellings a developer actually types.
    const LOCAL: &[&str] = &[
        "localhost", "LOCALHOST", "localhost.", "app.localhost",
        "127.0.0.1", "127.1.2.3", "127.0.0.255", "[::1]",
    ];
    /// Hosts it must not, including the ones built to look local.
    const REMOTE: &[&str] = &[
        "evil.com", "example.org", "192.168.1.1", "10.0.0.1", "169.254.169.254",
        "localhost.evil.com", "127.0.0.1.evil.com", "notlocalhost", "localhostx",
        "2130706433", "0177.0.0.1", "0.0.0.0",
    ];

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(4000))]

        /// The verdict depends on the HOST COMPONENT and nothing else.
        ///
        /// Every bypass this module has faced is a way to smuggle a local-looking string into some
        /// other part of the URL — userinfo, path, query, fragment — and have a careless parser read
        /// it as the host. Rather than enumerate those tricks one at a time, assemble URLs from
        /// parts and assert the answer tracks the host alone, whatever surrounds it.
        #[test]
        fn only_the_host_component_decides(
            scheme in prop::sample::select(vec!["", "http", "https", "tcp", "HTTP"]),
            userinfo in prop::sample::select(vec!["", "user", "user:pass", "localhost", "127.0.0.1", "a@b"]),
            local in any::<bool>(),
            idx in 0usize..32,
            port in prop::sample::select(vec!["", ":1", ":8000", ":65535"]),
            tail in prop::sample::select(vec!["", "/", "/p", "/p?q=1", "?q=1", "#f", "/@localhost", "#@localhost", "?@localhost"]),
        ) {
            let pool = if local { LOCAL } else { REMOTE };
            let host = pool[idx % pool.len()];
            let mut s = String::new();
            if !scheme.is_empty() {
                s.push_str(&format!("{scheme}://"));
            }
            if !userinfo.is_empty() {
                s.push_str(&format!("{userinfo}@"));
            }
            s.push_str(host);
            s.push_str(&port);
            s.push_str(&tail);
            prop_assert_eq!(
                is_loopback(&s),
                local,
                "host `{}` decides, but `{}` said otherwise", host, s,
            );
        }

        /// Hostnames are case-insensitive, so case must never change the verdict. A missed
        /// lowercase would make `LOCALHOST` deny (an annoyance) or, in a future spelling, make a
        /// case variant slip past a check that only matched lowercase.
        #[test]
        fn case_never_changes_the_verdict(
            local in any::<bool>(),
            idx in 0usize..32,
            port in prop::sample::select(vec!["", ":8000"]),
        ) {
            let pool = if local { LOCAL } else { REMOTE };
            let host = format!("http://{}{}", pool[idx % pool.len()], port);
            prop_assert_eq!(is_loopback(&host), is_loopback(&host.to_uppercase()));
            prop_assert_eq!(is_loopback(&host), is_loopback(&host.to_lowercase()));
        }

        /// A recognized host stops being recognized the moment it becomes a LABEL of some other
        /// domain. This is the `localhost.evil.com` class, stated for arbitrary suffixes rather
        /// than the three the table happens to list.
        #[test]
        fn a_local_host_under_another_domain_is_remote(
            idx in 0usize..32,
            suffix in "[a-z][a-z0-9-]{0,20}\\.[a-z]{2,6}",
        ) {
            let host = LOCAL[idx % LOCAL.len()];
            // A bracketed IPv6 cannot take a suffix; `[::1].evil.com` is malformed, not a spoof.
            if host.starts_with('[') {
                return Ok(());
            }
            let spoof = format!("http://{}.{}", host.trim_end_matches('.'), suffix);
            prop_assert!(!is_loopback(&spoof), "expected remote: {}", spoof);
        }

        /// Never panics, whatever bytes arrive. `--endpoint-url` takes an attacker-influenced value
        /// and a panic in the classifier is an availability bug in every harness hook that calls it.
        #[test]
        fn never_panics_on_any_input(s in ".*") {
            let _ = is_loopback(&s);
        }

        /// Adding a valid port never changes the verdict — the host decides, the port is noise.
        #[test]
        fn a_port_never_changes_the_verdict(
            local in any::<bool>(),
            idx in 0usize..32,
            port in 1u32..65535,
        ) {
            let pool = if local { LOCAL } else { REMOTE };
            let host = pool[idx % pool.len()];
            prop_assert_eq!(
                is_loopback(&format!("http://{host}")),
                is_loopback(&format!("http://{host}:{port}")),
            );
        }
    }


    #[test]
    fn recognizes_the_vetted_loopback_spellings() {
        for v in [
            "http://localhost",
            "http://localhost:8000",
            "http://localhost:8000/path?q=1",
            "https://LOCALHOST:443",
            "http://localhost.",
            "http://localhost.:8000",
            "http://myapp.localhost:3000",
            "http://127.0.0.1",
            "http://127.0.0.1:8000",
            "http://127.1.2.3:9",
            "http://127.0.0.255",
            "http://[::1]",
            "http://[::1]:8000",
            "localhost",
            "localhost:5432",
            "127.0.0.1:5432",
            "tcp://127.0.0.1:2375",
            "http://user:pass@localhost:8000",
        ] {
            assert!(is_loopback(v), "expected loopback: {v}");
        }
    }

    /// Each of these is a way to make our parse disagree with the tool's. A regression here is a
    /// redirected authenticated request, not a cosmetic bug.
    #[test]
    fn rejects_hosts_that_only_look_local() {
        for v in [
            // userinfo carrying the local-looking name
            "http://localhost@evil.com",
            "http://localhost@evil.com/x",
            "http://a@localhost@evil.com",
            "http://127.0.0.1@evil.com",
            // backslash is a path separator to WHATWG, so the host is evil.com
            "http://evil.com\\@localhost",
            "http://evil.com\\@127.0.0.1",
            // the authority ends at the query/fragment
            "http://evil.com#@localhost",
            "http://evil.com?@localhost",
            "http://evil.com/@localhost",
            // prefix/suffix confusion
            "http://localhost.evil.com",
            "http://127.0.0.1.evil.com",
            "http://notlocalhost",
            "http://localhostx",
            "http://evil-localhost.com",
            // `.localhost` needs a real label
            "http://.localhost",
            // ambiguous or unrecognized address spellings — loopback in fact, denied on principle
            "http://2130706433",
            "http://0177.0.0.1",
            "http://127.0.0.01",
            "http://0x7f000001",
            "http://127.1",
            "http://127.0.0.256",
            "http://127.0.0.1.1",
            // encoding tricks have no decode step to exploit
            "http://%6cocalhost",
            "http://loc%61lhost",
            "http://localhos\u{0074}.evil.com",
            // not this machine
            "http://192.168.1.1",
            "http://10.0.0.1",
            "http://169.254.169.254",
            "https://evil.com",
            // 0.0.0.0 means "every interface" when binding; as a target it is not a name for here
            "http://0.0.0.0:8000",
            // malformed
            "http://",
            "http://[::1",
            "http://[::1]x",
            "http://localhost:notaport",
            "http://localhost..",
            "",
        ] {
            assert!(!is_loopback(v), "expected NOT loopback: {v}");
        }
    }

    #[test]
    fn a_local_looking_label_never_survives_being_a_subdomain_of_something_else() {
        // Property: appending any registrable suffix to a recognized host must un-recognize it.
        for local in ["localhost", "127.0.0.1", "app.localhost"] {
            assert!(is_loopback(local), "sanity: {local}");
            for suffix in ["evil.com", "attacker.net", "co.uk"] {
                let spoof = format!("http://{local}.{suffix}");
                assert!(!is_loopback(&spoof), "expected NOT loopback: {spoof}");
            }
        }
    }

    #[test]
    fn userinfo_never_decides_the_host() {
        // Property: whatever precedes an `@`, the host is what follows the last one.
        for user in ["localhost", "127.0.0.1", "[::1]", "a@localhost", "x"] {
            let spoof = format!("http://{user}@evil.com/");
            assert!(!is_loopback(&spoof), "expected NOT loopback: {spoof}");
            let real = format!("http://{user}@localhost:8000/");
            assert!(is_loopback(&real), "expected loopback: {real}");
        }
    }

    #[test]
    fn the_v4_recognizer_covers_the_whole_loopback_block_and_nothing_outside_it() {
        for a in [0u16, 1, 9, 10, 126, 127, 128, 192, 255] {
            for d in [0u16, 1, 127, 254, 255] {
                let host = format!("{a}.0.0.{d}");
                assert_eq!(
                    is_loopback(&host),
                    a == 127,
                    "127.0.0.0/8 membership decides {host}",
                );
            }
        }
    }

    #[test]
    fn never_panics_on_arbitrary_input() {
        for v in [
            "\u{0}", "://", "]", "[", "@", ":", "...", "http://:", "http://@",
            "http://[]", "http://[]:", "\\\\", "a://b://c", &"a".repeat(1024),
            "http://[::1]:99999999", "http://localhost:00000",
        ] {
            let _ = is_loopback(v);
        }
    }
}

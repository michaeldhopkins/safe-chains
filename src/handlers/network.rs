use crate::parse::{Token, WordSet};
use crate::verdict::{SafetyLevel, Verdict};

static CURL_SAFE_STANDALONE: WordSet = WordSet::new(&[
    "--compressed", "--fail", "--globoff", "--head", "--include", "--insecure", "--ipv4", "--ipv6", "--location", "--no-buffer",
    "--no-progress-meter", "--show-error", "--silent", "--verbose", "-4", "-6", "-I", "-L", "-N", "-S", "-f", "-g", "-i", "-k", "-s", "-v",
]);

static CURL_SAFE_VALUED: WordSet = WordSet::new(&["--connect-timeout", "--max-time", "--user-agent", "--write-out", "-A", "-m", "-w"]);

static CURL_SAFE_METHODS: WordSet = WordSet::new(&["GET", "HEAD", "OPTIONS"]);

const CURL_STANDALONE_SHORT: &[u8] = b"46ILNOSfgiksv";
const CURL_VALUED_SHORT: &[u8] = b"Amow";

fn is_safe_method(method: &str) -> bool {
    CURL_SAFE_METHODS.contains(&method.to_ascii_uppercase())
}

fn is_safe_curl_header(value: &str) -> bool {
    if value.contains(['\r', '\n']) {
        return false;
    }
    let Some((name, _)) = value.split_once(':') else {
        return false;
    };
    let trimmed = name.trim();
    CURL_SAFE_HEADERS.iter().any(|h| trimmed.eq_ignore_ascii_case(h))
}

const CURL_SAFE_HEADERS: &[&str] = &[
    "Accept", "Accept-Charset", "Accept-Encoding", "Accept-Language", "Authorization", "Cache-Control", "Cookie", "If-Match",
    "If-Modified-Since", "If-None-Match", "If-Range", "If-Unmodified-Since", "Origin", "Range", "Referer", "User-Agent",
    "X-Correlation-ID", "X-Forwarded-For", "X-Forwarded-Host", "X-Forwarded-Proto", "X-GitHub-Api-Version", "X-Request-ID",
    "X-Requested-With",
];

/// curl's `--write-out` reads its format from a file when the value starts with `@` (`@-` =
/// stdin), and `%output{file}` (curl 8.3+) sends the rest of the output to a file. Admit only a
/// literal format that does neither.
fn write_out_admits(value: &str) -> bool {
    !value.starts_with('@') && !value.to_ascii_lowercase().contains("%output")
}

/// curl guesses a protocol for a URL without a scheme (`dict.example.com` becomes `dict://`) and
/// expands `{a,b}` and `[a-b]` globs before reading the scheme, so only a literal `http://` or
/// `https://` prefix is admitted.
fn is_allowed_url(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    lower.starts_with("http://") || lower.starts_with("https://")
}

fn check_curl_valued(t: &Token, next: Option<&Token>, has_write: &mut bool) -> Option<Result<usize, ()>> {
    if t == "-o" || t == "--output" {
        *has_write = true;
        return Some(Ok(2));
    }
    if t == "-H" || t == "--header" {
        return if next.is_some_and(|v| is_safe_curl_header(v)) { Some(Ok(2)) } else { Some(Err(())) };
    }
    if t == "-w" || t == "--write-out" {
        return Some(match next {
            Some(v) if !write_out_admits(v.as_str()) => Err(()),
            _ => Ok(2),
        });
    }
    if t == "-A" || t == "--user-agent" {
        return Some(match next {
            Some(v) if v.contains(['\r', '\n']) => Err(()),
            _ => Ok(2),
        });
    }
    if CURL_SAFE_VALUED.contains(t) {
        return Some(Ok(2));
    }
    None
}

pub fn is_safe_curl(tokens: &[Token]) -> Verdict {
    if tokens.len() == 2 && matches!(tokens[1].as_str(), "--help" | "-h" | "--version" | "-V") {
        return Verdict::Allowed(SafetyLevel::Inert);
    }
    let mut has_write = false;
    let mut i = 1;
    while i < tokens.len() {
        let t = &tokens[i];

        if !t.starts_with('-') {
            if !is_allowed_url(t.as_str()) {
                return Verdict::Denied;
            }
            i += 1;
            continue;
        }

        if CURL_SAFE_STANDALONE.contains(t) {
            i += 1;
            continue;
        }

        if t == "-O" || t == "--remote-name" {
            has_write = true;
            i += 1;
            continue;
        }

        if let Some(advance) = check_curl_valued(t, tokens.get(i + 1), &mut has_write) {
            match advance {
                Ok(skip) => {
                    i += skip;
                    continue;
                }
                Err(()) => return Verdict::Denied,
            }
        }

        if t == "-X" || t == "--request" {
            if !tokens.get(i + 1).is_some_and(|m| is_safe_method(m)) {
                return Verdict::Denied;
            }
            i += 2;
            continue;
        }
        if t.starts_with("-X") && t.len() > 2 {
            if !t.get(2..).is_some_and(is_safe_method) {
                return Verdict::Denied;
            }
            i += 1;
            continue;
        }

        if t.starts_with('-') && !t.starts_with("--") && t.len() > 2 {
            let bytes = t.as_bytes();
            for (j, &b) in bytes[1..].iter().enumerate() {
                let is_last = j == bytes.len() - 2;
                if CURL_STANDALONE_SHORT.contains(&b) {
                    if b == b'O' {
                        has_write = true;
                    }
                    continue;
                }
                if is_last && CURL_VALUED_SHORT.contains(&b) {
                    if b == b'o' {
                        has_write = true;
                    }
                    if b == b'w' && tokens.get(i + 1).is_some_and(|v| !write_out_admits(v.as_str())) {
                        return Verdict::Denied;
                    }
                    if b == b'A' && tokens.get(i + 1).is_some_and(|v| v.contains(['\r', '\n'])) {
                        return Verdict::Denied;
                    }
                    i += 1;
                    break;
                }
                return Verdict::Denied;
            }
            i += 1;
            continue;
        }

        return Verdict::Denied;
    }
    if i <= 1 {
        return Verdict::Denied;
    }
    let level = if has_write { SafetyLevel::SafeWrite } else { SafetyLevel::Inert };
    Verdict::Allowed(level)
}

#[cfg(test)]
pub(super) const REGISTRY: &[super::CommandEntry] =
    &[super::CommandEntry::Custom { cmd: "curl", valid_prefix: Some("curl https://example.com") }];

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use crate::is_safe_command;
    use crate::parse::Token;
    use crate::verdict::{SafetyLevel, Verdict};

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        curl_bare_get: "curl https://example.com",
        curl_silent: "curl -s https://example.com",
        curl_combined_flags: "curl -sSLk https://example.com",
        curl_head: "curl -I https://example.com",
        curl_timeout: "curl --connect-timeout 5 -m 10 https://example.com",
        curl_explicit_get: "curl -X GET https://example.com",
        curl_combined_x: "curl -XGET https://example.com",
        curl_head_method: "curl -X HEAD https://example.com",
        curl_options_method: "curl -X OPTIONS https://example.com",
        curl_write_out: "curl -w '%{http_code}' -s https://example.com",
        curl_multiple_urls: "curl -s https://example.com https://example.org",
        curl_compressed: "curl --compressed -s https://example.com",
        curl_no_progress: "curl -sS --no-progress-meter https://example.com",
        curl_ipv4: "curl -4 https://example.com",
        curl_fssl: "curl -fsSL https://example.com",
        curl_verbose: "curl -v https://example.com",
        curl_globoff: "curl -g https://example.com",
        curl_no_buffer: "curl -N https://example.com",
        curl_fail: "curl -f https://example.com",
        curl_version: "curl --version",
        curl_help: "curl --help",
        curl_method_case_insensitive: "curl -X get https://example.com",
        curl_combined_then_valued: "curl -sSm 10 https://example.com",
        curl_output: "curl -o output.html https://example.com",
        curl_output_long: "curl --output file.html https://example.com",
        curl_remote_name: "curl -O https://example.com/file.tar.gz",
        curl_remote_name_long: "curl --remote-name https://example.com/f.tar.gz",
        curl_combined_with_output: "curl -sSo output.html https://example.com",
        curl_get_then_output: "curl -X GET -o /tmp/file https://example.com",
        curl_header_user_agent: "curl -s https://example.com -H 'User-Agent: Mozilla/5.0'",
        curl_header_accept: "curl -s https://example.com -H 'Accept: text/html'",
        curl_header_auth: "curl -s https://example.com -H 'Authorization: Bearer token123'",
        curl_header_cache: "curl -s https://example.com -H 'Cache-Control: no-cache'",
        curl_header_cookie: "curl -s https://example.com -H 'Cookie: session=abc'",
        curl_header_range: "curl -s https://example.com -H 'Range: bytes=0-1023'",
        curl_header_if_none: "curl -s https://example.com -H 'If-None-Match: \"etag\"'",
        curl_header_origin: "curl -s https://example.com -H 'Origin: https://example.com'",
        curl_header_referer: "curl -s https://example.com -H 'Referer: https://example.com'",
        curl_header_accept_encoding: "curl -s https://example.com -H 'Accept-Encoding: gzip'",
        curl_header_github_api: "curl -s https://api.github.com -H 'X-GitHub-Api-Version: 2022-11-28'",
        curl_multiple_headers: "curl -s https://example.com -H 'Accept: text/html' -H 'User-Agent: Bot'",
        curl_user_agent_short: "curl -A 'Mozilla/5.0' https://example.com",
        curl_user_agent_long: "curl --user-agent 'Mozilla/5.0' https://example.com",
        curl_user_agent_combined: "curl -sLA 'Mozilla/5.0' --max-time 15 https://example.com",
        curl_include_short: "curl -i https://example.com",
        curl_include_long: "curl --include https://example.com",
        curl_include_combined: "curl -sSi https://example.com",
        curl_include_localhost: "curl -sS -i http://127.0.0.1:8001/",
        curl_scheme_uppercase: "curl HTTPS://example.com",
        curl_write_out_percent_literal: "curl -s -w '%{http_code}\\n' -o /dev/null https://example.com",
    }

    denied! {
        curl_post: "curl -X POST https://example.com",
        curl_put: "curl -X PUT https://example.com",
        curl_delete: "curl -X DELETE https://example.com",
        curl_data: "curl -d '{\"key\":\"val\"}' https://example.com",
        curl_json: "curl --json '{\"key\":\"val\"}' https://example.com",
        curl_form: "curl -F 'file=@secret.txt' https://example.com",
        curl_header_content_type: "curl -H 'Content-Type: application/json' https://example.com",
        curl_header_method_override: "curl -H 'X-HTTP-Method-Override: DELETE' https://example.com",
        curl_header_transfer_encoding: "curl -H 'Transfer-Encoding: chunked' https://example.com",
        curl_header_no_colon: "curl -H 'BadHeader' https://example.com",
        curl_header_crlf_injection: "curl -H $'Accept: x\\r\\nContent-Length: 5\\r\\n\\r\\nhello' https://example.com",
        curl_header_newline_injection: "curl -H $'Accept: x\\nX-Evil: 1' https://example.com",
        curl_user_agent_crlf: "curl -A $'x\\r\\nContent-Length: 5' https://example.com",
        curl_user_agent_long_newline: "curl --user-agent $'x\\nX-Evil: 1' https://example.com",
        curl_user_agent_cluster_crlf: "curl -sA $'x\\r\\nX: 1' https://example.com",
        curl_cookie_flag: "curl -b 'session=abc' https://example.com",
        curl_user: "curl -u admin:pass https://example.com",
        curl_referer_flag: "curl -e 'https://evil.com' https://example.com",
        curl_cookie_jar: "curl -c cookies.txt https://example.com",
        curl_config: "curl -K config.txt",
        curl_upload: "curl -T file.txt https://example.com",
        curl_bare: "curl",
        curl_netrc: "curl -n https://example.com",
        curl_data_long: "curl --data 'key=val' https://example.com",
        curl_form_long: "curl --form 'file=@f' https://example.com",
        curl_dump_header: "curl -D headers.txt https://example.com",
        curl_request_eq_post: "curl --request=POST https://example.com",
        curl_xpost: "curl -XPOST https://example.com",
        curl_get_then_data: "curl -X GET -d 'payload' https://example.com",
        curl_xget_then_bad_header: "curl -XGET -H 'Content-Type: text/plain' https://example.com",
        curl_request_eq_get_then_data: "curl --request=GET -d 'payload' https://example.com",
        // file: URLs read a LOCAL file to stdout — gated by read locus, so a secret denies.
        curl_file_scheme_secret: "curl file:///etc/shadow",
        curl_file_scheme_host: "curl file://localhost/etc/shadow",
        curl_file_scheme_home_key: "curl file:///root/.ssh/id_rsa",
        curl_file_single_slash: "curl file:/etc/shadow",
        // schemes are case-insensitive — uppercase/mixed must not slip past the gate.
        curl_file_scheme_upper: "curl FILE:///etc/shadow",
        curl_file_scheme_mixed: "curl File:///etc/shadow",
        curl_file_scheme_wild: "curl fIlE:///root/.ssh/id_rsa",
        // --write-out @file reads that file to stdout — a secret there must deny, every form.
        curl_write_out_at_secret: "curl -w @/etc/shadow https://x.com",
        curl_write_out_at_secret_long: "curl --write-out @/etc/shadow https://x.com",
        curl_write_out_at_secret_eq: "curl --write-out=@/root/.ssh/id_rsa https://x.com",
        curl_write_out_at_secret_cluster: "curl -sw @/etc/shadow https://x.com",
        curl_write_out_at_worktree: "curl -w @./fmt.txt -s https://example.com",
        curl_write_out_at_stdin: "curl -w @- -s https://example.com",
        curl_write_out_output_file: "curl -w '%output{/tmp/x}' https://example.com",
        curl_write_out_output_append: "curl -w '%output{>>/tmp/x}%{http_code}' https://example.com",
        curl_write_out_output_long: "curl --write-out '%output{x.txt}' https://example.com",
        curl_write_out_output_cluster: "curl -sw '%output{x.txt}' https://example.com",
        curl_write_out_output_upper: "curl -w '%OUTPUT{x.txt}' https://example.com",
        curl_gopher_scheme: "curl gopher://example.com/",
        curl_dict_scheme: "curl dict://example.com/d:word",
        curl_ftp_scheme: "curl ftp://example.com/file",
        curl_ldap_scheme: "curl ldap://example.com/",
        curl_file_scheme_worktree: "curl file://./README.md",
        curl_scheme_less: "curl example.com",
        curl_scheme_less_guessed: "curl dict.example.com",
        curl_scheme_glob: "curl '{gopher,http}://example.com/'",
        curl_scheme_single_slash: "curl http:/example.com",
        curl_second_url_gopher: "curl https://example.com gopher://example.com/",
        curl_request_eq: "curl --request=GET https://example.com",
        curl_valued_eq: "curl --max-time=30 https://example.com",
        curl_output_eq: "curl --output=file.html https://example.com",
        curl_header_eq: "curl -s https://example.com --header='Accept: text/html'",
        curl_user_agent_eq: "curl --user-agent='Mozilla/5.0' https://example.com",
        curl_connect_timeout_eq: "curl --connect-timeout=5 https://example.com",
        curl_write_out_eq: "curl --write-out=%{http_code} https://example.com",
    }

    inert! {
        curl_level_inert: "curl -s https://example.com",
        curl_level_header_inert: "curl -s https://example.com -H 'Accept: text/html'",
    }

    safe_write! {
        curl_level_output: "curl -s https://example.com -o /tmp/file.html",
        curl_level_remote_name: "curl -O https://example.com/file.tar.gz",
        curl_level_combined_output: "curl -sSo /tmp/file https://example.com",
    }

    #[test]
    fn the_reference_lists_every_admitted_flag_method_and_header() {
        let toml = include_str!("../../commands/net/curl.toml");
        let body = toml.lines().find(|l| l.starts_with("doc_body")).expect("doc_body");
        let standalone = crate::docs::wordset_items(&super::CURL_SAFE_STANDALONE);
        let valued = crate::docs::wordset_items(&super::CURL_SAFE_VALUED);
        let methods = crate::docs::wordset_items(&super::CURL_SAFE_METHODS);
        assert!(body.contains(&format!("Allowed standalone flags: {standalone}.")));
        assert!(body.contains(&format!("Allowed valued flags: {valued}.")));
        assert!(body.contains(&format!("Allowed methods (-X/--request): {methods}.")));
        for header in super::CURL_SAFE_HEADERS {
            assert!(body.contains(header), "{header}");
        }
    }

    #[test]
    fn header_values_with_line_breaks_are_refused() {
        assert!(super::is_safe_curl_header("Accept: text/html"));
        assert!(!super::is_safe_curl_header("Accept: x\r\nContent-Length: 5"));
        assert!(!super::is_safe_curl_header("Accept: x\nX-Evil: 1"));
        assert!(!super::is_safe_curl_header("Accept: x\r"));
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(500))]

        /// `is_safe_method` is the case-insensitive membership test for the allowed-method set,
        /// defined here against an explicit reference oracle rather than against the implementation's
        /// own body. The implementation calls `to_ascii_uppercase` and then a `WordSet::contains`
        /// binary search; the oracle is `eq_ignore_ascii_case` against the literal list. Equating the
        /// two pins the project-wide rule "method equality is ASCII-case-insensitive" — a regression
        /// that flipped either side (e.g. dropped the call to `to_ascii_uppercase` or tightened the
        /// set) would break one side of the equation, not both.
        #[test]
        fn is_safe_method_matches_a_simple_case_insensitive_reference_set(method in any::<String>()) {
            const REFERENCE: &[&str] = &["GET", "HEAD", "OPTIONS"];
            let expected = REFERENCE.iter().any(|m| method.eq_ignore_ascii_case(m));
            prop_assert_eq!(
                super::is_safe_method(&method),
                expected,
                "method {:?} should match the reference set iff {:?}", method, expected
            );
        }

        /// The header predicate parses `value` by splitting on the first `:`. A value that contains
        /// no colon at all is rejected before any name comparison runs, so the absence of a colon
        /// is an unconditional deny — independently of what the rest of `value` looks like, what
        /// case the bytes are in, and how much whitespace surrounds them. Pins the structural
        /// pre-check the function relies on: a regression that "fixed" the no-colon case by also
        /// tolerating an empty name (or matched the `CURL_SAFE_HEADERS` set directly on the whole
        /// value) would let `Content-Type application/json` and every other word that happens to be
        /// a safe-header string slip through.
        #[test]
        fn a_safe_curl_header_value_lacks_a_colon_it_is_denied(value in any::<String>()) {
            prop_assume!(!value.contains(':'));
            prop_assert!(
                !super::is_safe_curl_header(&value),
                "header value without `:` must be denied, but `{}` was admitted", value
            );
        }

        /// Roundtrip: a known-safe header name, whatever its original case and whatever whitespace
        /// surrounds it, still gets the value admitted as long as the colon is present. The
        /// implementation calls `name.trim()` and `eq_ignore_ascii_case`, so the surface the
        /// predicate is invariant on is `(name-after-trim, ASCII-case)` — anything that preserves
        /// that surface must keep admitting, anything that breaks it must keep denying. Pins the
        /// shape the surrounding `-H`/`--header` flag handlers rely on (the value the hook inspects
        /// is quoted, untokenized, and may come in mixed case from a real `curl` invocation).
        #[test]
        fn a_safe_curl_header_value_round_trips_through_name_case_and_whitespace(
            header in proptest::sample::select(super::CURL_SAFE_HEADERS),
            flip in any::<bool>(),
            leading_ws in "[ \t]{0,3}",
            trailing_ws in "[ \t]{0,3}",
            value in "[A-Za-z0-9./_+-]{0,12}",
        ) {
            // Fold once one way and once the other. `flip` selects between them so each iteration
            // lands somewhere along the case spectrum — `Accept`, `accept`, `ACCEPT`, and any mix
            // the half-way turns produce by inspecting byte position vs. flip. A regression that
            // dropped the `to_ascii_uppercase` fold (or compared against the set without
            // `eq_ignore_ascii_case`) would let one of these admits land on the wrong side.
            let name = if flip { header.to_ascii_uppercase() } else { header.to_ascii_lowercase() };

            let spelling = format!("{leading_ws}{name}{trailing_ws}:{value}");
            prop_assert!(
                super::is_safe_curl_header(&spelling),
                "header {:?} in case `{}` (whitespace + `:` + body) must remain admitted",
                header, name,
            );

            // The other half of the contract: a header that is NEITHER in the safe set NOR
            // case-insensitively equal to one must keep denying, regardless of whitespace around
            // its first-colon half. Constructed by inserting a leading `x` so the name cannot hit
            // the set; the leading byte stays put across any case fold.
            let bad = format!("{leading_ws}x{name}{trailing_ws}:{value}");
            prop_assert!(
                !super::is_safe_curl_header(&bad),
                "a header prefixed with `x` must be denied, but `{}` was admitted", bad
            );
        }

        /// The two-token `curl --help`/`-h`/`--version`/`-V` short-circuit returns exactly
        /// `Allowed(Inert)` — neither a write lift nor a denial. Stated as "tokens.len() == 2 and
        /// the second is one of the four help/version flags implies Allowed(Inert)"; combined with
        /// the existing `denied!` and `safe_write!` corpus covering non-help forms, this pins the
        /// precise contract the public surface advertises. A regression that promoted the
        /// short-circuit through `SafeWrite` would surface here as a level mismatch; one that turned
        /// it into `Denied` would surface as the wrong variant.
        #[test]
        fn is_safe_curl_help_or_version_in_two_tokens_is_always_inert(
            flag in proptest::sample::select(&["--help", "-h", "--version", "-V"]),
        ) {
            let tokens: Vec<Token> = vec![Token::from_test("curl"), Token::from_test(flag)];
            let verdict = super::is_safe_curl(&tokens);
            prop_assert_eq!(
                verdict,
                Verdict::Allowed(SafetyLevel::Inert),
                "two-token `curl {}` must short-circuit to Allowed(Inert)",
                flag
            );
        }

        /// A URL operand is admitted only when it begins with a literal `http://` or `https://`.
        /// Any other scheme, a scheme-less host curl would guess a protocol for, or a glob in the
        /// scheme position is refused.
        #[test]
        fn a_curl_url_is_admitted_only_with_an_http_scheme(url in "[a-zA-Z0-9:/.{},\\[\\]_@-]{1,24}") {
            prop_assume!(!url.starts_with('-'));
            let tokens: Vec<Token> = ["curl", "-s", url.as_str()].iter().map(|s| Token::from_test(s)).collect();
            let lower = url.to_ascii_lowercase();
            let expected = lower.starts_with("http://") || lower.starts_with("https://");
            prop_assert_eq!(super::is_safe_curl(&tokens).is_allowed(), expected, "url {:?}", url);
        }

        #[test]
        fn a_curl_url_with_an_http_scheme_in_any_case_is_admitted(
            scheme in "[hH][tT][tT][pP][sS]?",
            host in "[a-z0-9.-]{1,16}",
            path in "[a-zA-Z0-9/._-]{0,16}",
        ) {
            let url = format!("{scheme}://{host}/{path}");
            let tokens: Vec<Token> = ["curl", "-s", url.as_str()].iter().map(|s| Token::from_test(s)).collect();
            prop_assert!(super::is_safe_curl(&tokens).is_allowed(), "url {:?} was refused", url);
        }

        /// curl reads `--flag=value` as an unknown option, so no long flag is admitted in that
        /// spelling: the check and curl must agree on which word is the value.
        #[test]
        fn a_curl_long_flag_glued_with_equals_is_refused(
            flag in proptest::sample::select(
                ["--connect-timeout", "--max-time", "--user-agent", "--write-out", "--output", "--header", "--request"].to_vec()
            ),
            value in "[a-zA-Z0-9 :./%{}_-]{0,16}",
        ) {
            let glued = format!("{flag}={value}");
            let tokens: Vec<Token> =
                ["curl", glued.as_str(), "https://example.com"].iter().map(|s| Token::from_test(s)).collect();
            prop_assert_eq!(super::is_safe_curl(&tokens), Verdict::Denied, "`{}` was admitted", glued);
        }

        /// A `--write-out` format that reads a file (`@…`) or writes one (`%output{…}`) is refused
        /// in every spelling of the flag.
        #[test]
        fn a_curl_write_out_that_reads_or_writes_a_file_is_refused(
            form in proptest::sample::select(["-w", "--write-out", "-sw", "-Sw"].to_vec()),
            before in "[a-z%{}_ ]{0,8}",
            directive in proptest::sample::select(["%output{", "%OUTPUT{", "%Output{>>"].to_vec()),
            path in "[a-zA-Z0-9/._-]{1,12}",
            at in any::<bool>(),
        ) {
            let value = if at { format!("@{path}") } else { format!("{before}{directive}{path}}}") };
            let tokens: Vec<Token> =
                ["curl", form, value.as_str(), "https://example.com"].iter().map(|s| Token::from_test(s)).collect();
            prop_assert_eq!(super::is_safe_curl(&tokens), Verdict::Denied, "`{} {}` was admitted", form, value);
        }
    }
}

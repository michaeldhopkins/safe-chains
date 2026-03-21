use crate::parse::{Token, WordSet};
use crate::verdict::{SafetyLevel, Verdict};

static CURL_SAFE_STANDALONE: WordSet = WordSet::new(&[
    "--compressed", "--fail", "--globoff", "--head", "--insecure",
    "--ipv4", "--ipv6", "--location", "--no-buffer", "--no-progress-meter",
    "--show-error", "--silent", "--verbose",
    "-4", "-6", "-I", "-L", "-N", "-S", "-f", "-g", "-k", "-s", "-v",
]);

static CURL_SAFE_VALUED: WordSet = WordSet::new(&[
    "--connect-timeout", "--max-time", "--write-out",
    "-m", "-w",
]);

static CURL_SAFE_METHODS: WordSet = WordSet::new(&["GET", "HEAD", "OPTIONS"]);

const CURL_STANDALONE_SHORT: &[u8] = b"46ILNOSfgksv";
const CURL_VALUED_SHORT: &[u8] = b"mow";

fn is_safe_method(method: &str) -> bool {
    CURL_SAFE_METHODS.contains(&method.to_ascii_uppercase())
}

fn is_safe_curl_header(value: &str) -> bool {
    let Some((name, _)) = value.split_once(':') else {
        return false;
    };
    let trimmed = name.trim();
    CURL_SAFE_HEADERS.iter().any(|h| trimmed.eq_ignore_ascii_case(h))
}

const CURL_SAFE_HEADERS: &[&str] = &[
    "Accept", "Accept-Charset", "Accept-Encoding", "Accept-Language",
    "Authorization",
    "Cache-Control", "Cookie",
    "If-Match", "If-Modified-Since", "If-None-Match", "If-Range", "If-Unmodified-Since",
    "Origin",
    "Range", "Referer",
    "User-Agent",
    "X-Correlation-ID", "X-Forwarded-For", "X-Forwarded-Host", "X-Forwarded-Proto",
    "X-GitHub-Api-Version", "X-Request-ID", "X-Requested-With",
];

fn check_curl_valued(t: &Token, next: Option<&Token>, has_write: &mut bool) -> Option<Result<usize, ()>> {
    if t == "-o" || t == "--output" {
        *has_write = true;
        return Some(Ok(2));
    }
    if t == "-H" || t == "--header" {
        return if next.is_some_and(|v| is_safe_curl_header(v)) { Some(Ok(2)) } else { Some(Err(())) };
    }
    if CURL_SAFE_VALUED.contains(t) {
        return Some(Ok(2));
    }
    if let Some(val) = t.split_value("=") {
        let flag = t.as_str().split_once('=').map_or(t.as_str(), |(k, _)| k);
        if CURL_SAFE_VALUED.contains(flag) || flag == "--output" {
            if flag == "--output" { *has_write = true; }
            return Some(Ok(1));
        }
        if flag == "--request" {
            return if is_safe_method(val) { Some(Ok(1)) } else { Some(Err(())) };
        }
        if flag == "--header" {
            return if is_safe_curl_header(val) { Some(Ok(1)) } else { Some(Err(())) };
        }
        return Some(Err(()));
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
                Ok(skip) => { i += skip; continue; }
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

pub(crate) fn dispatch(cmd: &str, tokens: &[Token]) -> Option<Verdict> {
    match cmd {
        "curl" => Some(is_safe_curl(tokens)),
        _ => None,
    }
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    use crate::docs::{CommandDoc, DocBuilder, wordset_items};
    vec![
        CommandDoc::handler("curl",
            "https://curl.se/docs/manpage.html",
            DocBuilder::new()
                .section(format!(
                    "Allowed standalone flags: {}.",
                    wordset_items(&CURL_SAFE_STANDALONE),
                ))
                .section(format!(
                    "Allowed valued flags: {}.",
                    wordset_items(&CURL_SAFE_VALUED),
                ))
                .section(format!(
                    "Allowed methods (-X/--request): {}.",
                    wordset_items(&CURL_SAFE_METHODS),
                ))
                .section("-H/--header allowed with safe headers (Accept, User-Agent, Authorization, Cookie, Cache-Control, Range, etc.).")
                .section("-o/--output and -O/--remote-name allowed (writes files).")
                .build()),
    ]
}

#[cfg(test)]
pub(super) const REGISTRY: &[super::CommandEntry] = &[
    super::CommandEntry::Custom { cmd: "curl", valid_prefix: Some("curl https://example.com") },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

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
        curl_request_eq: "curl --request=GET https://example.com",
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
        curl_valued_eq: "curl --max-time=30 https://example.com",
        curl_combined_then_valued: "curl -sSm 10 https://example.com",
        curl_output: "curl -o output.html https://example.com",
        curl_output_long: "curl --output file.html https://example.com",
        curl_output_eq: "curl --output=file.html https://example.com",
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
        curl_header_eq: "curl -s https://example.com --header='Accept: text/html'",
        curl_multiple_headers: "curl -s https://example.com -H 'Accept: text/html' -H 'User-Agent: Bot'",
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
        curl_user_agent: "curl -A CustomBot https://example.com",
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
}

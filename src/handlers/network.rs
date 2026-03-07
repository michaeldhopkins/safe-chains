use crate::parse::{Segment, Token, WordSet};

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

const CURL_STANDALONE_SHORT: &[u8] = b"46ILNSfgksv";
const CURL_VALUED_SHORT: &[u8] = b"mw";

fn is_safe_method(method: &str) -> bool {
    CURL_SAFE_METHODS.contains(&method.to_ascii_uppercase())
}

pub fn is_safe_curl(tokens: &[Token]) -> bool {
    if tokens.len() == 2 && matches!(tokens[1].as_str(), "--help" | "-h" | "--version" | "-V") {
        return true;
    }
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

        if CURL_SAFE_VALUED.contains(t) {
            i += 2;
            continue;
        }

        if let Some(val) = t.split_value("=") {
            let flag = t.as_str().split_once('=').map_or(t.as_str(), |(k, _)| k);
            if CURL_SAFE_VALUED.contains(flag) {
                i += 1;
                continue;
            }
            if flag == "--request" {
                if !is_safe_method(val) {
                    return false;
                }
                i += 1;
                continue;
            }
            return false;
        }

        if t == "-X" || t == "--request" {
            if !tokens.get(i + 1).is_some_and(|m| is_safe_method(m)) {
                return false;
            }
            i += 2;
            continue;
        }
        if t.starts_with("-X") && t.len() > 2 {
            if !t.get(2..).is_some_and(is_safe_method) {
                return false;
            }
            i += 1;
            continue;
        }

        if t.starts_with('-') && !t.starts_with("--") && t.len() > 2 {
            let bytes = t.as_bytes();
            for (j, &b) in bytes[1..].iter().enumerate() {
                let is_last = j == bytes.len() - 2;
                if CURL_STANDALONE_SHORT.contains(&b) {
                    continue;
                }
                if is_last && CURL_VALUED_SHORT.contains(&b) {
                    i += 1;
                    break;
                }
                return false;
            }
            i += 1;
            continue;
        }

        return false;
    }
    i > 1
}

pub(crate) fn dispatch(cmd: &str, tokens: &[Token], _is_safe: &dyn Fn(&Segment) -> bool) -> Option<bool> {
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
    }

    denied! {
        curl_post: "curl -X POST https://example.com",
        curl_put: "curl -X PUT https://example.com",
        curl_delete: "curl -X DELETE https://example.com",
        curl_data: "curl -d '{\"key\":\"val\"}' https://example.com",
        curl_json: "curl --json '{\"key\":\"val\"}' https://example.com",
        curl_form: "curl -F 'file=@secret.txt' https://example.com",
        curl_output: "curl -o output.html https://example.com",
        curl_remote_name: "curl -O https://example.com/file.tar.gz",
        curl_header: "curl -H 'Authorization: Bearer token' https://example.com",
        curl_method_override: "curl -H 'X-HTTP-Method-Override: DELETE' https://example.com",
        curl_user_agent: "curl -A CustomBot https://example.com",
        curl_cookie: "curl -b 'session=abc' https://example.com",
        curl_user: "curl -u admin:pass https://example.com",
        curl_referer: "curl -e 'https://evil.com' https://example.com",
        curl_cookie_jar: "curl -c cookies.txt https://example.com",
        curl_config: "curl -K config.txt",
        curl_upload: "curl -T file.txt https://example.com",
        curl_bare: "curl",
        curl_netrc: "curl -n https://example.com",
        curl_data_long: "curl --data 'key=val' https://example.com",
        curl_form_long: "curl --form 'file=@f' https://example.com",
        curl_output_long: "curl --output file.html https://example.com",
        curl_remote_name_long: "curl --remote-name https://example.com/f.tar.gz",
        curl_dump_header: "curl -D headers.txt https://example.com",
        curl_combined_with_bad: "curl -sSo output.html https://example.com",
        curl_request_eq_post: "curl --request=POST https://example.com",
        curl_xpost: "curl -XPOST https://example.com",
        curl_get_then_data: "curl -X GET -d 'payload' https://example.com",
        curl_xget_then_header: "curl -XGET -H 'Evil: header' https://example.com",
        curl_get_then_output: "curl -X GET -o /tmp/file https://example.com",
        curl_request_eq_get_then_data: "curl --request=GET -d 'payload' https://example.com",
    }
}

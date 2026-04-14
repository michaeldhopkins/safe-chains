use crate::parse::{Token, WordSet};
use crate::verdict::{SafetyLevel, Verdict};

static CONVERT_FORMATS: WordSet = WordSet::new(&["binary1", "json", "swift", "xml1"]);

pub fn check_plutil_convert(tokens: &[Token]) -> Verdict {
    if tokens.len() == 2 && matches!(tokens[1].as_str(), "--help" | "-h") {
        return Verdict::Allowed(SafetyLevel::Inert);
    }
    let Some(fmt) = tokens.get(1) else {
        return Verdict::Denied;
    };
    if !CONVERT_FORMATS.contains(fmt) {
        return Verdict::Denied;
    }
    let mut i = 2;
    let mut saw_stdout = false;
    while i < tokens.len() {
        match tokens[i].as_str() {
            "-r" | "-s" => i += 1,
            "-o" => {
                if tokens.get(i + 1).map(Token::as_str) != Some("-") {
                    return Verdict::Denied;
                }
                saw_stdout = true;
                i += 2;
            }
            "--" => break,
            arg if arg.starts_with('-') => return Verdict::Denied,
            _ => i += 1,
        }
    }
    if saw_stdout {
        Verdict::Allowed(SafetyLevel::Inert)
    } else {
        Verdict::Denied
    }
}

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        convert_stdout_xml: "plutil -convert xml1 -o - /tmp/foo.plist",
        convert_stdout_binary: "plutil -convert binary1 -o - /tmp/foo.plist",
        convert_stdout_json: "plutil -convert json -o - /tmp/foo.plist",
        convert_stdout_swift: "plutil -convert swift -o - /tmp/foo.plist",
        convert_stdout_stdin: "plutil -convert xml1 -o -",
        convert_stdout_readable: "plutil -convert json -r -o - /tmp/foo.plist",
        convert_stdout_silent: "plutil -convert xml1 -s -o - /tmp/foo.plist",
        convert_stdout_multi_input: "plutil -convert xml1 -o - -- /tmp/a.plist /tmp/b.plist",
        convert_stdout_piped: "plutil -convert xml1 -o - /tmp/foo.plist | grep boundary",
        convert_help: "plutil -convert --help",
        convert_help_short: "plutil -convert -h",
    }

    denied! {
        convert_no_o: "plutil -convert xml1 /tmp/foo.plist",
        convert_o_to_path: "plutil -convert xml1 -o /tmp/out.plist /tmp/in.plist",
        convert_o_second_overrides: "plutil -convert xml1 -o - -o /tmp/out.plist /tmp/in",
        convert_o_path_before_stdout: "plutil -convert xml1 -o /tmp/out.plist -o - /tmp/in",
        convert_bad_format: "plutil -convert invalid -o - /tmp/foo.plist",
        convert_objc_removed: "plutil -convert objc -o - /tmp/foo.plist",
        convert_no_format: "plutil -convert",
        convert_dash_dash_as_o_value: "plutil -convert xml1 -o -- /tmp/in",
        convert_extension_flag: "plutil -convert xml1 -e plist -o - /tmp/in",
        convert_unknown_flag: "plutil -convert xml1 -o - --unknown /tmp/in",
        convert_format_only: "plutil -convert xml1",
    }
}

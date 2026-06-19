use crate::parse::{Token, WordSet};
use crate::verdict::{SafetyLevel, Verdict};

static CONVERT_FORMATS: WordSet = WordSet::new(&["binary1", "json", "swift", "xml1"]);
static EXTRACT_FORMATS: WordSet = WordSet::new(&["binary1", "json", "raw", "xml1"]);

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
    walk_modifier_args(tokens, 2)
}

pub fn check_plutil_extract(tokens: &[Token]) -> Verdict {
    if tokens.len() == 2 && matches!(tokens[1].as_str(), "--help" | "-h") {
        return Verdict::Allowed(SafetyLevel::Inert);
    }
    let Some(_keypath) = tokens.get(1) else {
        return Verdict::Denied;
    };
    let Some(fmt) = tokens.get(2) else {
        return Verdict::Denied;
    };
    if !EXTRACT_FORMATS.contains(fmt) {
        return Verdict::Denied;
    }
    walk_modifier_args(tokens, 3)
}

fn walk_modifier_args(tokens: &[Token], start: usize) -> Verdict {
    let mut i = start;
    let mut output_mode = OutputMode::InPlace;
    while i < tokens.len() {
        match tokens[i].as_str() {
            "-r" | "-s" => i += 1,
            "-o" => {
                let Some(value) = tokens.get(i + 1) else {
                    return Verdict::Denied;
                };
                if value.as_str().starts_with("--") {
                    return Verdict::Denied;
                }
                output_mode = if value.as_str() == "-" {
                    OutputMode::Stdout
                } else {
                    OutputMode::File
                };
                i += 2;
            }
            "--" => break,
            arg if arg.starts_with('-') => return Verdict::Denied,
            _ => i += 1,
        }
    }
    match output_mode {
        OutputMode::Stdout => Verdict::Allowed(SafetyLevel::Inert),
        OutputMode::File | OutputMode::InPlace => Verdict::Allowed(SafetyLevel::SafeWrite),
    }
}

enum OutputMode {
    Stdout,
    File,
    InPlace,
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
        convert_to_file: "plutil -convert json Info.plist -o out.json",
        convert_to_file_explicit: "plutil -convert xml1 -o /tmp/out.plist /tmp/in.plist",
        convert_in_place: "plutil -convert xml1 /tmp/foo.plist",
        convert_help: "plutil -convert --help",
        convert_help_short: "plutil -convert -h",
        extract_raw: "plutil -extract CFBundleVersion raw Info.plist",
        extract_json: "plutil -extract CFBundleShortVersionString json Info.plist",
        extract_to_file: "plutil -extract CFBundleURLTypes xml1 -o urls.plist Info.plist",
        extract_to_stdout: "plutil -extract CFBundleURLTypes xml1 -o - Info.plist",
        extract_help: "plutil -extract --help",
    }

    denied! {
        convert_bad_format: "plutil -convert invalid -o - /tmp/foo.plist",
        convert_objc_removed: "plutil -convert objc -o - /tmp/foo.plist",
        convert_no_format: "plutil -convert",
        convert_dash_dash_as_o_value: "plutil -convert xml1 -o -- /tmp/in",
        convert_extension_flag: "plutil -convert xml1 -e plist -o - /tmp/in",
        convert_unknown_flag: "plutil -convert xml1 -o - --unknown /tmp/in",
        extract_no_keypath: "plutil -extract",
        extract_bad_format: "plutil -extract Key bogus Info.plist",
        extract_unknown_flag: "plutil -extract Key json --evil Info.plist",
    }
}

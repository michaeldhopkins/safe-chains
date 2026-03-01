use crate::parse::{Segment, Token, WordSet};

static XARGS_FLAGS_WITH_ARG: WordSet =
    WordSet::new(&["-E", "-I", "-L", "-P", "-d", "-n", "-s"]);

static XARGS_FLAGS_NO_ARG: WordSet =
    WordSet::new(&["-0", "-p", "-r", "-t", "-x"]);

pub fn is_safe_shell(tokens: &[Token], is_safe: &dyn Fn(&Segment) -> bool) -> bool {
    if tokens.len() == 2 && (tokens[1] == "--version" || tokens[1] == "--help") {
        return true;
    }
    let Some(idx) = tokens.iter().position(|t| *t == "-c") else {
        return false;
    };
    let Some(script) = tokens.get(idx + 1) else {
        return false;
    };
    script.as_command_line().segments().iter().all(is_safe)
}

pub fn is_safe_xargs(tokens: &[Token], is_safe: &dyn Fn(&Segment) -> bool) -> bool {
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
            i += 1;
            continue;
        }
        let inner = Token::join(&tokens[i..]);
        return is_safe(&inner);
    }
    true
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    use crate::docs::CommandDoc;
    vec![
        CommandDoc::handler("bash / sh",
            "Allowed: --version, --help. Only `bash -c` / `sh -c` with a safe inner command. Scripts denied."),
        CommandDoc::handler("xargs",
            "Recursively validates the inner command. Skips xargs-specific flags (-I, -L, -n, -P, -s, -E, -d, -0, -r, -t, -p, -x)."),
    ]
}

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
        bash_version: "bash --version",
        sh_version: "sh --version",
        bash_help: "bash --help",
        sh_help: "sh --help",
        xargs_grep: "xargs grep pattern",
        xargs_cat: "xargs cat",
        xargs_with_flags: "xargs -I {} cat {}",
        xargs_zero_flag: "xargs -0 grep foo",
        xargs_npx_safe: "xargs npx @herb-tools/linter",
        xargs_find_safe: "xargs find . -name '*.py'",
        xargs_sed_safe: "xargs sed 's/foo/bar/'",
        xargs_nested_bash_safe: "xargs bash -c 'git status'",
    }

    denied! {
        bash_c_unsafe: "bash -c \"rm file\"",
        sh_c_unsafe: "sh -c \"curl https://evil.com\"",
        bash_script_denied: "bash script.sh",
        xargs_rm_denied: "xargs rm",
        xargs_curl_denied: "xargs curl",
        xargs_npx_unsafe: "xargs npx cowsay",
        xargs_sed_inplace_denied: "xargs sed -i 's/foo/bar/'",
        xargs_find_delete_denied: "xargs find . -delete",
        xargs_sort_output_denied: "xargs sort -o out.txt",
        xargs_nested_bash_chain_denied: "xargs bash -c 'ls && rm -rf /'",
    }
}

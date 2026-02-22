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
    use crate::docs::{CommandDoc, DocKind};
    vec![
        CommandDoc {
            name: "bash / sh",
            kind: DocKind::Handler,
            description: "Allowed: --version, --help. Only `bash -c` / `sh -c` with a safe inner command. Scripts denied.",
        },
        CommandDoc {
            name: "xargs",
            kind: DocKind::Handler,
            description: "Recursively validates the inner command. Skips xargs-specific flags (-I, -L, -n, -P, -s, -E, -d, -0, -r, -t, -p, -x).",
        },
    ]
}

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    #[test]
    fn bash_c_safe() {
        assert!(check("bash -c \"grep foo file\""));
    }

    #[test]
    fn bash_c_pipe() {
        assert!(check("bash -c \"cat file | head -5\""));
    }

    #[test]
    fn bash_c_unsafe() {
        assert!(!check("bash -c \"rm file\""));
    }

    #[test]
    fn sh_c_safe() {
        assert!(check("sh -c \"ls -la\""));
    }

    #[test]
    fn sh_c_unsafe() {
        assert!(!check("sh -c \"curl https://evil.com\""));
    }

    #[test]
    fn bash_version() {
        assert!(check("bash --version"));
    }

    #[test]
    fn sh_version() {
        assert!(check("sh --version"));
    }

    #[test]
    fn bash_help() {
        assert!(check("bash --help"));
    }

    #[test]
    fn sh_help() {
        assert!(check("sh --help"));
    }

    #[test]
    fn bash_script_denied() {
        assert!(!check("bash script.sh"));
    }

    #[test]
    fn xargs_grep() {
        assert!(check("xargs grep pattern"));
    }

    #[test]
    fn xargs_cat() {
        assert!(check("xargs cat"));
    }

    #[test]
    fn xargs_with_flags() {
        assert!(check("xargs -I {} cat {}"));
    }

    #[test]
    fn xargs_rm_denied() {
        assert!(!check("xargs rm"));
    }

    #[test]
    fn xargs_curl_denied() {
        assert!(!check("xargs curl"));
    }

    #[test]
    fn xargs_zero_flag() {
        assert!(check("xargs -0 grep foo"));
    }

    #[test]
    fn xargs_npx_safe() {
        assert!(check("xargs npx @herb-tools/linter"));
    }

    #[test]
    fn xargs_npx_unsafe() {
        assert!(!check("xargs npx cowsay"));
    }

    #[test]
    fn xargs_find_safe() {
        assert!(check("xargs find . -name '*.py'"));
    }

    #[test]
    fn xargs_sed_safe() {
        assert!(check("xargs sed 's/foo/bar/'"));
    }

    #[test]
    fn xargs_sed_inplace_denied() {
        assert!(!check("xargs sed -i 's/foo/bar/'"));
    }

    #[test]
    fn xargs_find_delete_denied() {
        assert!(!check("xargs find . -delete"));
    }

    #[test]
    fn xargs_sort_output_denied() {
        assert!(!check("xargs sort -o out.txt"));
    }

    #[test]
    fn xargs_nested_bash_chain_denied() {
        assert!(!check("xargs bash -c 'ls && rm -rf /'"));
    }

    #[test]
    fn xargs_nested_bash_safe() {
        assert!(check("xargs bash -c 'git status'"));
    }
}

use std::collections::HashSet;
use std::sync::LazyLock;

use crate::parse::split_outside_quotes;

static XARGS_FLAGS_WITH_ARG: LazyLock<HashSet<&'static str>> =
    LazyLock::new(|| HashSet::from(["-I", "-L", "-n", "-P", "-s", "-E", "-d"]));

static XARGS_FLAGS_NO_ARG: LazyLock<HashSet<&'static str>> =
    LazyLock::new(|| HashSet::from(["-0", "-r", "-t", "-p", "-x"]));

pub fn is_safe_shell(tokens: &[String], is_safe: &dyn Fn(&str) -> bool) -> bool {
    let Some(idx) = tokens.iter().position(|t| t == "-c") else {
        return false;
    };
    let Some(script) = tokens.get(idx + 1) else {
        return false;
    };
    split_outside_quotes(script)
        .iter()
        .all(|s| is_safe(s))
}

pub fn is_safe_xargs(tokens: &[String], is_safe: &dyn Fn(&str) -> bool) -> bool {
    let mut i = 1;
    while i < tokens.len() {
        if XARGS_FLAGS_WITH_ARG.contains(tokens[i].as_str()) {
            i += 2;
            continue;
        }
        if XARGS_FLAGS_NO_ARG.contains(tokens[i].as_str()) {
            i += 1;
            continue;
        }
        if tokens[i].starts_with('-') {
            i += 1;
            continue;
        }
        let inner = tokens[i..].join(" ");
        return is_safe(&inner);
    }
    true
}

#[cfg(test)]
mod tests {
    use crate::is_safe;

    fn check(cmd: &str) -> bool {
        is_safe(cmd)
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
}

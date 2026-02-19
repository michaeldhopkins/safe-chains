use std::collections::HashSet;
use std::sync::LazyLock;

use crate::parse::has_flag;

static FIND_DANGEROUS_FLAGS: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    HashSet::from([
        "-delete",
        "-exec",
        "-execdir",
        "-ok",
        "-okdir",
        "-fls",
        "-fprint",
        "-fprint0",
        "-fprintf",
    ])
});

pub fn is_safe_find(tokens: &[String]) -> bool {
    !tokens[1..].iter().any(|t| FIND_DANGEROUS_FLAGS.contains(t.as_str()))
}

pub fn is_safe_sed(tokens: &[String]) -> bool {
    !has_flag(tokens, "-i", Some("--in-place"))
}

pub fn is_safe_sort(tokens: &[String]) -> bool {
    !has_flag(tokens, "-o", Some("--output"))
}

pub fn is_safe_yq(tokens: &[String]) -> bool {
    !has_flag(tokens, "-i", Some("--inplace"))
}

pub fn is_safe_xmllint(tokens: &[String]) -> bool {
    !tokens[1..].iter().any(|t| t == "--output" || t.starts_with("--output="))
}

#[cfg(test)]
mod tests {
    use crate::is_safe;

    fn check(cmd: &str) -> bool {
        is_safe(cmd)
    }

    #[test]
    fn find_name() {
        assert!(check("find . -name '*.rb'"));
    }

    #[test]
    fn find_type_name() {
        assert!(check("find . -type f -name '*.py'"));
    }

    #[test]
    fn find_maxdepth() {
        assert!(check("find /tmp -maxdepth 2"));
    }

    #[test]
    fn find_print() {
        assert!(check("find . -name '*.log' -print"));
    }

    #[test]
    fn find_print0() {
        assert!(check("find . -name '*.log' -print0"));
    }

    #[test]
    fn find_delete_denied() {
        assert!(!check("find . -name '*.tmp' -delete"));
    }

    #[test]
    fn find_exec_denied() {
        assert!(!check("find . -exec rm {} \\;"));
    }

    #[test]
    fn find_execdir_denied() {
        assert!(!check("find . -execdir cat {} \\;"));
    }

    #[test]
    fn find_ok_denied() {
        assert!(!check("find . -ok rm {} \\;"));
    }

    #[test]
    fn find_okdir_denied() {
        assert!(!check("find . -okdir rm {} \\;"));
    }

    #[test]
    fn find_exec_grep_denied() {
        assert!(!check("find . -name '*.py' -exec grep pattern {} +"));
    }

    #[test]
    fn find_type_delete_denied() {
        assert!(!check("find . -type f -name '*.bak' -delete"));
    }

    #[test]
    fn find_fprint_denied() {
        assert!(!check("find . -fprint /tmp/list.txt"));
    }

    #[test]
    fn find_fprint0_denied() {
        assert!(!check("find . -fprint0 /tmp/list.txt"));
    }

    #[test]
    fn find_fls_denied() {
        assert!(!check("find . -fls /tmp/list.txt"));
    }

    #[test]
    fn find_fprintf_denied() {
        assert!(!check("find . -fprintf /tmp/list.txt '%p'"));
    }

    #[test]
    fn sed_substitute() {
        assert!(check("sed 's/foo/bar/'"));
    }

    #[test]
    fn sed_n_flag() {
        assert!(check("sed -n 's/foo/bar/p'"));
    }

    #[test]
    fn sed_e_flag() {
        assert!(check("sed -e 's/foo/bar/' -e 's/baz/qux/'"));
    }

    #[test]
    fn sed_extended() {
        assert!(check("sed -E 's/[0-9]+/NUM/g'"));
    }

    #[test]
    fn sed_inplace_denied() {
        assert!(!check("sed -i 's/foo/bar/' file.txt"));
    }

    #[test]
    fn sed_in_place_long_denied() {
        assert!(!check("sed --in-place 's/foo/bar/' file.txt"));
    }

    #[test]
    fn sed_inplace_backup_denied() {
        assert!(!check("sed -i.bak 's/foo/bar/' file.txt"));
    }

    #[test]
    fn sed_ni_combined_denied() {
        assert!(!check("sed -ni 's/foo/bar/p' file.txt"));
    }

    #[test]
    fn sed_in_combined_denied() {
        assert!(!check("sed -in 's/foo/bar/' file.txt"));
    }

    #[test]
    fn sed_in_place_eq_denied() {
        assert!(!check("sed --in-place=.bak 's/foo/bar/' file.txt"));
    }

    #[test]
    fn sort_basic() {
        assert!(check("sort file.txt"));
    }

    #[test]
    fn sort_reverse() {
        assert!(check("sort -r file.txt"));
    }

    #[test]
    fn sort_n_u() {
        assert!(check("sort -n -u file.txt"));
    }

    #[test]
    fn sort_field() {
        assert!(check("sort -t: -k2 /etc/passwd"));
    }

    #[test]
    fn sort_output_denied() {
        assert!(!check("sort -o output.txt file.txt"));
    }

    #[test]
    fn sort_output_long_denied() {
        assert!(!check("sort --output=result.txt file.txt"));
    }

    #[test]
    fn sort_output_long_space_denied() {
        assert!(!check("sort --output result.txt file.txt"));
    }

    #[test]
    fn sort_rno_combined_denied() {
        assert!(!check("sort -rno sorted.txt file.txt"));
    }

    #[test]
    fn yq_read() {
        assert!(check("yq '.key' file.yaml"));
    }

    #[test]
    fn yq_eval() {
        assert!(check("yq eval '.metadata.name' deployment.yaml"));
    }

    #[test]
    fn yq_inplace_denied() {
        assert!(!check("yq -i '.key = \"value\"' file.yaml"));
    }

    #[test]
    fn yq_inplace_long_denied() {
        assert!(!check("yq --inplace '.key = \"value\"' file.yaml"));
    }

    #[test]
    fn xmllint_read() {
        assert!(check("xmllint --xpath '//name' file.xml"));
    }

    #[test]
    fn xmllint_format() {
        assert!(check("xmllint --format file.xml"));
    }

    #[test]
    fn xmllint_output_denied() {
        assert!(!check("xmllint --output result.xml file.xml"));
    }

    #[test]
    fn xmllint_output_eq_denied() {
        assert!(!check("xmllint --output=result.xml file.xml"));
    }
}

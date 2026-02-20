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

static AWK_DANGEROUS: &[&str] = &["system", "getline", "|", ">", ">>"];

pub fn is_safe_awk(tokens: &[String]) -> bool {
    if has_flag(tokens, "-f", None) {
        return false;
    }
    for token in &tokens[1..] {
        if token.starts_with('-') {
            continue;
        }
        if AWK_DANGEROUS.iter().any(|d| token.contains(d)) {
            return false;
        }
    }
    true
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    use crate::docs::{CommandDoc, DocKind};
    vec![
        CommandDoc {
            name: "find",
            kind: DocKind::Handler,
            description: "Safe unless dangerous flags: -delete, -exec, -execdir, -ok, -okdir, -fls, -fprint, -fprint0, -fprintf.",
        },
        CommandDoc {
            name: "sed",
            kind: DocKind::Handler,
            description: "Safe unless -i/--in-place flag.",
        },
        CommandDoc {
            name: "sort",
            kind: DocKind::Handler,
            description: "Safe unless -o/--output flag.",
        },
        CommandDoc {
            name: "yq",
            kind: DocKind::Handler,
            description: "Safe unless -i/--inplace flag.",
        },
        CommandDoc {
            name: "awk / gawk / mawk / nawk",
            kind: DocKind::Handler,
            description: "Safe unless program contains system, getline, |, >, >>, or -f flag (file-based program).",
        },
        CommandDoc {
            name: "xmllint",
            kind: DocKind::Handler,
            description: "Safe unless --output flag.",
        },
    ]
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

    #[test]
    fn awk_print_field() {
        assert!(check("awk '{print $1}' file.txt"));
    }

    #[test]
    fn awk_print_multiple_fields() {
        assert!(check("awk '{print $1, $3}' file.txt"));
    }

    #[test]
    fn awk_field_separator() {
        assert!(check("awk -F: '{print $1}' /etc/passwd"));
    }

    #[test]
    fn awk_pattern() {
        assert!(check("awk '/error/ {print $0}' log.txt"));
    }

    #[test]
    fn awk_nr() {
        assert!(check("awk 'NR==5' file.txt"));
    }

    #[test]
    fn awk_begin_end_safe() {
        assert!(check("awk 'BEGIN{n=0} {n++} END{print n}' file.txt"));
    }

    #[test]
    fn awk_system_denied() {
        assert!(!check("awk 'BEGIN{system(\"rm -rf /\")}'"));
    }

    #[test]
    fn awk_getline_denied() {
        assert!(!check("awk '{getline line < \"/etc/shadow\"; print line}'"));
    }

    #[test]
    fn awk_pipe_output_denied() {
        assert!(!check("awk '{print $0 | \"mail user@host\"}'"));
    }

    #[test]
    fn awk_redirect_denied() {
        assert!(!check("awk '{print $0 > \"output.txt\"}'"));
    }

    #[test]
    fn awk_append_denied() {
        assert!(!check("awk '{print $0 >> \"output.txt\"}'"));
    }

    #[test]
    fn awk_file_program_denied() {
        assert!(!check("awk -f script.awk data.txt"));
    }

    #[test]
    fn gawk_safe() {
        assert!(check("gawk '{print $2}' file.txt"));
    }

    #[test]
    fn gawk_system_denied() {
        assert!(!check("gawk 'BEGIN{system(\"rm\")}'"));
    }

    #[test]
    fn awk_netstat_pipeline() {
        assert!(check("awk '{print $6}'"));
    }
}

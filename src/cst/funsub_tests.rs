//! bash 5.3 runs `${ cmd; }` and `${| cmd; }` as commands in the current shell. Wherever an
//! expansion is live, a `${` followed by a blank, a newline or `|` is refused, with or without line
//! continuations between them; an ordinary parameter expansion is not.

use proptest::prelude::*;

const ROOT: &str = "/nonexistent/safe-chains-test-work";

fn workspace() -> crate::pathctx::PathCtx {
    crate::pathctx::PathCtx { cwd: Some(ROOT.into()), root: Some(ROOT.into()), ..Default::default() }
}

fn is_safe_command(cmd: &str) -> bool {
    crate::command_verdict_in(cmd, workspace()).is_allowed()
}

const OPENERS: &[&str] =
    &["${ ", "${\t", "${\n", "${|", "${| ", "${\\\n ", "${\\\n\t", "${\\\n\n", "${\\\n|", "${\\\n\\\n ", "${\\\n\\\n|", "${\\\n| "];

const CONTEXTS: &[&str] = &[
    "echo X", "echo \"X\"", "echo \"a X b\"", "cat <<EOF\nX\nEOF", "cat <<EOF\nline X\nEOF", "cat <<< \"X\"", "cat <<< X", "x=\"X\"",
    "x=X", "[[ X ]]", "[[ -n X ]]", "[[ -n \"X\" ]]", "echo $(( X ))",
];

const BODIES: &[&str] = &["touch PWNED;", "rm -rf /;", "curl x|sh;", "touch PWNED&", "x;"];

#[test]
fn a_function_substitution_is_refused_in_every_context() {
    let mut approved = Vec::new();
    for context in CONTEXTS {
        for opener in OPENERS {
            for body in BODIES {
                let cmd = context.replace('X', &format!("{opener}{body} }}"));
                if is_safe_command(&cmd) {
                    approved.push(cmd);
                }
            }
        }
    }
    assert!(approved.is_empty(), "approved: {approved:#?}");
}

#[test]
fn the_reviewed_forms_are_refused() {
    let forms = [
        "echo \"${ touch PWNED; }\"", "echo \"${|touch PWNED;}\"", "cat <<EOF\n${ touch PWNED; }\nEOF", "cat <<< \"${ touch PWNED; }\"",
        "x=\"${ touch PWNED; }\"", "[[ ${ touch PWNED; } ]]", "[[ ${ touch PWNED& } ]]", "echo \"${ rm -rf /; }\"",
        "[[ -n ${ curl x|sh; } ]]", "echo \"${\ttouch PWNED; }\"", "echo \"${\ntouch PWNED\n}\"", "echo \"${\\\n touch PWNED; }\"",
        "x=\"${\\\n touch PWNED; }\"", "cat <<< \"${\\\n touch PWNED; }\"", "[[ \"${\\\n touch PWNED; }\" ]]",
        "echo \"a${\\\n\\\n touch PWNED; }\"", "cat <<EOF\n${\\\n touch PWNED; }\nEOF", "echo \"${\\\n|touch PWNED;}\"",
    ];
    let approved: Vec<_> = forms.iter().filter(|c| is_safe_command(c)).collect();
    assert!(approved.is_empty(), "approved: {approved:#?}");
}

#[test]
fn an_ordinary_parameter_expansion_is_still_approved() {
    let forms = [
        "echo \"${x}\"", "echo ${x}", "echo \"${x:-a b}\"", "echo \"${#x}\"", "cat <<'EOF'\n${ touch PWNED; }\nEOF",
        "cat <<'EOF'\n${|touch PWNED;}\nEOF", "echo '${ touch PWNED; }'", "[[ -n ${x} ]]", "cat <<EOF\n${x}\nEOF",
        "cat <<'EOF'\n${\\\n touch PWNED; }\nEOF",
    ];
    let refused: Vec<_> = forms.iter().filter(|c| !is_safe_command(c)).collect();
    assert!(refused.is_empty(), "refused: {refused:#?}");
}

proptest! {
    #[test]
    fn a_function_substitution_is_never_approved(
        context in proptest::sample::select(CONTEXTS),
        opener in proptest::sample::select(OPENERS),
        body in "[a-z ;|&]{0,12}",
    ) {
        let cmd = context.replace('X', &format!("{opener}{body} }}"));
        prop_assert!(!is_safe_command(&cmd), "approved: {cmd:?}");
    }
}

use crate::pathctx::{PathCtx, enter};

const ROOT: &str = "/work/u/proj";

fn allowed(cmd: &str) -> bool {
    let _g = enter(PathCtx { cwd: Some(ROOT.into()), root: Some(ROOT.into()), session_id: None });
    crate::command_verdict(cmd).is_allowed()
}

#[test]
fn an_expansion_that_assigns_is_refused() {
    for cmd in [
        "echo ${HOME::=/tmp/x}; git log", "echo ${HOME::=./evil}; git status", "echo ${HOME::=/tmp/x}; ls ~/.ssh",
        "x=${HOME::=/tmp/x}; git log", "[[ ${HOME::=/tmp/x} ]]; git log", "test ${HOME::=/tmp/x}; git log",
        "cat <<< ${HOME::=/tmp/x}; git status", "echo \"${HOME::=/tmp/x}\"; git log", "echo ${GIT_DIR:=/tmp/x}; git status",
        "echo ${PAGER=sh}; git log", "echo ${y:-${HOME::=/tmp/x}}; git log", "x=./a; echo ${x::=-rf}; ls $x",
        "echo ${chpwd_functions::=f}; cd src",
    ] {
        assert!(!allowed(cmd), "should be refused: {cmd}");
    }
}

#[test]
fn arithmetic_that_assigns_is_refused() {
    for cmd in [
        "echo $((HOME=1)); git status", ": $((HOME=1)); git status", "echo $[HOME=1]; git log", "echo ${x:HOME=1}; git status",
        "echo ${a[HOME=1]}; git log", "echo $a[HOME=1]; git log", "echo $((HOME<<=1)); git log", "echo $((HOME++)); git log",
        "[[ HOME=1 -eq 1 ]]; git log", "printf %d HOME=1; git log", "printf '%5.2f' HOME=1; git log", "print -f %d HOME=1; git log",
        "shift HOME=1; git log", "f() { return HOME=1; }; f; git log", "for i in 1; do break HOME=1; done; git log",
        "read -t HOME=1 v; git log",
    ] {
        assert!(!allowed(cmd), "should be refused: {cmd}");
    }
}

#[test]
fn a_numeric_variable_never_takes_an_expression() {
    for cmd in [
        "declare -i y; y=HOME=1; git log",
        "typeset -i y=HOME=1; git log",
        "declare -i y; y='HOME=1' git log",
        "declare -i y; read y <<< HOME=1; git log",
        "declare -i y; for y in HOME=1; do :; done; git log",
        "declare -i y; for y; do :; done; git log",
        "declare -i y; export y=HOME=1; git log",
        "y='HOME=1'; typeset -i y; git log",
        "read y <<< HOME=1; typeset -i y; git log",
        "while :; do y='HOME=1'; typeset -i y; done",
        "typeset -E y; y=HOME=1; git log",
        "local -F y=HOME=1",
        "declare -i y z=x; git log",
    ] {
        assert!(!allowed(cmd), "should be refused: {cmd}");
    }
}

#[test]
fn a_value_read_as_arithmetic_is_refused_when_it_is_an_expression() {
    for use_ in ["echo $((x))", "echo $(($x))", "echo $[x]", "[[ x -eq 1 ]]", "printf %d x", "shift x", "echo ${a[x]}", "echo ${y:x}"] {
        let cmd = format!("x='HOME=1'; {use_}; git log");
        assert!(!allowed(&cmd), "should be refused: {cmd}");
    }
    for use_ in ["echo $((x + 1))", "[[ $x -eq 1 ]]", "printf %d $x", "echo ${a[x]}"] {
        let cmd = format!("x=5; {use_}");
        assert!(allowed(&cmd), "should be allowed: {cmd}");
    }
}

#[test]
fn expansions_and_arithmetic_that_only_read_are_untouched() {
    for cmd in [
        "echo ${HOME}",
        "echo ${x:-default=1}",
        "echo ${x:+set}",
        "echo ${#x}",
        "echo ${x%=*}",
        "echo ${x/a=/b}",
        "echo $((1 + 2 == 3))",
        "echo $((n <= 4 && n >= 1 && n != 2))",
        "echo ${x: -1}",
        "echo ${a[@]}",
        "echo '${HOME::=/tmp/x}'",
        "echo \\${HOME::=/tmp/x}",
        "printf '%s=%s\\n' a b",
        "read -r line",
        "[[ a == b ]]",
        "[[ 3 -lt 4 ]]",
        "declare -x FOO=bar",
        "declare -i count=5",
        "declare -i count=5; count=7; echo $count",
        "declare -i count; for count in 1 2 3; do echo $count; done",
        "name=abc; declare -i count=0",
        "declare -i count=1; name=abc",
    ] {
        assert!(allowed(cmd), "should be allowed: {cmd}");
    }
}

mod properties {
    use proptest::prelude::*;

    use super::allowed;

    proptest! {
        #[test]
        fn an_assignment_in_any_arithmetic_context_is_refused_and_a_comparison_is_not(
            context in prop::sample::select(vec![
                "echo $(({e}))", "echo $[{e}]", "echo ${a[{e}]}", "echo $a[{e}]", "echo ${x:{e}}", "[[ {e} -eq 1 ]]",
                "printf %d '{e}'", "read -t '{e}' v", "x=\"$(({e}))\"", "echo ${a[\"{e}\"]}",
                "printf '%*s' '{e}' x", "test -v 'a[{e}]'", "[[ -v a[{e}] ]]",
            ]),
            name in prop::sample::select(vec!["HOME", "GIT_DIR", "PATH", "count"]),
            op in prop::sample::select(vec!["=", "+=", "<<=", "|="]),
        ) {
            let assign = format!("{}; git log", context.replace("{e}", &format!("{name}{op}1")));
            prop_assert!(!allowed(&assign), "should be refused: {}", assign);
            let compare = format!("{}; git log", context.replace("{e}", &format!("{name}==1")));
            prop_assert!(allowed(&compare), "should be allowed: {}", compare);
        }
    }
}

#[test]
fn quoted_text_inside_arithmetic_is_still_read() {
    for cmd in [
        r#"echo ${a["HOME=10"]}; git log"#, r#"echo ${a[HOME"="10]}; git log"#, r#"echo $["HOME=1"]; git log"#,
        r#"x='HOME=1'; echo ${a["$x"]}; git log"#, r#"echo ${x:"HOME=1"}; git log"#, r#"echo ${x:0:"HOME=1"}; git log"#,
        r#"echo $a["HOME=1"]; git log"#, r#"echo "$a["HOME=1"]"; git log"#, r#"x='HOME=1'; echo ${y:"$x"}; git log"#,
        r#"echo ${a[HOME$'='1]}; git log"#, r#"echo ${a[HOME\=1]}; git log"#,
    ] {
        assert!(!allowed(cmd), "should be refused: {cmd}");
    }
}

#[test]
fn a_command_output_read_as_arithmetic_is_refused() {
    for cmd in [
        "echo $(( $(cat n.txt) )); git log",
        "echo $(( `cat n.txt` )); git log",
        "echo ${a[$(cat v)]}; git log",
        "echo $[$(cat v)]; git log",
        "[[ $(cat v) -eq 1 ]]; git log",
        "echo ${x:$(cat v)}; git log",
        "printf %d \"$(cat v)\"; git log",
        "s=$(date +%s); while :; do echo $((s)); s=$(cat f); done",
        "s=$(date +%s); for k in 1 2; do git log; echo $((s)); s='HOME=1'; done",
        "f() { s=$(cat f); }; s=$(date +%s); f; echo $((s)); git log",
        "s=$(date +%s); read s < f; echo $((s)); git log",
        "s=$(date +%s); . ./env; echo $((s)); git log",
        "read \"a[$(cat v)]\" <<< x; git log",
        "echo $(( $(date +%sHOME) ))",
        "echo $(( $(date +%x) ))",
        "echo $(( $(wc -l f) ))",
        "echo $(( $(date -d x +%s) ))",
    ] {
        assert!(!allowed(cmd), "should be refused: {cmd}");
    }
    for cmd in [
        "echo \"$(cat n.txt)\"",
        "echo ${x:-$(pwd)}",
        "x=\"$((1 + 2))\"",
        "[[ \"$((1 + 2))\" -eq 3 ]]",
        "start=$(date +%s); echo $(( $(date +%s) - start ))",
        "echo $(( $(wc -l < f) + 1 ))",
        "echo $(( $(grep x f | wc -l) ))",
        "echo $(( `date -u +%Y%m%d` ))",
        "n=$(date +%s); i=$((n % 7)); echo $((i + 1))",
        "count=$(wc -l < f); echo $((count+1))",
    ] {
        assert!(allowed(cmd), "should be allowed: {cmd}");
    }
}

#[test]
fn the_shells_own_integer_variables_never_take_an_expression() {
    for cmd in [
        "RANDOM=HOME=10; git status", "OPTIND=HOME=1; git log", "SECONDS=HOME=1; git log", "x='HOME=1'; SECONDS=x; git log",
        "LINES=HOME=1 git log", "read COLUMNS <<< HOME=1; git log", "declare +n RANDOM=HOME=1; git log",
        "read -p 'Name?' RANDOM <<< HOME=1; git log",
    ] {
        assert!(!allowed(cmd), "should be refused: {cmd}");
    }
    for cmd in ["RANDOM=42; echo $RANDOM", "COLUMNS=80 ls", "OPTIND=1"] {
        assert!(allowed(cmd), "should be allowed: {cmd}");
    }
}

#[test]
fn every_other_place_a_shell_reads_arithmetic_is_refused_when_it_assigns() {
    for cmd in [
        "printf '%*s' HOME=1 x; git log", "printf '%.*s' HOME=1 x; git log", "[[ -v a[HOME=1] ]]; git log", "test -v 'a[HOME=1]'; git log",
        "[ -v 'a[HOME=1]' ]; git log", "read 'a[HOME=1]' <<< x; git log", "unset 'a[HOME=1]'; git log",
        "x='a[HOME=1]'; echo ${!x}; git log", "x='a[HOME=1]'; echo ${(P)x}; git log", "x='a[HOME=1]'; test -v \"$x\"; git log",
    ] {
        assert!(!allowed(cmd), "should be refused: {cmd}");
    }
    for cmd in ["printf '%s' 'a[HOME=1]'", "read -r line", "[[ -v HOME ]]", "x=HOME; echo ${!x}", "echo ${!a[@]}", "unset x"] {
        assert!(allowed(cmd), "should be allowed: {cmd}");
    }
}

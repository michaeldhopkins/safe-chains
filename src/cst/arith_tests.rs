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
        "n=3; echo $((n <= 4 && n >= 1 && n != 2))",
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
        "declare -i count=0; for count in 1 2 3; do echo $count; done",
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
            // Only a name holding a known number reads as one.
            let compare = format!("count=5; {}; git log", context.replace("{e}", "1==count"));
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
        "echo $(( $(seq -f HOME=%g 1) ))",
        "for i in $(seq 1 5) HOME=1; do echo $((i)); done; git log",
        "f() { i='HOME=1'; }; for i in $(seq 1 5); do f; echo $((i)); done; git log",
        "for i in $(seq 1 5); do :; done; i='HOME=1'; echo $((i)); git log",
        "for i in $(seq 3); do while :; do echo $((i)); i='HOME=1'; done; done; git log",
        "i=5; for i in *; do echo $((i)); done; git log",
        "for i in {1..3} HOME=1; do echo $((i)); done; git log",
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
        "for i in $(seq 1 5); do echo $((i+1)); done",
        "for i in {1..3}; do echo $((i*2)); done",
        "for i in 1 $(seq 3); do echo $((i * 2)); done",
    ] {
        assert!(allowed(cmd), "should be allowed: {cmd}");
    }
}

#[test]
fn a_counter_kept_in_a_loop_is_read_as_the_number_it_is() {
    for cmd in [
        "i=0; while [ $i -lt 3 ]; do echo $i; i=$((i+1)); done",
        "n=5; while [ $n -gt 0 ]; do echo $n; n=$((n-1)); done",
        "sum=0; for n in 1 2 3; do sum=$((sum+n)); done; echo $sum",
        "i=0; for x in a b c; do i=$((i+1)); echo $i; done",
        "declare -i c=0; echo $((c+1))",
        "typeset -i n=5; echo $((n))",
        "f() { n=0; while :; do n=$((n+1)); done; }; f",
    ] {
        assert!(allowed(cmd), "should be allowed: {cmd}");
    }
    for cmd in [
        "i=0; while :; do echo $((i)); i='HOME=1'; done; git log",
        "f() { echo $((n)); }; n=$(date +%s); f; n=$(cat v); f; git log",
        "n=0; for k in 1 2; do echo $((n)); { n='HOME=1'; }; done; git log",
        "n=0; while :; do echo $((n)); read n; done; git log",
        "n=0; while :; do echo $((n)); . ./env; done; git log",
        "n=0; while :; do echo $((n)); declare n=x; done; git log",
        "declare n=0 n=x; while :; do echo $((n)); done; git log",
    ] {
        assert!(!allowed(cmd), "should be refused: {cmd}");
    }
}

#[test]
fn a_value_set_without_an_assignment_is_never_counted_as_a_number() {
    for cmd in [
        "n=$(date +%s); for k in 1 2; do echo $((n)); declare n+='+(HOME=1)'; done; git log",
        "n=$(date +%s); for k in 1 2; do echo $((n)); export n+='+(HOME=1)'; done; git log",
        "read < f; echo $((REPLY)); git log",
        "while read; do echo $((REPLY)); done < f; git log",
        "REPLY=$((0)); for k in 1 2; do echo $((REPLY)); read < f; done; git log",
        "read -ran < f; echo $((n)); git log",
        "n=$(date +%s); for k in 1 2; do echo $((n)); read -ran < f; done; git log",
        "echo HOME=1; echo $((_)); git log",
        "[[ 'HOME=1' =~ .* ]]; echo $((BASH_REMATCH)); git log",
        "[[ 'HOME=1' =~ .* ]]; echo $((MATCH)); git log",
        "f() { echo $(( $1 )); }; f 'HOME=1'; git log",
        "f() { echo $(( ${1} )); }; f 'HOME=1'; git log",
        "f() { echo $(( $@ )); }; f 'HOME=1'; git log",
        "f() { echo $(( $* )); }; f 1 'HOME=1'; git log",
        "f() { [[ $2 -eq 1 ]]; }; f 1 'HOME=1'; git log",
    ] {
        assert!(!allowed(cmd), "should be refused: {cmd}");
    }
    for cmd in ["f() { echo $(( $1 + 1 )); }; f 5", "f() { echo $(( $@ )); }; f 5", "declare n=0; echo $((n + 1))"] {
        assert!(allowed(cmd), "should be allowed: {cmd}");
    }
}

#[test]
fn the_explanation_refuses_a_number_contradicted_later_in_the_line() {
    let _g = enter(PathCtx { cwd: Some(ROOT.into()), root: Some(ROOT.into()), session_id: None });
    let cmd = "i=0; while :; do echo $((i)); i='HOME=1'; done; git log";
    assert!(!crate::cst::explain(cmd).is_allowed(), "should be refused: {cmd}");
    assert!(crate::cst::explain("i=0; while :; do echo $((i)); i=$((i+1)); done").is_allowed());
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
        "read {x,'a[HOME=1]'} < /dev/null; git log", "test {-v,'b[HOME=1]'}; git log", "[ {-v,'b[HOME=1]'} ]; git log",
        "printf %<->d HOME=1; git log", "printf %5<->d HOME=1; git log", "printf %.<->f HOME=1; git log", "test -v a*; git log",
        "test ?v 'a[HOME=1]'; git log", "[ -? 'a[HOME=1]' ]; git log", "test -* 'a[HOME=1]'; git log", "test *; git log",
        "test $(cat f); git log", "test ?\"v\" 'a[HOME=1]'; git log", "x=$(cat f); test $x; git log", "[ $x ]; git log",
        "test $$? 'a[HOME=1]'; git log", "test $* x; git log", "test -n y -a `cat f`; git log", "o=-v; test $o 'a[HOME=1]'; git log",
        "o=-v; [ \"$o\" 'a[HOME=1]' ]; git log", "test $(echo -v) 'a[HOME=1]'; git log", "test \"$1\" 'a[HOME=1]'; git log",
        "[ * ]; git log", "test -n x -a *; git log", "test *[!/]; git log", "test *.[ch]; git log", "[ -v a? ]; git log",
        "printf ~+ HOME=1; git log", "printf ~1 HOME=1; git log", "[ -v 'a[HOME=1]' ]; git log", "read 'a[HOME=1]' <<< x; git log",
        "unset 'a[HOME=1]'; git log", "x='a[HOME=1]'; echo ${!x}; git log", "x='a[HOME=1]'; echo ${(P)x}; git log",
        "x='a[HOME=1]'; test -v \"$x\"; git log",
    ] {
        assert!(!allowed(cmd), "should be refused: {cmd}");
    }
    for cmd in ["printf '%s' 'a[HOME=1]'", "read -r line", "[[ -v HOME ]]", "x=HOME; echo ${!x}", "echo ${!a[@]}", "unset x"] {
        assert!(allowed(cmd), "should be allowed: {cmd}");
    }
}

#[test]
fn an_expansion_that_runs_what_a_variable_holds_is_refused() {
    for cmd in ["x='$(touch f)'; echo ${x@P}", "echo ${x@P}", "x='$(touch f)'; echo ${(e)x}", "echo \"${(e)x}\""] {
        assert!(!allowed(cmd), "should be refused: {cmd}");
    }
    for cmd in ["echo ${x@Q}", "echo ${x@U}"] {
        assert!(allowed(cmd), "should be allowed: {cmd}");
    }
}

#[test]
fn a_descriptor_named_by_a_variable_is_refused() {
    for cmd in ["echo hi {HOME}>/dev/null; git status", "exec {fd}>/dev/null", ": {GIT_DIR}</dev/null; git log"] {
        assert!(!allowed(cmd), "should be refused: {cmd}");
    }
    for cmd in ["echo {a} b", "echo {a,b} > /dev/null"] {
        assert!(allowed(cmd), "should be allowed: {cmd}");
    }
}

#[test]
fn a_name_the_line_never_set_is_not_read_as_a_number() {
    for cmd in [
        "aarch64='HOME=1'; echo $(( HOSTTYPE )); git log",
        "aarch64='HOME=1'; echo $(( $HOSTTYPE )); git log",
        "aarch64='HOME=1'; [[ HOSTTYPE -eq 0 ]]; git log",
        "apple='HOME=1'; echo $(( VENDOR )); git log",
        "f() { HOME=1; }; echo $(( functions[f] )); git log",
        "f() { HOME=1; }; echo $(( ${functions[f]} )); git log",
        "f() { HOME=1; }; [[ functions[f] -eq 0 ]]; git log",
        "f() { echo $(( a )); }; a='HOME=1' f; git log",
        "a='HOME=1' :; echo $(( a )); git log",
        "a='HOME=1' export b; echo $(( a )); git log",
        "f() { echo $(( argv[1] )); }; f 'HOME=1'; git log",
        "f() { echo $(( $argv )); }; f 'HOME=1'; git log",
        "f() { echo $(( ${argv[-1]} )); }; f 'HOME=1'; git log",
        "f() { echo $(( $10 )); }; f 1 2 3 4 5 6 7 8 9 'HOME=1'; git log",
        "f() { echo $(( $~1 )); }; f 1; git log",
        "f() { echo $(( ${~1} )); }; f 1; git log",
        "f() { echo $(( ${(P)1} )); }; f x; git log",
        "f() { echo $(( ${!1} )); }; f x; git log",
        "f() { echo $(( ${(j: :)@} )); }; f 1; git log",
        "f() { echo $(( $^@ )); }; f 1; git log",
        "echo $(( $- )); git log",
        "for v; do echo $((v)); done; git log",
        "echo $(( $1 )); git log",
        "bash -c 'echo $(( $1 ))' _ 'HOME=1'; git log",
        "x='1 HOME=1'; f() { echo $(( $2 )); }; f $x; git log",
        "IFS=x; n=1xHOME=1; f() { echo $(( $2 )); }; f $n; git log",
        "n=4; f() { echo $(( $2 )); }; f $n* HOME=1; git log",
        "f() { echo $(( $2 )); }; f {1,HOME=1}; git log",
        "f() { echo $(( $3 )); }; f 1{,} 'HOME=1'; git log",
        "f() { echo $(( $2 )); }; f * 'HOME=1'; git log",
        "n=1; for k in 1 2; do echo $((n)); n='HOME=1'; done; git log",
        "aarch64='HOME=1'; typeset -i HOSTTYPE; git log",
    ] {
        assert!(!allowed(cmd), "should be refused: {cmd}");
    }
    for cmd in [
        "echo $(( LINENO / 2 ))",
        "echo $(( RANDOM % 6 ))",
        "f() { echo $(( $1 + $2 )); }; f 2 3",
        "f() { echo $(( $1 + 1 )); }; f 5 *",
        "f() { echo $(( ${#1} )); }; f abc",
        "n=3; echo $(( n * 2 ))",
        "f() { echo $(( $1 )); }; n=4; f $n",
        "f() { echo $(( $1 )); }; for i in 1 2; do f $i; done",
    ] {
        assert!(allowed(cmd), "should be allowed: {cmd}");
    }
}

#[test]
fn printf_reads_only_its_numeric_slots_as_numbers() {
    for cmd in [
        "printf '%d %s\\n' 5 hello", "printf '%*s' 5 x", "printf '%s=%d\\n' a 1 b 2", "printf '%%d %s' x", "x=hi; printf '%s\\n' \"$x\"",
        "printf '%s\\n' {a,b}", "[ -f *.txt ]", "[ -e *.log ]", "[ -d src/* ]", "[ -z \"$(git status --porcelain)\" ]",
        "[ \"$(uname)\" = Darwin ]", "x=hi; printf '%s\\n' $x", "[ -n \"$x\" ]", "[ \"$a\" = \"$b\" ]", "[ -f *.txt -a -r x ]",
        "test -f ~/x", "printf 'x%' 5", "[ $? -ne 0 ]; git log", "test $? = 0", "test $$ -gt 0", "[ $# -eq 0 ]",
    ] {
        assert!(allowed(cmd), "should be allowed: {cmd}");
    }
    for cmd in [
        "printf '%s=%d\\n' a 1 b HOME=1; git log", "printf '%1$s' HOME=1; git log", "printf '%s %*d' a HOME=1 2; git log",
        "printf \"%s$(cat f)\" a HOME=1; git log", "printf -- \"$(cat f)\" HOME=1; git log", "printf \"%s`cat f`\" a HOME=1; git log",
        "o=-v; printf $o x %d HOME=1; git log", "printf %{d,s} HOME=1 HOME=2; git log", "printf {%,x}d HOME=1; git log",
        "printf '%'{d,} HOME=1; git log", "printf %{1..1}d HOME=1; git log", "printf -- %{d,s} HOME=1; git log",
        "printf {-v,'a[HOME=1]'} x; git log", "printf %[d] HOME=1; git log", "printf '%s %d' {a,b} 1 HOME=1; git log",
        "printf '%s %d' * HOME=1; git log",
    ] {
        assert!(!allowed(cmd), "should be refused: {cmd}");
    }
}

#[test]
fn a_reading_that_reaches_text_no_call_set_is_refused() {
    for cmd in [
        "hBc='HOME=1'; echo $(( ${-} )); git log",
        "[[ ${-} -eq 1 ]]; git log",
        "f='HOME=1'; f() { echo $(( ${@:0:1} )); }; f 1; git log",
        "f='HOME=1'; f() { echo $(( ${*:0:1} )); }; f 1; git log",
        "f() { echo $(( ${@:0} )); }; f 1; git log",
        "env KEYTIMEOUT='HOME=1' bash -c 'echo $((KEYTIMEOUT))'; git log",
        "env COLUMNS='HOME=1' bash -c 'echo $((COLUMNS))'; git log",
        "echo $(( UID )); git log",
        "echo $(( COLUMNS / 2 )); git log",
        "x=+; n=1; echo $(( n$x$x )); git log",
        "x=-; echo $(( -$x SHLVL )); git log",
        "x=--; n=5; echo $(( $x n )); git log",
        "x=-5; n=1; echo $(( n-$x )); git log",
        "n=$((0-5)); m=1; echo $(( m-$n )); git log",
        "date() { echo HOME=1; }; x=$(date +%s); echo $((x)); git log",
        "date() { echo HOME=1; }; echo $(( $(date +%s) )); git log",
        "wc() { echo HOME=1; }; echo $(( $(wc -l) )); git log",
        "for k in 1 2; do echo $(( $(seq 1) )); seq() { echo HOME=1; }; done; git log",
        "printf '%ld\\n' HOME=1; git log",
        "printf '%hd\\n' HOME=1; git log",
        "printf \"%s %'d\\n\" a HOME=1; git log",
        "echo \"${(l:HOME=1:)x}\"; git log",
    ] {
        assert!(!allowed(cmd), "should be refused: {cmd}");
    }
    for cmd in [
        "echo \"${(U)x}\"", "x=a; echo \"${(U)x}\"", "echo \"${(j:,:)x}\"", "echo \"${(l:5::0:)x}\"", "n=$#; echo $(( n - 1 ))",
        "echo $(( 10-$# ))", "f() { echo $(( ${@:1:1} )); }; f 2", "printf '%ld\\n' 5", "x=5; n=1; echo $(( n-$x ))",
        "echo $(( RANDOM % 6 + SECONDS ))",
    ] {
        assert!(allowed(cmd), "should be allowed: {cmd}");
    }
}

/// `print` and `return` are refused as commands today, so only the arithmetic verdict shows that
/// their arguments are read as numbers.
#[test]
fn builtins_that_read_a_number_refuse_an_argument_that_assigns() {
    let _g = enter(PathCtx { cwd: Some(ROOT.into()), root: Some(ROOT.into()), session_id: None });
    let verdict = |text: &str| {
        super::forget_numeric_names();
        let script = crate::cst::parse::parse(text).expect("parses");
        let Some(crate::cst::Cmd::Simple(cmd)) = script.0[0].pipeline.commands.first() else { panic!("not simple: {text}") };
        super::commands::command_verdict(cmd).is_allowed()
    };
    for text in
        ["return y", "shift y", "print -f %d y", "print -f%d y", "print -f %s -f %d y", "print -rf %d y", "print $o %d y", "print -r$x%d y"]
    {
        assert!(!verdict(text), "should be refused: {text}");
    }
    for text in ["return 1", "print -f %s y"] {
        assert!(verdict(text), "should be allowed: {text}");
    }
}

#[test]
fn each_boundary_of_the_arithmetic_reader_holds() {
    for cmd in [
        "x=abc; x=5; echo $((x)); git log", "echo $(( $(date +%s 2>&1) - 1 )); git log", "echo $(( $(TZ=UTC date +%s) - 1 )); git log",
        "echo $(( $(cat 1) )); git log", "echo $a[LINENO[1]+(HOME=1)]; git log", "echo ${a[1]:HOME=1}; git log",
        "read x; declare -i x=5; git log", "declare -i x+=5; git log", "printf -- %d y; git log", "printf '%s %s %d\\n' 1 2 y; git log",
        "printf '%-*s|\\n' y x; git log", "[[ 1 -eq y ]]; git log", "echo $((RANDOM++)); git log",
    ] {
        assert!(!allowed(cmd), "should be refused: {cmd}");
    }
    for cmd in [
        "echo $a[LINENO[1]]=1", "x='a b'; echo ${x}", "n=${#HOME}; echo $((n + 1))", "declare a=5 b=x; echo $((a + 1))",
        "test -v '[HOME=1]'", "test -v", "printf - %d y", "printf 'x%*' 5",
    ] {
        assert!(allowed(cmd), "should be allowed: {cmd}");
    }
}

#[test]
fn a_bracket_closes_at_its_own_depth() {
    assert_eq!(super::bracketed("a[1]+HOME=1] tail", '[', ']'), "a[1]+HOME=1");
    assert_eq!(super::bracketed("a[b[1]]]", '[', ']'), "a[b[1]]");
    assert_eq!(super::bracketed("never closed", '[', ']'), "never closed");
}

#[test]
fn a_parameter_flag_of_any_length_is_skipped_before_the_name() {
    // The subscript after the flags is read only when the flags are skipped exactly.
    for body in ["(U)a[HOME=1]", "(l:30:)a[HOME=1]", "(l:3:)a[HOME=1]", "(Ul:3:)a[HOME=1]"] {
        assert!(!super::parameter_verdict(body).is_allowed(), "should be refused: {body}");
    }
    for body in ["(U)a[1]", "(l:30:)a[1]", "(l:3:)a[1]"] {
        assert!(super::parameter_verdict(body).is_allowed(), "should be allowed: {body}");
    }
}

#[test]
fn a_name_given_something_other_than_a_number_is_free_even_when_unbound() {
    assert!(!super::is_free("n"));
    super::mark_free("n");
    assert!(super::is_free("n"));
    assert!(!super::is_free("m"));
    super::mark_free("*");
    assert!(super::is_free("m"));
}

/// A word that stays one word is not read as a numeric slot, so an assignment in a text slot beside
/// it passes; one that may split makes every argument count.
#[test]
fn printf_counts_every_argument_only_beside_a_word_that_may_split() {
    for cmd in [
        "printf '%s %s %d' HOME=1 ${#x} 1", "printf '%s %s %d' HOME=1 $$ 1", "x=abc; printf '%s %s %d' HOME=1 $x 1",
        "x_y=5; printf '%s %s %d' HOME=1 $x_y 1", "printf 'HOME=1 %s %d' {1,2} 5",
    ] {
        assert!(allowed(cmd), "should be allowed: {cmd}");
    }
    for cmd in ["printf '%s %s %d' HOME=1 $@ 1; git log", "printf '%s %s %d' HOME=1 $y 1; git log"] {
        assert!(!allowed(cmd), "should be refused: {cmd}");
    }
}

/// `printf`'s own options and `print`/`read` are checked on the command alone: the command
/// classifier refuses most of them before the arithmetic reader is asked.
#[test]
fn the_words_a_builtin_reads_as_numbers_start_where_its_options_end() {
    let _g = enter(PathCtx { cwd: Some(ROOT.into()), root: Some(ROOT.into()), session_id: None });
    let verdict = |text: &str| {
        super::forget_numeric_names();
        let script = crate::cst::parse::parse(text).expect("parses");
        let Some(crate::cst::Cmd::Simple(cmd)) = script.0[0].pipeline.commands.first() else { panic!("not simple: {text}") };
        super::commands::command_verdict(cmd).is_allowed()
    };
    for text in [
        "printf '%s %s %d' HOME=1 ${!} 1", "printf -$x 5", "printf 'HOME=1 %d' {1,2}", "print -f%d 1", "read -t", "read -r -t",
        "printf -x -- %d 5",
    ] {
        assert!(verdict(text), "should be allowed: {text}");
    }
    for text in ["printf -x -y %d HOME=1", "printf -x -- %d HOME=1", "printf -v x %d HOME=1", "read -t HOME=1"] {
        assert!(!verdict(text), "should be refused: {text}");
    }
}

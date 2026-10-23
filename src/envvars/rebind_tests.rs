use crate::pathctx::{PathCtx, enter};

const ROOT: &str = "/work/u/proj";

fn allowed(cmd: &str) -> bool {
    let _g = enter(PathCtx { cwd: Some(ROOT.into()), root: Some(ROOT.into()), session_id: None });
    crate::command_verdict(cmd).is_allowed()
}

#[test]
fn a_loop_over_a_listed_variable_is_judged_as_its_assignment() {
    for cmd in [
        "for HOME in /tmp/x; do git log; done",
        "for HOME in /tmp/x; do :; done; git log",
        "for XDG_CONFIG_HOME in /tmp/x; do jj log; done",
        "for GIT_CONFIG_GLOBAL in /tmp/g; do git log; done",
        "for LD_PRELOAD in /tmp/x.so; do ls; done",
        "for HOME in /work/u/proj /tmp/x; do git log; done",
        "for HOME in .; do git log; done",
        "for HOME in $X; do git log; done",
        "for HOME in /work/u/*; do git log; done",
        "for HOME in $(mktemp -d); do git log; done",
        "for HOME; do git log; done",
        "for HOME in /{tmp,x}/y; do git log; done",
        "for HOME in /t{m,}p/y; do git log; done",
        "for XDG_CONFIG_HOME in /{tmp,x}/y; do jj log; done",
        "for HOME in /work/u/proj/~x; do git log; done",
    ] {
        assert!(!allowed(cmd), "should be refused: {cmd}");
    }
    for cmd in [
        "for HOME in /work/u/proj/home; do git log; done",
        "declare -x FOO=bar",
        "declare +n r",
        "for f in /tmp/x; do echo hi; done",
        "for HOME in ''; do echo hi; done",
    ] {
        assert!(allowed(cmd), "should be allowed: {cmd}");
    }
}

#[test]
fn reading_into_a_listed_variable_is_refused_since_its_value_is_unknown() {
    for cmd in [
        "read HOME <<< /tmp/x; git log",
        "read -r HOME < /work/u/proj/home.txt; git log",
        "echo /tmp/x | while read HOME; do git log; done",
        "while read HOME; do git log; done <<< /tmp/x",
        "read -r a XDG_CONFIG_HOME <<< 'x y'",
        "read -a HOME <<< x",
        "read -raHOME <<< x",
        "IFS= read -r GIT_CONFIG_GLOBAL <<< x",
        "read 'HOME?prompt' <<< /tmp/x; git log",
        "builtin read HOME <<< x",
        "x=HOME; read $x <<< /tmp/x; git log",
        "x=HOME; read -r \"$x\" <<< /tmp/x; git log",
        "read -A zshexit_functions <<< f",
        "touch HOME; read HOM? <<< /tmp; git log",
        "read GIT_DI[R] <<< /tmp; git status",
        "read -A *_functions <<< f",
        "typeset -T X chpwd_functions; X=f; cd src",
        "typeset -T X=f chpwd_functions; cd src",
        "declare -T X zshexit_functions; X=f",
        "export -T X chpwd_functions; X=f; cd src",
        "readonly -T X=f chpwd_functions; cd src",
        "typeset -gT X HOME",
        "declare HOM?=x; git log",
        "export GIT_DI[R]=x; git status",
        "typeset HOM{E,}=x; git log",
        "export $(echo HOME)=x; git log",
        "declare -n r=HOME; r=/tmp/x; git log",
        "typeset -n r=HOME; r=/tmp/x; git log",
        "declare -n r; r=HOME; r=/tmp/x; git log",
        "declare -gn r=LD_PRELOAD; export r=/tmp/x.so; ls",
        "bash -c 'declare -n r=HOME; r=/tmp/x; git log'",
    ] {
        assert!(!allowed(cmd), "should be refused: {cmd}");
    }
    for cmd in [
        "read -r line <<< x", "read -p 'Name?' line <<< x", "read 'line?Name? ' <<< x", "read XHOME <<< x", "echo HOME",
        "typeset -T X words; X=a:b", "export FOO=\"$BAR\"", "declare x=$(echo hi)",
    ] {
        assert!(allowed(cmd), "should be allowed: {cmd}");
    }
}

#[test]
fn an_option_that_is_not_its_own_text_is_refused() {
    for option in ["-$x", "-$(echo T)", "-`echo n`", "\"-$x\"", "-g$(echo T)", "-n$x", "-a$(echo HOME)", "+$x"] {
        for setter in ["read", "declare", "typeset", "local", "export", "readonly"] {
            let cmd = format!("x=T; {setter} {option} X chpwd_functions; X=f; cd src");
            assert!(!allowed(&cmd), "should be refused: {cmd}");
        }
    }
}

#[test]
fn an_operand_that_is_not_its_own_text_never_names_a_variable() {
    for var in ["HOME", "GIT_DIR", "chpwd_functions"] {
        for spelling in ["$(echo V)", "`echo V`", "\"$(printf V)\"", "{V,zz}", "V{,}"] {
            let operand = spelling.replace('V', var);
            for setter in ["read -r S <<< /tmp/x", "read -A S <<< f", "typeset -T X S", "export -T X S", "declare -T X S"] {
                let cmd = format!("{}; git status", setter.replace('S', &operand));
                assert!(!allowed(&cmd), "should be refused: {cmd}");
            }
        }
    }
}

mod properties {
    use proptest::prelude::*;

    use super::allowed;

    proptest! {
        #[test]
        fn a_loop_variable_agrees_with_the_same_assignment(
            var in prop::sample::select(vec!["HOME", "XDG_CONFIG_HOME", "GIT_CONFIG_GLOBAL", "LD_PRELOAD", "PATHX", "f"]),
            value in prop::sample::select(vec!["/tmp/x", "/private/tmp/a/b", "/work/u/proj/home", "/opt/x", "/TMP/y"]),
        ) {
            let looped = allowed(&format!("for {var} in {value}; do git log; done"));
            let assigned = allowed(&format!("{var}={value} git log"));
            prop_assert_eq!(looped, assigned, "{} {}", var, value);
        }

        #[test]
        fn reading_into_a_listed_variable_is_always_refused(
            var in prop::sample::select(vec!["HOME", "XDG_CONFIG_HOME", "GIT_CONFIG_GLOBAL", "JJ_CONFIG", "LD_PRELOAD", "GIT_DIR"]),
            flags in prop::sample::select(vec!["", "-r ", "-r -a ", "-ra", "-t 1 "]),
        ) {
            let cmd = format!("read {flags}{var} <<< x");
            prop_assert!(!allowed(&cmd), "should be refused: {}", cmd);
        }
    }
}

//! Loop variables, and words whose text is unknown.
//!
//! The invariants: a `for` over literal words approves only if its body approves with each word
//! written in place of the variable, and a word whose text is unknown is never approved where a
//! command reads it as code.

use crate::handler_property_tests::workspace;

fn is_safe_command(cmd: &str) -> bool {
    crate::command_verdict_in(cmd, workspace()).is_allowed()
}
use proptest::prelude::*;

fn refused(cmds: &[&str]) -> Vec<String> {
    cmds.iter().filter(|c| is_safe_command(c)).map(|c| format!("approved: {c}")).collect()
}

fn approved(cmds: &[&str]) -> Vec<String> {
    cmds.iter().filter(|c| !is_safe_command(c)).map(|c| format!("refused: {c}")).collect()
}

#[test]
fn a_literal_loop_item_reaches_the_handler_as_its_value() {
    let bad = refused(&[
        r#"for p in '{print > "out"}'; do awk "$p" a.txt; done"#,
        r#"for f in /etc/shadow; do awk '{print}' "$f"; done"#,
        r#"for s in 'unlink "x"'; do perl -e "$s"; done"#,
        r#"for a in $'a\nb'; do curl -A "$a" https://example.com; done"#,
        r#"for f in /inet/x; do gawk '{print}' "$f"; done"#,
        r#"for f in a.txt /etc/shadow; do cat "$f"; done"#,
        r#"for p in '{print}'; do for f in /etc/shadow; do awk "$p" "$f"; done; done"#,
        r#"for f in /etc/shadow; do timeout 5 cat "$f"; done"#,
        r#"for f in /etc/shadow; do awk '{print}' "${f}"; done"#,
        r#"for f in /etc/shadow; do awk -- '{print}' $f; done"#,
    ]);
    assert!(bad.is_empty(), "{bad:#?}");
}

#[test]
fn a_harmless_literal_loop_is_still_approved() {
    let bad = approved(&[
        r#"for f in a.txt b.txt; do cat "$f"; done"#,
        r#"for f in a.txt b.txt; do awk '{print}' "$f"; done"#,
        r#"for p in '{print}' '{print $1}'; do awk "$p" a.txt; done"#,
        "for x in 1 2 3; do echo $x; done",
        r#"for f in ./*.txt; do cat "$f"; done"#,
        r#"for f in ./*.txt; do awk '{print}' "$f"; done"#,
        r#"o='{print}'; for p in "$o"; do awk "$p" a.txt; done"#,
        r#"for o in '{print}'; do for p in "$o"; do awk "$p" a.txt; done; done"#,
    ]);
    assert!(bad.is_empty(), "{bad:#?}");
}

#[test]
fn what_one_pass_leaves_is_in_force_on_the_next() {
    let bad = refused(&[
        r#"for f in ./*.txt; do f=/etc/shadow; cat "$f"; done"#,
        r#"q=./a.txt; for i in 1 2; do cat "$q"; q=/etc/shadow; done"#,
        r#"q=./a.txt; while true; do cat "$q"; q=/etc/shadow; done"#,
        r#"for i in 1 2; do cat ./shadow; cd /etc; done"#,
        r#"g() { cat ./a.txt; }; for i in 1 2; do g; g() { cat /etc/shadow; }; done"#,
        r#"q=./a.txt; for i in 1 2; do cat "$q"; for q in /etc/shadow; do true; done; done"#,
        r#"q=./a.txt; for q in /etc/shadow; do true; done; cat "$q""#,
        r#"for i in 1 2; do cat ./shadow; c() { cd /etc; }; c; done"#,
        r#"for i in 1 2; do cat ./shadow; function c { cd /etc; }; if true; then c; fi; done"#,
        r#"for i in 1 2; do cat ./shadow; m() { c() { cd /etc; }; }; m; c; done"#,
        r#"q=./a.txt; for i in 1 2; do cat "$q"; c() { q=/etc/shadow; }; c; done"#,
    ]);
    assert!(bad.is_empty(), "{bad:#?}");
}

#[test]
fn a_value_the_shell_may_change_again_is_not_read_as_written() {
    let bad = refused(&[
        r#"for f in '.*'; do cat $f/a.txt; done"#,
        r#"for f in 'x*'; do grep -r foo $f; done"#,
        r#"v='.*'; cat $v/a.txt"#,
        r#"f='{print}'; echo x | read f; awk "$f" a.txt"#,
        r#"f=a.txt; echo x | read f; cat "$f""#,
        "echo x | cd /etc; cat ./shadow",
        r#"f=a.txt; true | f=/etc/shadow; cat "$f""#,
        r#"for f in a.txt; do true | f=/etc/shadow; cat "$f"; done"#,
        r#"g() { cat ./a.txt; }; true | g() { cat /etc/shadow; }; g"#,
    ]);
    assert!(bad.is_empty(), "{bad:#?}");
}

#[test]
fn a_module_flag_is_read_as_code() {
    let bad = refused(&[
        r#"for p in ./*.txt; do perl -M"$p" -e 'print 1'; done"#,
        "perl -MFoo=a} -e 'print 1'",
        "perl '-MFoo;print' -e 'print 1'",
        "find . -name '*.txt' -exec awk {} a.txt \\;",
    ]);
    assert!(bad.is_empty(), "{bad:#?}");
    let bad = approved(&["perl -MList::Util -e 'print 1'", "perl -Mstrict -Mwarnings -e 'print 1'", "perl -MFoo=a,b -e 'print 1'"]);
    assert!(bad.is_empty(), "{bad:#?}");
}

#[test]
fn an_unknown_word_is_refused_where_it_is_read_as_code() {
    let bad = refused(&[
        r#"awk "BEGIN{$x}""#,
        r#"awk "{print \$1; $x}" a.txt"#,
        r#"awk -- "$p""#,
        r#"awk "/$x/" a.txt"#,
        r#"sed "s/foo/$x/" a.txt"#,
        r#"sed -n "/$x/p" a.txt"#,
        r#"perl -e "print $x""#,
        r#"perl -ne "print if /$x/" a.txt"#,
        r#"awk "$(cat a.txt)" b.txt"#,
        r#"echo x | while read p; do awk "$p"; done"#,
        r#"echo x | while read p; do sed "$p" a.txt; done"#,
        r#"echo /etc/shadow | while read f; do awk '{print}' "$f"; done"#,
        r#"ls | while read p; do awk "$p" a.txt; done"#,
        r#"for f in ./*.txt; do awk "$f" a.txt; done"#,
        "echo x | xargs -I{} awk {} a.txt",
        "echo x | xargs -I{} sed {} a.txt",
        "echo x | xargs awk",
        r#"echo x | while read p; do q="$p"; awk "$q" a.txt; done"#,
        r#"for p in ./*.txt; do q="pre$p"; awk "$q" a.txt; done"#,
        r#"f() { awk "$1" a.txt; }; echo x | while read p; do f "$p"; done"#,
        r#"ls | while read p; do awk -v "$p" '{print}' a.txt; done"#,
    ]);
    assert!(bad.is_empty(), "{bad:#?}");
}

#[test]
fn an_unknown_word_is_judged_as_a_relative_name_where_it_is_read_as_a_path() {
    let bad = approved(&[
        "echo a.txt | xargs cat",
        "echo x | xargs -I{} cat {}",
        "find . -name '*.txt' | xargs -I{} cp {} out/",
        r#"find . -name '*.txt' | while read f; do cat "$f"; done"#,
    ]);
    assert!(bad.is_empty(), "{bad:#?}");
}

const ITEMS: &[&str] = &["a.txt", "./b.txt", "{print}", "/etc/shadow", "x y", "-n", "~/notes", "sub/c.txt"];

const BODIES: &[&str] = &[
    r#"cat "$v""#, r#"awk '{print}' "$v""#, r#"awk "$v" a.txt"#, r#"echo "$v""#, r#"ls "$v""#, r#"head -n 1 "$v""#, r#"grep foo "$v""#,
    r#"sed -n p "$v""#, r#"wc -l -- "$v""#,
];

fn quoted(item: &str) -> String {
    format!("'{}'", item.replace('\'', r"'\''"))
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(128))]

    #[test]
    fn a_literal_loop_approves_exactly_when_every_substituted_body_does(
        items in proptest::sample::subsequence(ITEMS, 1..=3),
        body in proptest::sample::select(BODIES),
    ) {
        let list: Vec<String> = items.iter().map(|i| quoted(i)).collect();
        let looped = format!("for v in {}; do {body}; done", list.join(" "));
        let each = list.iter().all(|i| is_safe_command(&body.replace("\"$v\"", i)));
        prop_assert_eq!(is_safe_command(&looped), each, "{}", looped);
    }
}

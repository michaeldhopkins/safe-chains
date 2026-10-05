use std::collections::BTreeSet;

use proptest::prelude::*;

use super::corpus::command_invocations;
use super::rust_examples::examples_in;
use super::snapshot::{self, Drift, Snapshot};

fn snap(rows: &[(&str, &str)]) -> Snapshot {
    rows.iter()
        .map(|(i, v)| (i.to_string(), v.to_string()))
        .collect()
}

fn generated(toml_text: &str) -> BTreeSet<String> {
    let value: toml::Table = toml::from_str(toml_text).expect("fixture parses");
    let mut out = BTreeSet::new();
    for command in value["command"].as_array().expect("commands") {
        command_invocations(command.as_table().expect("table"), &mut out);
    }
    out
}

proptest! {
    #[test]
    fn escaping_round_trips_and_leaves_one_tab_per_line(inv in "\\PC*|[\\\\\t\n\r a-z]*") {
        let escaped = snapshot::escape(&inv);
        prop_assert!(!escaped.contains(['\t', '\n', '\r']));
        prop_assert_eq!(snapshot::unescape(&escaped), Ok(inv.clone()));
        let one = snap(&[(inv.as_str(), "safe-read")]);
        prop_assert_eq!(snapshot::parse(&snapshot::render(&one)), Ok(one));
    }

    #[test]
    fn the_split_files_put_every_row_back_in_its_own_bucket(
        rows in proptest::collection::btree_map("\\PC{0,8}", prop_oneof!["denied", "inert"], 0..30)
    ) {
        let mut merged = Snapshot::new();
        for (bucket, text) in snapshot::render_buckets(&rows) {
            let part = snapshot::parse(&text).expect("bucket parses");
            prop_assert!(part.keys().all(|inv| snapshot::bucket_of(inv) == bucket));
            merged.extend(part);
        }
        prop_assert_eq!(merged, rows);
    }

    #[test]
    fn a_snapshot_compared_with_itself_has_no_drift(
        rows in proptest::collection::btree_map("[a-z -]{1,12}", prop_oneof!["denied", "inert", "safe-read", "safe-write"], 0..20)
    ) {
        prop_assert!(Drift::between(&rows, &rows, &Snapshot::new()).is_empty());
    }
}

#[test]
fn parse_refuses_what_render_never_writes() {
    for bad in [
        "no tab here",
        "ls\tmaybe",
        "a\\q\tdenied",
        "ls\tdenied\nls\tinert",
    ] {
        assert!(snapshot::parse(bad).is_err(), "{bad:?} parsed");
    }
}

#[test]
fn drift_sorts_each_movement_into_its_own_bucket() {
    let old = snap(&[
        ("lost", "safe-read"),
        ("raised", "safe-read"),
        ("gained", "denied"),
        ("lowered", "safe-write"),
        ("same", "inert"),
        ("gone", "safe-read"),
        ("gone-and-lost", "safe-read"),
    ]);
    let new = snap(&[
        ("lost", "denied"),
        ("raised", "safe-write"),
        ("gained", "inert"),
        ("lowered", "safe-read"),
        ("same", "inert"),
        ("fresh", "denied"),
    ]);
    let recheck = snap(&[("gone", "safe-read"), ("gone-and-lost", "denied")]);
    let d = Drift::between(&old, &new, &recheck);
    let row = |i: &str, a: &str, b: &str| (i.to_string(), a.to_string(), b.to_string());
    assert_eq!(
        d.newly_denied,
        vec![
            row("gone-and-lost", "safe-read", "denied"),
            row("lost", "safe-read", "denied")
        ]
    );
    assert_eq!(d.raised, vec![row("raised", "safe-read", "safe-write")]);
    assert_eq!(d.newly_allowed, vec![row("gained", "denied", "inert")]);
    assert_eq!(d.lowered, vec![row("lowered", "safe-write", "safe-read")]);
    assert_eq!(d.added, vec![("fresh".to_string(), "denied".to_string())]);
    let dropped: Vec<_> = d.dropped.iter().map(|(i, _)| i.as_str()).collect();
    assert_eq!(dropped, ["gone", "gone-and-lost"]);
    let summary = d.summary(1);
    assert!(
        summary.contains(
            "REGRESSION newly denied: 2\n    gone-and-lost    (safe-read -> denied)\nREGRESSION"
        ),
        "{summary}"
    );
}

const FROB: &str = r#"
[[command]]
name = "frob"
aliases = ["gfrob"]
standalone = ["--list", "-q"]
valued = ["--depth"]
examples_safe = ["frob --list --odd"]
examples_denied = ["frob --nuke"]

[command.path_gate]
flags = { "--out" = "write" }

[[command.sub]]
name = "show"
aliases = ["sh"]
require_any = ["--json"]
standalone = ["--all"]
first_arg = ["item/*"]

[[command.sub.sub]]
name = "deep"
candidate = true
loopback_valued = ["--endpoint"]

[[command.sub.flag]]
name = "-c"
classifies = "unclassified"
value_prefix = "core.x="

[[command.matrix]]
parents = ["pr"]
level = "Inert"
[command.matrix.actions]
list = "listing"
get = { policy = "listing", guard = "--output" }

[command.handler_policy.listing]
standalone = ["--web"]
"#;

#[test]
fn a_node_generates_its_flags_subs_aliases_matrix_and_examples() {
    let got = generated(FROB);
    for want in [
        "frob",
        "frob x",
        "frob --help",
        "frob --version",
        "frob --list",
        "frob -q",
        "frob --depth x",
        "frob --out x",
        "frob --list -q",
        "frob --list x",
        "frob --list --depth x",
        "frob --list --odd",
        "frob --nuke",
        "gfrob",
        "gfrob --list",
        "gfrob show --json",
        "frob show",
        "frob show --json",
        "frob show --all",
        "frob show --json --all",
        "frob sh --json --all",
        "frob show item/x",
        "frob show item/x --all",
        "frob show -c core.x=x",
        "frob show deep",
        "frob show deep --endpoint http://localhost:8000",
        "frob show deep --endpoint x",
        "frob pr list",
        "frob pr list --web",
        "frob pr get",
        "frob pr get --output x --web",
    ] {
        assert!(got.contains(want), "missing {want:?}; generated {got:#?}");
    }
    assert!(
        !got.iter().any(|i| i.contains('*')),
        "a glob reached the corpus: {got:#?}"
    );
}

#[test]
fn rust_example_blocks_yield_their_strings_and_nothing_else() {
    let source = r#"
        #[cfg(test)]
        mod tests {
            safe! { a: "curl -s x", b: "awk 1" }
            denied! { c: "curl -o /etc/x x", }
            safe_read! { d: "sed -n 1p x" }
            other! { e: "never" }
            const S: &str = "safe! { f: \"never\" }";
            fn t() { inert! { g: "echo" } }
        }
    "#;
    let got: Vec<_> = examples_in(source, "fixture").into_iter().collect();
    assert_eq!(
        got,
        [
            "awk 1",
            "curl -o /etc/x x",
            "curl -s x",
            "echo",
            "sed -n 1p x"
        ]
    );
}

#[test]
#[should_panic(expected = "an example macro block")]
fn an_unreadable_example_block_fails_loudly() {
    examples_in("fn t() { safe! { a: format!(\"x\") } }", "fixture");
}

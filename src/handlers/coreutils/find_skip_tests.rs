use super::skip_of;
use crate::engine::resolve::frozen_walk::{Skip, Test};

fn skip(line: &str) -> Skip {
    skip_of(&line.split_whitespace().collect::<Vec<_>>())
}

fn path(p: &str) -> Test {
    Test::Path(p.to_string(), false)
}

#[test]
fn a_prune_prefix_keeps_its_tests_away() {
    let s = skip(". -path ./.git -prune -o -exec chmod +x {} +");
    assert_eq!(s.pruned, vec![(path("./.git"), false)]);
    let s = skip("-L . ( -name .git -o -type d -iname .jj ) -prune -or -print");
    assert_eq!(s.pruned, vec![(Test::Name(".git".into(), false), false), (Test::Name(".jj".into(), true), true)]);
}

#[test]
fn depth_turns_a_prune_into_an_exclusion() {
    for line in
        [". -path ./.git -prune -o -delete", ". -depth -path ./.git -prune -o -print", ". -mindepth 2 -path ./.git -prune -o -print"]
    {
        let s = skip(line);
        assert!(s.pruned.is_empty(), "{line}");
        assert_eq!(s.excluded, vec![path("./.git")], "{line}");
    }
}

#[test]
fn filters_before_the_first_action_are_read() {
    let s = skip(". -type f -not -path ./.git/* ! -name .envrc -exec sed -i s/a/b/ {} +");
    assert!(s.files_only);
    assert_eq!(s.excluded, vec![path("./.git/*"), Test::Name(".envrc".into(), false)]);
}

#[test]
fn a_filter_after_the_action_or_beside_an_alternative_is_not_read() {
    for line in [
        ". -exec chmod +x {} + -not -path ./.git/*",
        ". -not -path ./.git/* -o -name x -exec chmod +x {} +",
        ". ( -not -path ./.git/* ) -exec chmod +x {} +",
        ". -not -path ./.git/* , -exec chmod +x {} +",
        ". ! -type f -exec chmod +x {} +",
        ". -name ! -path ./.git/* -exec chmod +x {} +",
        ". -path ./.git -o -prune -exec chmod +x {} +",
    ] {
        assert_eq!(skip(line), Skip::default(), "{line}");
    }
}

#[test]
fn an_inner_command_s_own_words_do_not_count_as_alternatives() {
    let s = skip(". -not -path ./.git/* -exec cc -o x {} ;");
    assert_eq!(s.excluded, vec![path("./.git/*")]);
}

mod properties {
    use super::skip;
    use proptest::prelude::*;

    const WORDS: [&str; 26] = [
        "-mindepth", "2", "-depth", "-delete", ".", "-path", "./.git", "-name", ".git", "-iname", "*", "-prune", "-o", "-or", "!", "-not",
        "-type", "d", "f", "(", ")", ",", "-exec", "chmod", "{}", "+",
    ];

    proptest! {
        #[test]
        fn only_a_word_that_keeps_entries_away_keeps_any(picks in prop::collection::vec(0..WORDS.len(), 0..14)) {
            let line = picks.iter().map(|i| WORDS[*i]).collect::<Vec<_>>().join(" ");
            let s = skip(&line);
            let has = |w: &str| line.split(' ').any(|x| x == w);
            prop_assert!(s.pruned.is_empty() || has("-prune"), "{}", line);
            prop_assert!(s.excluded.is_empty() || has("-prune") || has("!") || has("-not"), "{}", line);
            prop_assert!(!s.files_only || (has("-type") && has("f")), "{}", line);
            prop_assert!(s.pruned.is_empty() || !(has("-mindepth") || has("-depth") || has("-delete")), "{}", line);
        }
    }
}

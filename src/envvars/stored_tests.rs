use proptest::prelude::*;

use super::super::{Shape, shape_of};
use super::{changes_later_shells, parsed, stored_assignment_verdict};
use crate::verdict::Verdict;

fn spelled(name: &str, middle: &str, case: u8) -> String {
    let name = name.replace('*', middle);
    match case {
        0 => name,
        1 => name.to_ascii_lowercase(),
        _ => name
            .chars()
            .enumerate()
            .map(|(i, c)| if i % 2 == 0 { c.to_ascii_lowercase() } else { c.to_ascii_uppercase() })
            .collect(),
    }
}

proptest! {
    /// Every name in `[stored]`, in any letter case, with any text where it has a `*`, with or
    /// without bash's `+`, is refused whatever its value, so a stored value never decides it.
    #[test]
    fn every_stored_name_is_refused_whatever_the_value(
        index in 0usize..1000,
        middle in "[A-Za-z0-9_]{0,6}",
        case in 0u8..3,
        appends in any::<bool>(),
        value in prop_oneof![Just(String::new()), Just("x".to_string()), Just("./bin".to_string()), "[ -~]{0,12}"],
    ) {
        let names = parsed().names;
        let name = spelled(&names[index % names.len()], &middle, case);
        let name = if appends { format!("{name}+") } else { name };
        prop_assert_eq!(stored_assignment_verdict(&name, &value), Verdict::Denied, "{}={}", name, value);
    }

    /// A name outside `[stored]` is judged exactly as `export` judges it.
    #[test]
    fn an_unlisted_name_is_judged_as_an_export(name in "[A-Z][A-Z0-9_]{0,10}", value in "[ -~]{0,12}") {
        prop_assume!(!changes_later_shells(&name));
        prop_assert_eq!(stored_assignment_verdict(&name, &value), super::super::statement_assignment_verdict(&name, &value, true));
    }
}

#[test]
fn every_variable_a_shell_expands_or_runs_at_each_prompt_is_stored() {
    let prompts = ["PROMPT_COMMAND", "PS0", "PS1", "PS2", "PS3", "PS4", "PROMPT", "PROMPT2", "PROMPT3", "PROMPT4"];
    let zsh = ["RPROMPT", "RPROMPT2", "RPS1", "RPS2", "SPROMPT"];
    for name in prompts.iter().chain(&zsh) {
        assert!(matches!(shape_of(name), Some(Shape::Opaque | Shape::Command)), "{name} is not judged as code in envvars.toml");
        assert!(changes_later_shells(name), "{name} is in [env] as code but missing from [stored]");
    }
}

#[test]
fn ordinary_stored_variables_are_judged_as_exports() {
    for (name, value) in [("FOO", "bar"), ("NODE_ENV", "production"), ("RUST_LOG", "debug"), ("PATH_EXTRA", "./bin")] {
        assert!(stored_assignment_verdict(name, value).is_allowed(), "{name}={value}");
    }
}

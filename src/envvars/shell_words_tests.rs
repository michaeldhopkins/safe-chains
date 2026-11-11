use proptest::prelude::*;

use super::{SHELLS, STD_BINS, shell_program_verdict, shell_text_verdict};

proptest! {
    #[test]
    fn only_a_named_shell_in_a_standard_bin_is_allowed(prefix in "[a-z/~.]{0,11}/", name in "[a-z]{1,6}", tail in "( -c x)?") {
        let value = format!("{prefix}{name}{tail}");
        let expected = tail.is_empty() && STD_BINS.contains(&prefix.as_str()) && SHELLS.contains(&name.as_str());
        prop_assert_eq!(shell_program_verdict(&value).is_allowed(), expected, "{}", value);
    }

    #[test]
    fn every_standard_shell_is_allowed(bin in prop::sample::select(STD_BINS), shell in prop::sample::select(SHELLS)) {
        let value = format!("{bin}{shell}");
        prop_assert!(shell_program_verdict(&value).is_allowed());
    }

    #[test]
    fn shell_text_is_refused_and_plain_text_is_not(plain in "[A-Za-z0-9 ._/:@+,=-]{0,12}", c in prop::sample::select(crate::handlers::shell_text::SHELL_TEXT)) {
        prop_assert!(shell_text_verdict(&plain).is_allowed());
        let marked = format!("{plain}{c}");
        prop_assert!(!shell_text_verdict(&marked).is_allowed());
    }
}

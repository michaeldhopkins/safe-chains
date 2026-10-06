use super::proptests::arb_script;
use super::*;
use proptest::prelude::*;

fn redirect_count(cmd: &Cmd) -> usize {
    match cmd {
        Cmd::Simple(s) => s.redirs.len(),
        Cmd::Subshell { redirs, .. }
        | Cmd::BraceGroup { redirs, .. }
        | Cmd::For { redirs, .. }
        | Cmd::While { redirs, .. }
        | Cmd::Until { redirs, .. }
        | Cmd::If { redirs, .. }
        | Cmd::DoubleBracket { redirs, .. }
        | Cmd::Case { redirs, .. } => redirs.len(),
        Cmd::FunctionDef { .. } => 0,
    }
}

fn redirect_counts(script: &Script) -> Vec<usize> {
    script.0.iter().flat_map(|s| s.pipeline.commands.iter().map(redirect_count)).collect()
}

proptest! {
    #[test]
    fn normalizing_keeps_every_redirect(script in arb_script(2)) {
        prop_assert_eq!(redirect_counts(&script.normalize()), redirect_counts(&script), "{}", script);
    }
}

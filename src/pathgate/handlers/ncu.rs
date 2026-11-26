//! `ncu`: the manifest is written only in upgrade mode.

use crate::parse::Token;
use crate::pathgate::{Role, gate};

/// `ncu [--upgrade] [--packageFile FILE]` — npm-check-updates REPORTS available updates by
/// default and only rewrites the manifest with `--upgrade`/`-u`, so the manifest's role follows
/// the mode. Without this, `ncu --upgrade --packageFile /etc/package.json` wrote outside the
/// workspace.
pub(in crate::pathgate) fn ncu_mode(tokens: &[Token]) -> bool {
    let args: Vec<&str> = tokens[1..].iter().map(Token::as_str).collect();
    let writes = args.iter().any(|a| matches!(*a, "--upgrade" | "-u"));
    let role = if writes { Role::Write } else { Role::Read };
    let mut it = args.iter().copied();
    while let Some(t) = it.next() {
        if t == "--packageFile"
            && let Some(v) = it.next()
            && gate(role, v)
        {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::ncu_mode;
    use crate::parse::Token;
    use crate::pathgate::{Role, gate};
    use proptest::prelude::*;

    fn toks(words: &[&str]) -> Vec<Token> {
        words.iter().map(|w| Token::from_test(w)).collect()
    }

    proptest! {
        #[test]
        fn manifest_role_follows_upgrade_flag_in_every_position(
            path in prop::sample::select(vec![
                "package.json", "./sub/package.json", ".git/config", "/etc/package.json",
                "~/package.json", "~/.ssh/id_rsa", "~/.aws/credentials", "/etc/shadow",
            ]),
            upgrade in prop::sample::select(vec!["--upgrade", "-u"]),
            upgrade_first in any::<bool>(),
        ) {
            let read_tokens = toks(&["ncu", "--packageFile", path]);
            prop_assert_eq!(super::super::dispatch("ncu_mode", &read_tokens), gate(Role::Read, path));

            let write_tokens = if upgrade_first {
                toks(&["ncu", upgrade, "--packageFile", path])
            } else {
                toks(&["ncu", "--packageFile", path, upgrade])
            };
            prop_assert_eq!(ncu_mode(&write_tokens), gate(Role::Write, path));
            prop_assert!(!ncu_mode(&toks(&["ncu", upgrade, "--packageFile"])));
            prop_assert!(!super::super::dispatch("mtree_mode", &toks(&["mtree", "-p", "package.json"])));
        }
    }
}

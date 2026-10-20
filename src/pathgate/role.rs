//! The role a path argument plays, out of `pathgate.rs`.

use serde::Deserialize;

/// What to do with a path found in a given argument slot.
#[derive(Deserialize, Clone, Copy, PartialEq, Default, Debug)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Role {
    /// Gate by read locus — a disclosing read (`od FILE`, `scp` source, `wget --post-file`).
    Read,
    /// Gate by read locus, as a SWEEP — the command descends the path rather than reading it as
    /// one file (`rg PATTERN DIR`, `ag`, an archiver's source tree).
    ///
    /// The distinction matters only above the workspace, and it is the shield that makes it
    /// matter: `rg foo ~` names `~`, which is not a credential store, and then reads
    /// `~/.ssh/id_rsa` out of it. A name test can only clear a name someone wrote, so a root that
    /// stands for everything beneath it cannot be cleared at all. `read` stays correct for the
    /// commands that open exactly the file they are given.
    #[serde(rename = "read_tree")]
    ReadTree,
    /// Gate by write locus — a write-target (`tee FILE`, `curl -o`, a converter's output).
    Write,
    /// Gate by EXECUTOR locus — a flag whose value selects code to run (`cargo --manifest-path
    /// DIR/Cargo.toml` runs that project's build.rs/tests). Denies a foreign or `/tmp` executor
    /// (the execution-origin band), where `write` would allow `/tmp`. See
    /// docs/design/behavioral-taxonomy-execution-origin.md.
    Exec,
    /// Gate as the remote shell rsync starts: a command line, allowed only as ssh or rsh with plain
    /// options, or as a program in the project judged by the executor locus. `exec` judges one path
    /// and so refuses `ssh -p 22`; this reads the options instead. See `rsync_shell.rs`.
    #[serde(rename = "remote_shell")]
    RemoteShell,
    /// Never gate — a URL, an `-i` identity, a converter's non-disclosing transcode input. The
    /// default, so a command declaring only path-bearing flags leaves its positionals ungated.
    #[default]
    Ignore,
}

impl Role {
    /// How much this role withholds, for picking between clauses that both hold.
    ///
    /// Written out rather than derived from declaration order so that reordering the enum — which
    /// looks cosmetic — cannot quietly change which role a self-overlapping gate selects. `Exec`
    /// outranks `Write` because it withholds strictly more: it refuses `/tmp` and home, where a
    /// write target is allowed to land.
    pub(crate) fn restrictiveness(self) -> u8 {
        match self {
            Role::Ignore => 0,
            Role::Read => 1,
            Role::ReadTree => 2,
            Role::Write => 3,
            Role::Exec | Role::RemoteShell => 4,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Role;
    use proptest::prelude::*;

    const ALL: [(Role, &str); 6] = [
        (Role::Ignore, "ignore"),
        (Role::Read, "read"),
        (Role::ReadTree, "read_tree"),
        (Role::Write, "write"),
        (Role::Exec, "exec"),
        (Role::RemoteShell, "remote_shell"),
    ];

    proptest! {
        /// Every role parses from the name the TOML uses, and a role later on the ladder ignore,
        /// read, read_tree, write, then the two executor roles never withholds less.
        #[test]
        fn roles_parse_by_name_and_rank_along_the_ladder(a in 0usize..6, b in 0usize..6) {
            let ((role_a, name_a), (role_b, _)) = (ALL[a], ALL[b]);
            #[derive(serde::Deserialize)]
            struct Holder {
                role: Role,
            }
            let parsed: Holder = toml::from_str(&format!("role = \"{name_a}\"")).expect("a role name parses");
            prop_assert_eq!(parsed.role, role_a);
            if a.min(4) < b.min(4) {
                prop_assert!(role_a.restrictiveness() < role_b.restrictiveness());
            } else if a.min(4) == b.min(4) {
                prop_assert_eq!(role_a.restrictiveness(), role_b.restrictiveness());
            }
        }
    }
}

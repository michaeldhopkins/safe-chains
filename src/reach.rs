use crate::{pathctx, sanitize_display};

/// Why a not-auto-approved command's path reach was flagged — so the nudge can explain the actual
/// reason instead of a one-size-fits-all "outside the working directory". A peer's hidden file and a
/// path genuinely above cwd both deny, but the remedy differs, and conflating them is what reads as
/// "directory parsing is broken".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReachReason {
    /// A known credential store (`.ssh`, `.aws`, keychain…).
    Credential,
    /// A file safe-chains reads its OWN permissions from (`~/.config/safe-chains.toml`,
    /// `~/.claude/settings.json`). Distinct from `OutsideWorkspace` because the generic remedy
    /// there — grant the path — is not merely unhelpful but FALSE: the write face is frozen, so
    /// following the advice changes nothing. For safe-chains' own config it is also circular,
    /// telling the user to edit the file they are being stopped from editing.
    FrozenTrustFile,
    /// The DIRECTORY a trust file lives in. Distinct from `FrozenTrustFile` because only HALF of
    /// it is refused: writing a file into `~/.config` is ordinary and stays allowed, and only
    /// removing or replacing the directory is not. Copy that said "this is refused" flatly would
    /// misdescribe a directory the user explicitly granted.
    FrozenTrustRoot,
    /// One of the files that decide who may log in (`/etc/passwd`, `/etc/sudoers`, `/etc/pam.d`,
    /// the loader and boot). Same false-remedy problem as `FrozenTrustFile`.
    FrozenSystemIntegrity,
    /// A raw block or character device (`/dev/mem`, `/dev/rdisk0`, `/dev/sda`). Distinct from
    /// `FrozenSystemIntegrity` because the `device` rung outranks `system-integrity`, so without
    /// its own arm every device was explained as a file that "decides who may log in" — which is
    /// both false and unhelpful about the thing that actually makes a device dangerous.
    RawDevice,
    /// Genuinely above/outside the working directory.
    OutsideWorkspace,
    /// A path built by an interpolation that nothing confines (`./out/$i`, `> $(cmd)`). It is not
    /// outside anything — it names WHATEVER the value turns out to be, which is why it cannot be
    /// admitted — so the remedy is to constrain the spelling, not to grant a directory.
    Unconfined,
    /// A temp path that is NOT this session's scratchpad. Reading and writing it is fine; RUNNING
    /// code from it is not, because anonymous `/tmp` is where downloaded/foreign code lands. This
    /// is the one reach whose remedy is usually "that IS my working directory" — so the nudge says
    /// how to bless it rather than implying the agent did something wrong.
    ForeignTemp,
    RepositorySnapshot,
}

impl ReachReason {
    /// The reason as the reader should hear it: a snapshot refusal whose `--ignore-working-copy`
    /// form would not be approved either (a stricter level) is reported as the trust root it is.
    pub fn settled(self, command: &str, approved: &dyn Fn(&str) -> bool) -> ReachReason {
        match self {
            ReachReason::RepositorySnapshot if !without_snapshot(command).is_some_and(|c| approved(&c)) => ReachReason::FrozenTrustRoot,
            reason => reason,
        }
    }

    /// Whether naming this path in `~/.config/safe-chains.toml` actually changes the verdict.
    ///
    /// The refusal copy offers a grant, or explains that granting will not help, and the two must
    /// not diverge: advice that does nothing costs more than silence, because the reader follows
    /// it, sees no change, and stops believing the rest. `a_refusal_offers_a_grant_only_when_one_
    /// would_work` runs the verdict twice — once with the grant applied — and holds this to it.
    ///
    /// `Credential` is `true` as of 2026-09-01 and was `false` before it. A grant naming a store
    /// used to move the locus and leave `reads_secret` set, so the message's "name that path"
    /// advice was FALSE while reading exactly right. That is the failure this pairing exists to
    /// catch, and it went unnoticed because nothing compared the sentence to the behaviour.
    pub fn grant_helps(self) -> bool {
        match self {
            // A grant that NAMES the store clears the shield for reading it.
            ReachReason::Credential | ReachReason::RawDevice => true,
            // "That IS my working directory" is the usual answer here.
            ReachReason::ForeignTemp | ReachReason::OutsideWorkspace => true,
            // Frozen faces stay frozen however specifically they are named: an agent that can edit
            // the trust file decides what is approved next, and one that can write `/etc/sudoers`
            // owns the machine's authorization substrate.
            ReachReason::FrozenTrustFile
            | ReachReason::FrozenTrustRoot
            | ReachReason::FrozenSystemIntegrity
            | ReachReason::RepositorySnapshot => false,
            // There is no path to grant. The remedy is to constrain the spelling.
            ReachReason::Unconfined => false,
        }
    }

    /// The self-contained nudge body ("it reaches `X`, …") including the reason-appropriate remedy.
    /// Callers add their own framing (block / please-confirm) and the docs link.
    pub fn message(self, path: &str) -> String {
        let path = &sanitize_display(path);
        match self {
            ReachReason::Credential => format!(
                "it reaches `{path}`, a credential store. The agent has no ordinary reason to touch \
                 one. If that was not intended, stop it. If you do want to allow it, name that path \
                 in ~/.config/safe-chains.toml. A grant on a parent directory does not reach a \
                 credential store"
            ),
            ReachReason::RawDevice => format!(
                "it reaches `{path}`, a raw device. Reading one is not reading a file — a disk \
                 device hands over every file on it and memory hands over every secret in it — and \
                 writing one goes underneath the filesystem entirely. If you do want the read, \
                 name that path in ~/.config/safe-chains.toml; the write stays refused"
            ),
            ReachReason::FrozenTrustFile => format!(
                "it reaches `{path}`. safe-chains reads its own permissions from that file, so a \
                 write there is never auto-approved. Granting the path does not change that, \
                 because an agent that can edit this file can decide what gets approved next. Edit \
                 it yourself if you meant to change it"
            ),
            ReachReason::FrozenTrustRoot => format!(
                "it reaches `{path}`. safe-chains reads its own permissions from a file in that \
                 directory, so removing or replacing the directory itself is never auto-approved: \
                 doing that would point the trust root somewhere else. Writing files into it is \
                 fine, and granting the path does not change either half. Move or delete it \
                 yourself if you meant to"
            ),
            ReachReason::FrozenSystemIntegrity => format!(
                "it reaches `{path}`. That file decides who may log in and what they may do, so a \
                 write there is never auto-approved. Granting the path does not change that. Edit \
                 it yourself if you meant to change it"
            ),
            ReachReason::ForeignTemp => format!(
                "it runs code from `{path}`, a temporary directory that is not this session's \
                 scratchpad. Reading and writing temp files is fine. Running code from there is \
                 not, because a downloaded script lands in the same place. If this is a working \
                 directory you trust, grant it in ~/.config/safe-chains.toml. A scratchpad the \
                 harness reports for this session is recognized on its own and needs no grant"
            ),
            ReachReason::Unconfined => format!(
                "the path `{path}` is built by an interpolation, so what it names depends on a \
                 value the command does not show. It could be anywhere, which is why it cannot be \
                 auto-approved. If the interpolated part cannot contain a `/`, put literal text \
                 beside it in the same path component. `out/dx_$i.txt` is approved where `out/$i` \
                 is not: the first is a filename whatever `$i` holds, and the second could be `..`"
            ),
            ReachReason::RepositorySnapshot => format!(
                "it names `{path}` as the repository to use, and jj snapshots the working copy into \
                 that repository before it reads anything: a write outside the working directory. \
                 With --ignore-working-copy right after `jj`, an observing command (log, show, \
                 status) skips the snapshot and only reads the repository, so it is judged as a \
                 read; it then shows the repository as last recorded, without edits made since. Granting \
                 the path does not change that: a write to that repository can rewrite the trust \
                 file inside it, so a command that needs the snapshot is one to run yourself"
            ),
            ReachReason::OutsideWorkspace => match pathctx::cwd().map(|c| sanitize_display(&c)) {
                Some(cwd) => format!(
                    "it reaches `{path}`, outside the working directory `{cwd}`. If the agent is \
                     running from the wrong directory, relaunch it where you meant to be. To allow \
                     it from here, grant that path in ~/.config/safe-chains.toml"
                ),
                None => format!(
                    "it reaches `{path}`, outside the working directory. To allow it, grant that \
                     path in ~/.config/safe-chains.toml"
                ),
            },
        }
    }
}

pub(crate) fn classify(resolved: &str) -> ReachReason {
    // Asked on the LITERAL structure, so an interpolated component cannot strip the credential
    // warning off a path that plainly names one. `cat ~/.ssh/$(id)` was reported as merely
    // "built by an interpolation" — offering to flank it, which can never help, while dropping
    // the one sentence that matters — and the CONFINED spelling fell through to
    // "outside the working directory", whose remedy is to GRANT the path.
    //
    // Granting a credential store IS possible now (a grant covers what it names), so the
    // objection is no longer "that remedy cannot work". It is that the generic wording says
    // "grant that path" while meaning the ordinary parent-directory grant, which is exactly
    // the form a credential store does not accept. The Credential arm spells out the
    // difference instead.
    if crate::engine::resolve::names_credential_store(resolved) {
        ReachReason::Credential
    } else if let Some(kind) = crate::engine::resolve::frozen_write_kind(resolved) {
        // Ahead of Unconfined and OutsideWorkspace for the same reason Credential is: both of
        // those end in "grant that path", which for a frozen write face is FALSE rather than
        // merely vague, and for safe-chains' own config it is circular as well.
        match kind {
            crate::engine::resolve::FrozenWrite::TrustFile => ReachReason::FrozenTrustFile,
            crate::engine::resolve::FrozenWrite::TrustRootDir => ReachReason::FrozenTrustRoot,
            crate::engine::resolve::FrozenWrite::SystemIntegrity => ReachReason::FrozenSystemIntegrity,
            crate::engine::resolve::FrozenWrite::RawDevice => ReachReason::RawDevice,
        }
    } else if crate::engine::resolve::anchoring_of(resolved) == crate::engine::facet::Anchoring::Opaque {
        // Ahead of OutsideWorkspace because it is the more specific diagnosis of the SAME
        // refusal, and the generic wording actively misleads here: it names a working-directory
        // problem the user does not have and a remedy (grant the path) that cannot work,
        // since the path is not a fixed path at all.
        ReachReason::Unconfined
    } else {
        ReachReason::OutsideWorkspace
    }
}

pub(crate) fn without_snapshot(command: &str) -> Option<String> {
    let rest = command.trim_start().strip_prefix("jj")?;
    rest.starts_with([' ', '\t']).then(|| format!("jj --ignore-working-copy{rest}"))
}

pub(crate) fn repository_snapshot(command: &str) -> Option<(String, ReachReason)> {
    let rewritten = without_snapshot(command)?;
    let words = shell_words::split(command).ok()?;
    let repository = crate::pathgate::handlers::jj::repository_value(&words)?;
    if classify(&crate::pathctx::resolve(&repository)) != ReachReason::FrozenTrustRoot {
        return None;
    }
    (!crate::is_safe_command(command) && crate::is_safe_command(&rewritten)).then_some((repository, ReachReason::RepositorySnapshot))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::ReachReason;
    use proptest::prelude::*;

    pub(crate) const ALL: &[ReachReason] = &[
        ReachReason::Credential,
        ReachReason::FrozenTrustFile,
        ReachReason::FrozenTrustRoot,
        ReachReason::FrozenSystemIntegrity,
        ReachReason::RawDevice,
        ReachReason::OutsideWorkspace,
        ReachReason::Unconfined,
        ReachReason::ForeignTemp,
        ReachReason::RepositorySnapshot,
    ];

    proptest! {
        #[test]
        fn every_reason_names_the_path_on_one_line(path in "[~/a-z.\n\r\t\u{202e} ]{1,24}") {
            for reason in ALL {
                let text = reason.message(&path);
                prop_assert_eq!(text.lines().count(), 1, "{:?}: {}", reason, text);
                prop_assert!(text.contains(&crate::sanitize_display(&path)), "{:?}: {}", reason, text);
            }
        }
    }

    fn in_a_project<T>(body: impl FnOnce(&str) -> T) -> T {
        use crate::pathctx::{PathCtx, enter};
        let home = std::env::var("HOME").expect("HOME");
        assert!(home.starts_with('/'), "HOME must be absolute for these tests: {home}");
        let ws = format!("{home}/projects/scproj");
        let _g = enter(PathCtx { cwd: Some(ws.clone()), root: Some(ws), ..Default::default() });
        body(&home)
    }

    const OBSERVING: &[&str] = &["log", "diff", "show", "status", "st", "bookmark list", "op log", "file list", "tag list"];
    const MUTATING: &[&str] = &["new", "describe -m x", "abandon", "bookmark set main", "git fetch", "workspace update-stale"];

    proptest! {
        #[test]
        fn a_trust_root_repository_is_explained_by_its_snapshot_only_for_an_observing_sub(
            observing in any::<bool>(),
            pick in any::<prop::sample::Index>(),
            spelling in 0usize..3,
            root in prop::sample::select(vec![".claude", ".config"]),
            tail in prop::sample::select(vec!["", " --no-pager", " --quiet"]),
        ) {
            in_a_project(|home| {
                let repo = format!("{home}/{root}");
                let flag = match spelling {
                    0 => format!("-R {repo}"),
                    1 => format!("--repository {repo}"),
                    _ => format!("--repository={repo}"),
                };
                let sub = if observing { pick.get(OBSERVING) } else { pick.get(MUTATING) };
                let command = format!("jj {flag} {sub}{tail}");
                let reach = crate::workspace_overreach(&command);
                if observing {
                    assert_eq!(reach, Some((repo.clone(), ReachReason::RepositorySnapshot)), "{command}");
                } else {
                    assert!(!matches!(reach, Some((_, ReachReason::RepositorySnapshot))), "{command}: {reach:?}");
                }
            });
        }
    }

    #[test]
    fn the_snapshot_reason_is_true_to_the_verdict() {
        use crate::engine::resolve::regions::with_grants;
        in_a_project(|home| {
            let repo = format!("{home}/.claude");
            let command = format!("jj -R {repo} log");
            assert!(!crate::is_safe_command(&command));
            assert!(crate::is_safe_command(&format!("jj --ignore-working-copy -R {repo} log")));
            assert!(!with_grants(&[(repo.as_str(), false, true)], || crate::is_safe_command(&command)));
            assert!(!ReachReason::RepositorySnapshot.grant_helps());
            for other in [format!("{home}/.config/safe-chains.toml"), "/etc/sudoers".to_string(), format!("{home}/x")] {
                let reach = crate::workspace_overreach(&format!("jj -R {other} log"));
                assert!(!matches!(reach, Some((_, ReachReason::RepositorySnapshot))), "{other}: {reach:?}");
            }
            assert_eq!(super::repository_snapshot(&format!("jjx -R {repo} log")), None);
            let snapshot = ReachReason::RepositorySnapshot;
            assert_eq!(snapshot.settled(&command, &|c| crate::is_safe_command(c)), snapshot);
            assert_eq!(snapshot.settled(&command, &|_| false), ReachReason::FrozenTrustRoot);
            assert_eq!(ReachReason::OutsideWorkspace.settled(&command, &|_| false), ReachReason::OutsideWorkspace);
        });
    }
}

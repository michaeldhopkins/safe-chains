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
}

impl ReachReason {
    /// The self-contained nudge body ("it reaches `X`, …") including the reason-appropriate remedy.
    /// Callers add their own framing (block / please-confirm) and the docs link.
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
            ReachReason::FrozenTrustFile | ReachReason::FrozenTrustRoot | ReachReason::FrozenSystemIntegrity => false,
            // There is no path to grant. The remedy is to constrain the spelling.
            ReachReason::Unconfined => false,
        }
    }

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

#[cfg(test)]
mod tests {
    use super::ReachReason;
    use proptest::prelude::*;

    const ALL: &[ReachReason] = &[
        ReachReason::Credential,
        ReachReason::FrozenTrustFile,
        ReachReason::FrozenTrustRoot,
        ReachReason::FrozenSystemIntegrity,
        ReachReason::RawDevice,
        ReachReason::OutsideWorkspace,
        ReachReason::Unconfined,
        ReachReason::ForeignTemp,
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
}

//! `--unknown-folder`: checking a command as a hook does when the tool does not say which folder it
//! runs in, and the part of `--explain` that says how each write was judged.

use safe_chains::cst::Explanation;
use safe_chains::engine::level::Level;
use safe_chains::pathctx::anchor::{Anchor, FolderLevel, Use};
use safe_chains::pathctx::folder::{self, Note};
use safe_chains::verdict::{SafetyLevel, Verdict};

/// At `reads` nothing above a read is approved, whatever `--level` allows, as the hook's ceiling does.
pub fn ceiling(
    level: Option<FolderLevel>,
    threshold: SafetyLevel,
    engine: Option<&'static Level>,
) -> (SafetyLevel, Option<&'static Level>) {
    match level {
        Some(FolderLevel::Reads) => (threshold.min(SafetyLevel::SafeRead), engine),
        _ => (threshold, engine),
    }
}

/// The explanation held to `threshold`: a segment approved only above it is shown as refused, as the
/// verdict `run_cli` gives would have it. Without this `--level reader --explain 'cargo fmt'` read
/// "auto-approves" while `--level reader 'cargo fmt'` exited 1.
pub fn under_threshold(mut explanation: Explanation, threshold: SafetyLevel) -> Explanation {
    let cap = |v: Verdict| match v {
        Verdict::Allowed(l) if l > threshold => Verdict::Denied,
        other => other,
    };
    for segment in &mut explanation.segments {
        segment.verdict = cap(segment.verdict);
    }
    explanation.overall = cap(explanation.overall);
    explanation
}

/// Print how the folder level judged each write, and return whether the command is approved once
/// the `reads` ceiling is applied.
pub fn explain(level: Option<FolderLevel>, overall: Verdict) -> bool {
    let Some(level) = level else {
        return overall.is_allowed();
    };
    let writes = matches!(overall, Verdict::Allowed(l) if l > SafetyLevel::SafeRead);
    print!("{}", render(level, &folder::notes(), writes));
    match overall {
        Verdict::Allowed(l) => level > FolderLevel::Reads || l <= SafetyLevel::SafeRead,
        Verdict::Denied => false,
    }
}

fn render(level: FolderLevel, notes: &[Note], writes: bool) -> String {
    let mut out = format!(
        "\n  folder unknown: writes are judged at `{}` (--unknown-folder, or [unknown_folder] writes in ~/.config/safe-chains.toml)\n",
        level.name()
    );
    if level == FolderLevel::Reads {
        if writes {
            out.push_str("    · `reads` approves no write, and this command writes, so it goes to the prompt\n");
        }
        return out;
    }
    for note in notes {
        if let Some(line) = describe(level, note) {
            out.push_str(&format!("    · {line}\n"));
        }
    }
    out
}

fn describe(level: FolderLevel, note: &Note) -> Option<String> {
    let shown = |s: &str| safe_chains::sanitize_display(s);
    Some(match note {
        Note::Path { path, anchor, placed: true, .. } => {
            format!("`{}` is {}: judged as a path in the workspace", shown(path), anchor.name())
        }
        Note::Path { path, anchor, use_, placed: false } => {
            let why = match (anchor, use_) {
                (Anchor::RelativePlain, Use::Rebind) if level == FolderLevel::Developer => {
                    "`developer` leaves deleting, moving and linking to the prompt".to_string()
                }
                (Anchor::RelativePlain, Use::Rebind) => "the folder itself is not removed, moved or linked".to_string(),
                (Anchor::RelativeSensitive, _) => "its name means something wherever the folder is".to_string(),
                _ => "it could land outside the folder".to_string(),
            };
            format!("`{}` is {}: {why}, so it goes to the prompt", shown(path), anchor.name())
        }
        Note::Leaf { anchor: Some(Anchor::NamesItsWrites), admitted: true, .. } => return None,
        Note::Leaf { command, anchor: Some(anchor), admitted: true } => {
            format!("`{}` is {}: approved at `{}`", shown(command), anchor.name(), level.name())
        }
        Note::Leaf { admitted: true, .. } => return None,
        Note::Leaf { command, admitted: false, .. } => format!(
            "`{}` writes in its folder without naming a path, and safe-chains has no record of what it writes there, so it goes to the prompt",
            shown(command)
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_caps_the_threshold_and_the_others_leave_it() {
        let name = |(t, l): (SafetyLevel, Option<&'static Level>)| (t, l.map(|l| l.name.clone()));
        assert_eq!(name(ceiling(Some(FolderLevel::Reads), SafetyLevel::SafeWrite, None)), (SafetyLevel::SafeRead, None));
        assert_eq!(name(ceiling(Some(FolderLevel::Reads), SafetyLevel::Inert, None)), (SafetyLevel::Inert, None));
        assert_eq!(name(ceiling(Some(FolderLevel::Developer), SafetyLevel::SafeWrite, None)), (SafetyLevel::SafeWrite, None));
        assert_eq!(name(ceiling(None, SafetyLevel::SafeWrite, None)), (SafetyLevel::SafeWrite, None));
    }

    #[test]
    fn every_kind_of_note_is_described() {
        let notes = [
            Note::Path { path: "out.txt".into(), anchor: Anchor::RelativePlain, use_: Use::Write, placed: true },
            Note::Path { path: "build".into(), anchor: Anchor::RelativePlain, use_: Use::Rebind, placed: false },
            Note::Path { path: ".zshrc".into(), anchor: Anchor::RelativeSensitive, use_: Use::Write, placed: false },
            Note::Path { path: "../x".into(), anchor: Anchor::RelativeUnplaced, use_: Use::Write, placed: false },
            Note::Leaf { command: "cargo build".into(), anchor: Some(Anchor::ImplicitOutput), admitted: true },
            Note::Leaf { command: "git stash".into(), anchor: None, admitted: false },
        ];
        let text = render(FolderLevel::Developer, &notes, true);
        for expected in ["out.txt", "deleting", ".zshrc", "outside the folder", "implicit-output", "no record"] {
            assert!(text.contains(expected), "{expected} missing from:\n{text}");
        }
        assert!(render(FolderLevel::Reads, &notes, true).contains("`reads` approves no write"));
        assert!(!render(FolderLevel::Reads, &notes, false).contains("approves no write"));
    }
}

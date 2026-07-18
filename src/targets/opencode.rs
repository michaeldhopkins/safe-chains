use std::path::{Path, PathBuf};

use super::{InstallOutcome, Target};

pub struct OpenCodeTarget;

impl Target for OpenCodeTarget {
    fn name(&self) -> &'static str {
        "opencode"
    }

    fn display_name(&self) -> &'static str {
        "OpenCode"
    }

    fn detect_paths(&self, home: &Path) -> Vec<PathBuf> {
        vec![home.join(".config").join("opencode")]
    }

    fn install(&self, _home: &Path) -> Result<InstallOutcome, String> {
        // opencode has no integration point safe-chains can use YET:
        //  - No runtime hook — its `permission.ask` plugin hook is defined in the SDK but never fires
        //    (github.com/anomalyco/opencode/issues/7006).
        //  - The only alternative, a static `permission.bash` glob allowlist, cannot represent
        //    safe-chains' per-ARGUMENT classification (`"git *"` would allow `git push` as well as
        //    `git status`). A previous `--opencode-config` generator was inert (empty pattern set) and
        //    was DROPPED as misleading. Revisit when #7006 lands (a real runtime hook).
        Ok(InstallOutcome::Skipped {
            reason: "OpenCode has no runtime hook safe-chains can drive yet (its plugin hook does not \
                     fire — opencode #7006), and a static permission glob can't represent per-argument \
                     safety. Not integrated — watching for an upstream hook."
                .to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn install_returns_skip_explaining_no_integration() {
        let dir = tempfile::tempdir().unwrap();
        let outcome = OpenCodeTarget.install(dir.path()).unwrap();
        match outcome {
            InstallOutcome::Skipped { reason } => {
                assert!(reason.to_lowercase().contains("hook"));
            }
            other => panic!("expected Skipped, got {:?}", std::mem::discriminant(&other)),
        }
    }
}

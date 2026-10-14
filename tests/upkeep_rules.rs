//! Dependencies move through the owner's `safe-chains-deps` upkeep job, never Dependabot: a Dependabot config
//! would open pull requests nobody manages (and, in a private repository, run CI on metered
//! minutes), and AGENTS.md has to name the job so the next agent knows where updates come from.

use std::path::PathBuf;

const JOB: &str = "safe-chains-deps";

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn agents_md_names_the_upkeep_job() {
    let agents = std::fs::read_to_string(root().join("AGENTS.md")).unwrap_or_default();
    assert!(!agents.is_empty(), "AGENTS.md is missing or empty; the root is wrong");
    assert!(agents.contains(&format!("`{JOB}`")), "AGENTS.md does not name the `{JOB}` upkeep job");
}

#[test]
fn there_is_no_dependabot_config() {
    let github = root().join(".github");
    for name in ["dependabot.yml", "dependabot.yaml"] {
        assert!(!github.join(name).exists(), ".github/{name} exists; updates move through `{JOB}`");
    }
}

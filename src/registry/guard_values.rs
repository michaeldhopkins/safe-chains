//! A required flag counts only where it is a flag. A valued flag with nothing glued to it takes
//! the next word as its value whatever that word looks like, as getopt does, so `make -f -n` reads
//! a makefile named `-n` and runs its recipes.

use super::TOML_REGISTRY;
use super::types::DispatchKind;

fn require_any_sites(prefix: &[String], kind: &DispatchKind, out: &mut Vec<(Vec<String>, Vec<String>, Vec<String>)>) {
    match kind {
        DispatchKind::RequireAny { require_any, policy, .. } => {
            out.push((prefix.to_vec(), require_any.clone(), policy.valued.clone()));
        }
        DispatchKind::Branching { subs, .. } | DispatchKind::Custom { subs, .. } => {
            for sub in subs {
                let mut words = prefix.to_vec();
                words.push(sub.name.clone());
                require_any_sites(&words, &sub.kind, out);
            }
        }
        _ => {}
    }
}

#[test]
fn a_required_flag_given_as_another_flags_value_is_not_given() {
    let mut sites = Vec::new();
    for (name, spec) in TOML_REGISTRY.iter().filter(|(key, spec)| *key == &spec.name) {
        require_any_sites(std::slice::from_ref(name), &spec.kind, &mut sites);
    }
    let (mut checked, mut approved) = (0usize, Vec::new());
    for (prefix, guards, valued) in &sites {
        for guard in guards.iter().filter(|g| g.len() > 1 && g.starts_with('-')) {
            let alone = format!("{} {guard}", prefix.join(" "));
            if !crate::is_safe_command(&alone) {
                continue;
            }
            for flag in valued.iter().filter(|v| !guards.contains(v)) {
                checked += 1;
                let line = format!("{} {flag} {guard}", prefix.join(" "));
                if crate::is_safe_command(&line) {
                    approved.push(line);
                }
            }
        }
    }
    assert!(checked > 50, "only {checked} guard-as-value lines probed: the walk is wrong");
    assert!(approved.is_empty(), "a guard given as a value was approved:\n{}", approved.join("\n"));
}

use crate::verdict::{SafetyLevel, Verdict};

pub(super) fn command_search_verdict(name: &str, appends: bool, value: &str) -> Verdict {
    let value = if name == "path" { value.strip_prefix("${path}").unwrap_or(value) } else { value };
    if appends && name == "PATH" {
        return search_path_verdict(&format!("$PATH{value}"));
    }
    search_path_verdict(value)
}

fn search_path_verdict(value: &str) -> Verdict {
    value.split(':').map(element_verdict).fold(Verdict::Allowed(SafetyLevel::Inert), Verdict::combine)
}

fn element_verdict(dir: &str) -> Verdict {
    if matches!(dir, "$PATH" | "${PATH}") {
        return Verdict::Allowed(SafetyLevel::Inert);
    }
    if !dir.starts_with('/') {
        return Verdict::Denied;
    }
    if crate::handlers::direct_script::trusted_command_path(&format!("{}/x", dir.trim_end_matches('/'))) {
        return Verdict::Allowed(SafetyLevel::Inert);
    }
    crate::engine::resolve::execute_file_verdict(dir)
}

#[cfg(test)]
#[path = "command_search_tests.rs"]
mod tests;

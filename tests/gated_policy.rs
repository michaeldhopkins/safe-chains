//! What each target does with a gated command, by its kind and the harness's permission mode.
//!
//! Only Grok splits: a not-yet-approved command goes to its permission prompt in a mode where one
//! is shown, and everything else is refused as before. Every other target must answer exactly what
//! its `gated_policy` answered before the split existed, whatever the kind and the mode.

#[path = "support/formats.rs"]
mod formats;
use formats::every_hook_format;
use proptest::prelude::*;
use safe_chains::explain_fix::gate::GateKind;

const SPLIT_TARGETS: &[&str] = &["grok"];

proptest! {
    #[test]
    fn every_other_target_is_unchanged_for_both_kinds(mode in proptest::option::of(".{0,24}"), not_yet in any::<bool>()) {
        let kind = if not_yet { GateKind::NotYet } else { GateKind::Boundary };
        for (name, format) in every_hook_format() {
            if SPLIT_TARGETS.contains(&name) {
                continue;
            }
            prop_assert_eq!(format.gated_policy_for(kind, mode.as_deref()), format.gated_policy(), "{}", name);
        }
    }

}

#[test]
fn the_split_list_names_real_targets() {
    let names: Vec<&str> = every_hook_format().into_iter().map(|(n, _)| n).collect();
    for split in SPLIT_TARGETS {
        assert!(names.contains(split), "{split} is not a hook target");
    }
}

/// A target that does not split never has its kind worked out, so no new code runs ahead of its refusal.
#[test]
fn only_the_split_targets_work_out_the_kind() {
    for (name, format) in every_hook_format() {
        assert_eq!(format.splits_by_kind(), SPLIT_TARGETS.contains(&name), "{name}");
    }
}

#[path = "support/hooks.rs"]
mod hooks;

fn grok_envelope(command: &str, mode: Option<&str>) -> String {
    let mut envelope = serde_json::json!({
        "hookEventName": "pre_tool_use",
        "toolName": "run_terminal_command",
        "toolInput": { "command": command },
        "cwd": "/w",
        "workspaceRoot": "/w",
    });
    if let Some(mode) = mode {
        envelope["permissionMode"] = serde_json::Value::String(mode.to_string());
    }
    envelope.to_string()
}

fn grok_hook(command: &str, mode: Option<&str>) -> (serde_json::Value, i32) {
    let (stdout, _stderr, code) = hooks::run_hook(&["hook", "grok"], &grok_envelope(command, mode));
    (serde_json::from_str(&stdout).unwrap_or_default(), code)
}

fn agy_reason(command: &str) -> String {
    let envelope =
        serde_json::json!({ "toolCall": { "name": "run_command", "args": { "CommandLine": command } }, "workspacePaths": ["/w"] });
    let (stdout, _stderr, _code) = hooks::run_hook(&["hook", "antigravity"], &envelope.to_string());
    let v: serde_json::Value = serde_json::from_str(&stdout).unwrap_or_default();
    v.pointer("/reason").and_then(serde_json::Value::as_str).unwrap_or_default().to_string()
}

#[test]
fn grok_asks_about_a_not_yet_command_with_agys_text_in_a_prompting_mode() {
    for mode in ["default", "auto", "plan"] {
        let (v, code) = grok_hook("zzgatetool sync", Some(mode));
        assert_eq!(code, 0, "{mode}: {v}");
        assert_eq!(v.pointer("/decision").and_then(serde_json::Value::as_str), Some("deny"), "{mode}: {v}");
        assert_eq!(v.pointer("/hookSpecificOutput/permissionDecision").and_then(serde_json::Value::as_str), Some("ask"), "{mode}: {v}");
        let reason = v
            .pointer("/hookSpecificOutput/permissionDecisionReason")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default();
        assert!(!reason.is_empty(), "{mode}: {v}");
        assert_eq!(v.pointer("/reason").and_then(serde_json::Value::as_str), Some(reason));
        assert_eq!(reason, agy_reason("zzgatetool sync"), "the ask reuses agy's text for the same case");
    }
}

#[test]
fn grok_denies_as_before_outside_a_prompting_mode_and_at_a_boundary() {
    let deny_only = |v: &serde_json::Value, code: i32, what: &str| {
        assert_eq!(code, 2, "{what}: {v}");
        assert_eq!(v.pointer("/decision").and_then(serde_json::Value::as_str), Some("deny"), "{what}: {v}");
        assert!(v.pointer("/hookSpecificOutput").is_none(), "{what}: {v}");
    };
    for mode in [None, Some("bypassPermissions"), Some("dontAsk"), Some("zz")] {
        let (v, code) = grok_hook("zzgatetool sync", mode);
        deny_only(&v, code, &format!("not-yet in {mode:?}"));
    }
    for command in ["echo x >> ~/.zshrc", "cat ~/.ssh/id_rsa", "safe-chains config add zzgatetool", "claude -p hi"] {
        for mode in [Some("default"), Some("auto"), Some("plan")] {
            let (v, code) = grok_hook(command, mode);
            deny_only(&v, code, &format!("{command} in {mode:?}"));
        }
    }
}

/// The hook's HOME is a folder under the temporary directory, which no word is held to leave, so
/// `~/notes` is only read that way once the `~` is expanded to it.
#[test]
fn grok_asks_about_a_not_yet_command_naming_a_path_under_a_home_in_a_temporary_folder() {
    let (v, code) = grok_hook("zzgatetool ~/notes", Some("default"));
    assert_eq!(code, 0, "{v}");
    assert_eq!(v.pointer("/hookSpecificOutput/permissionDecision").and_then(serde_json::Value::as_str), Some("ask"), "{v}");
}

/// Under a configured level, a refused segment the default would not approve keeps its own kind:
/// running an agent stays a boundary, refused without a prompt.
#[test]
fn grok_refuses_an_agent_under_a_configured_level_without_asking() {
    use std::io::Write;
    let home = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir_all(home.path().join(".config")).expect("config dir");
    std::fs::write(home.path().join(".config/safe-chains.toml"), "level = \"developer\"\n").expect("config");
    let mut child = std::process::Command::new(hooks::binary())
        .args(["hook", "grok"])
        .env("HOME", home.path())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("spawn safe-chains");
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(grok_envelope("claude -p hi", Some("default")).as_bytes())
        .expect("write");
    let out = child.wait_with_output().expect("wait");
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap_or_default();
    assert_eq!(v.pointer("/decision").and_then(serde_json::Value::as_str), Some("deny"), "{v}");
    assert_eq!(v.pointer("/hookSpecificOutput/permissionDecision"), None, "{v}");
}

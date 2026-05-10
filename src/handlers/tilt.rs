//! `tilt` is two distinct CLIs sharing one binary name. Process inspection
//! can't tell which is on PATH, so the handler routes the first token to
//! the correct grammar:
//!
//! - **K8s tilt** (tilt-dev/tilt) sub names → that sub's TOML-declared policy.
//! - **Ruby tilt** (gem `tilt`, a template engine) → fallback policy. It
//!   accepts at most one positional and requires it to look like a path,
//!   so a bare-word first token (`tilt up`) is denied — those would be
//!   K8s subs we deliberately don't allowlist.
//!
//! All grammar data lives in `commands/tools/tilt.toml`. This file is
//! pure dispatch logic.
use crate::parse::Token;
use crate::registry;
use crate::verdict::Verdict;

pub fn check_tilt(tokens: &[Token]) -> Verdict {
    if let Some(verdict) = registry::try_sub_dispatch("tilt", tokens) {
        return verdict;
    }
    registry::try_fallback_grammar("tilt", tokens).unwrap_or(Verdict::Denied)
}

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    use crate::verdict::{SafetyLevel, Verdict};

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    fn verdict(cmd: &str) -> Verdict {
        crate::command_verdict(cmd)
    }

    safe! {
        // Bare + shared flags
        bare: "tilt",
        long_help: "tilt --help",
        short_help: "tilt -h",
        long_version: "tilt --version",
        short_version_lower: "tilt -v",
        short_version_upper: "tilt -V",

        // Ruby tilt
        ruby_long_list: "tilt --list",
        ruby_short_list: "tilt -l",
        ruby_render_extension: "tilt template.erb",
        ruby_render_subdir: "tilt views/index.haml",
        ruby_render_absolute: "tilt /tmp/page.md",
        ruby_render_stdin: "tilt -",
        ruby_render_with_type_long: "tilt --type erb template.erb",
        ruby_render_with_type_short: "tilt -t erb template.erb",
        ruby_render_with_type_eq: "tilt --type=erb template.erb",
        ruby_render_with_layout: "tilt --layout layout.erb page.erb",
        ruby_render_with_layout_short: "tilt -y layout.erb page.erb",
        ruby_render_flags_after_file: "tilt page.erb -t erb",

        // K8s tilt diagnostic subs
        k8s_version: "tilt version",
        k8s_version_help: "tilt version --help",
        k8s_doctor: "tilt doctor",
        k8s_verify_install: "tilt verify-install",
        k8s_completion: "tilt completion bash",
        k8s_help: "tilt help",
        k8s_help_topic: "tilt help up",

        // K8s tilt get/describe (kubectl-style read)
        k8s_get: "tilt get pod",
        k8s_get_named: "tilt get pod my-pod",
        k8s_get_namespace: "tilt get pod -n default",
        k8s_get_namespace_eq: "tilt get pod --namespace=default",
        k8s_get_all_namespaces: "tilt get pod -A",
        k8s_describe: "tilt describe pod my-pod",
        k8s_describe_namespace: "tilt describe pod my-pod -n prod",
    }

    denied! {
        // K8s tilt write/long-running subs
        k8s_up: "tilt up",
        k8s_down: "tilt down",
        k8s_ci: "tilt ci",
        k8s_apply: "tilt apply -f manifest",
        k8s_delete: "tilt delete resource",
        k8s_trigger: "tilt trigger build",
        k8s_logs: "tilt logs",
        k8s_dump: "tilt dump engine",

        // Ruby tilt: bare-word file form (rejected — could be a K8s sub)
        bare_word_first: "tilt foo",
        bare_word_render: "tilt template",

        // Unknown flags
        unknown_flag: "tilt --evil",
        unknown_short: "tilt -X",

        // Multi-positional (Ruby tilt takes one file)
        two_files: "tilt a.erb b.erb",

        // Sub + bad arg
        version_extra: "tilt version foo",
        doctor_with_dash_flag: "tilt doctor --evil",
    }

    #[test]
    fn k8s_get_returns_safe_read() {
        assert_eq!(
            verdict("tilt get pod"),
            Verdict::Allowed(SafetyLevel::SafeRead),
        );
    }

    #[test]
    fn ruby_render_returns_inert() {
        assert_eq!(
            verdict("tilt template.erb"),
            Verdict::Allowed(SafetyLevel::Inert),
        );
    }

    #[test]
    fn k8s_doctor_returns_inert() {
        assert_eq!(
            verdict("tilt doctor"),
            Verdict::Allowed(SafetyLevel::Inert),
        );
    }

    /// Proptests of the grammar-selection invariant: K8s sub name → sub
    /// policy verdict; otherwise → fallback verdict. They synthesize
    /// inputs the unit tests don't enumerate (random resource names,
    /// random extensions) and would catch regressions in
    /// `try_sub_dispatch` / `try_fallback_grammar` routing logic.
    use proptest::prelude::*;
    proptest! {
        #[test]
        fn get_with_random_resource_is_safe_read(
            resource in "[a-z][a-z0-9-]{0,15}",
        ) {
            let cmd = format!("tilt get {resource}");
            prop_assert_eq!(
                crate::command_verdict(&cmd),
                Verdict::Allowed(SafetyLevel::SafeRead),
            );
        }

        #[test]
        fn describe_with_random_resource_is_safe_read(
            resource in "[a-z][a-z0-9-]{0,15}",
            name in "[a-z][a-z0-9-]{0,15}",
        ) {
            let cmd = format!("tilt describe {resource} {name}");
            prop_assert_eq!(
                crate::command_verdict(&cmd),
                Verdict::Allowed(SafetyLevel::SafeRead),
            );
        }

        #[test]
        fn random_path_shaped_arg_is_inert(
            stem in "[a-z][a-z0-9_]{0,15}",
            ext in "(erb|haml|md|slim|txt)",
        ) {
            let cmd = format!("tilt {stem}.{ext}");
            prop_assert_eq!(
                crate::command_verdict(&cmd),
                Verdict::Allowed(SafetyLevel::Inert),
            );
        }

        #[test]
        fn random_bare_word_is_denied(
            word in "[a-z][a-z0-9-]{0,15}",
        ) {
            // Bare-word tokens that aren't K8s sub names must be denied
            // by the Ruby fallback (path-shape predicate).
            let known_subs = [
                "doctor", "verify-install", "version",
                "completion", "help", "describe", "get",
            ];
            prop_assume!(!known_subs.contains(&word.as_str()));
            let cmd = format!("tilt {word}");
            prop_assert_eq!(crate::command_verdict(&cmd), Verdict::Denied);
        }
    }
}

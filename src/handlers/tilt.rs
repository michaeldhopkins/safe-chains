//! Disambiguates two distinct tools that share the binary name `tilt`:
//!
//! - **Ruby tilt** (gem `tilt`): a template engine. `tilt FILE` renders a
//!   template to stdout. Flags: `--list/-l` (engines), `--type/-t TYPE`,
//!   `--layout/-y LAYOUT`. No subcommands.
//!
//! - **Kubernetes tilt** (tilt-dev/tilt): a containerized dev environment.
//!   `tilt up/down/ci/...` mutates a local cluster. Diagnostic subs are
//!   `version`, `doctor`, `verify-install`, `completion`, `help`, plus
//!   kubectl-style `get` and `describe`.
//!
//! Without process inspection there's no way to know which binary the
//! user has on `$PATH`. We use the first token after `tilt` to pick a
//! grammar: K8s sub names route to K8s validation; flag/file shapes
//! route to Ruby template-engine validation. Forms ambiguous to either
//! grammar (bare `tilt`, `tilt --help`, `tilt --version`) are accepted
//! at Inert.
use crate::parse::{Token, WordSet};
use crate::verdict::{SafetyLevel, Verdict};

static K8S_DIAGNOSTIC_SUBS: WordSet =
    WordSet::new(&["doctor", "verify-install", "version"]);
static K8S_PASSTHROUGH_SUBS: WordSet = WordSet::new(&["completion", "help"]);
static K8S_READ_SUBS: WordSet = WordSet::new(&["describe", "get"]);

static SHARED_BARE_FLAGS: WordSet =
    WordSet::new(&["--help", "--version", "-V", "-h", "-v"]);
static RUBY_LIST_FLAGS: WordSet = WordSet::new(&["--list", "-l"]);
static RUBY_VALUED_FLAGS: WordSet =
    WordSet::new(&["--layout", "--type", "-t", "-y"]);

static K8S_GET_BARE: WordSet = WordSet::new(&[
    "--help", "--no-headers", "--show-labels", "--watch",
    "-A", "-h", "-w",
]);
static K8S_GET_VALUED: WordSet = WordSet::new(&[
    "--context", "--field-selector", "--kubeconfig", "--namespace",
    "--output", "--selector",
    "-l", "-n", "-o",
]);

pub fn check_tilt(tokens: &[Token]) -> Verdict {
    if tokens.len() == 1 {
        return Verdict::Allowed(SafetyLevel::Inert);
    }
    let first = tokens[1].as_str();

    // Shared diagnostic bare flags (same in Ruby and K8s tilt).
    if tokens.len() == 2 && SHARED_BARE_FLAGS.contains(first) {
        return Verdict::Allowed(SafetyLevel::Inert);
    }

    // Ruby tilt: --list / -l prints engine list, no further args.
    if tokens.len() == 2 && RUBY_LIST_FLAGS.contains(first) {
        return Verdict::Allowed(SafetyLevel::Inert);
    }

    if K8S_DIAGNOSTIC_SUBS.contains(first) {
        return validate_diagnostic_sub(tokens);
    }
    if K8S_PASSTHROUGH_SUBS.contains(first) {
        return validate_passthrough_sub(tokens);
    }
    if K8S_READ_SUBS.contains(first) {
        return validate_get_describe(tokens);
    }

    validate_ruby_render(tokens)
}

fn validate_diagnostic_sub(tokens: &[Token]) -> Verdict {
    // version / doctor / verify-install — accept --help/-h only after the sub.
    let mut i = 2;
    while i < tokens.len() {
        let t = tokens[i].as_str();
        if matches!(t, "--help" | "-h") {
            i += 1;
            continue;
        }
        return Verdict::Denied;
    }
    Verdict::Allowed(SafetyLevel::Inert)
}

fn validate_passthrough_sub(tokens: &[Token]) -> Verdict {
    // help <topic> / completion <shell>: accept any non-flag token plus --help/-h.
    let mut i = 2;
    while i < tokens.len() {
        let t = tokens[i].as_str();
        if matches!(t, "--help" | "-h") {
            i += 1;
            continue;
        }
        if t.starts_with('-') {
            return Verdict::Denied;
        }
        i += 1;
    }
    Verdict::Allowed(SafetyLevel::Inert)
}

fn validate_get_describe(tokens: &[Token]) -> Verdict {
    // K8s tilt get/describe: kubectl-style read. Accept the standard read flags.
    let mut i = 2;
    while i < tokens.len() {
        let t = tokens[i].as_str();
        if K8S_GET_BARE.contains(t) {
            i += 1;
            continue;
        }
        if K8S_GET_VALUED.contains(t) {
            if tokens.get(i + 1).is_none() {
                return Verdict::Denied;
            }
            i += 2;
            continue;
        }
        if let Some(eq) = t.find('=') {
            let key = &t[..eq];
            if K8S_GET_VALUED.contains(key) {
                i += 1;
                continue;
            }
        }
        if t.starts_with('-') {
            return Verdict::Denied;
        }
        // Positional: resource type / name. Allow any non-flag token.
        i += 1;
    }
    Verdict::Allowed(SafetyLevel::SafeRead)
}

fn looks_like_template_file(s: &str) -> bool {
    s == "-" || s.contains('/') || s.contains('.')
}

fn validate_ruby_render(tokens: &[Token]) -> Verdict {
    // Ruby tilt: optional flags, then a single template file (or `-` for stdin).
    let mut i = 1;
    let mut saw_file = false;
    while i < tokens.len() {
        let t = tokens[i].as_str();

        // Bare `-` is the stdin file marker, not a flag.
        if t == "-" {
            if saw_file {
                return Verdict::Denied;
            }
            saw_file = true;
            i += 1;
            continue;
        }

        if RUBY_LIST_FLAGS.contains(t) {
            i += 1;
            continue;
        }
        if RUBY_VALUED_FLAGS.contains(t) {
            if tokens.get(i + 1).is_none() {
                return Verdict::Denied;
            }
            i += 2;
            continue;
        }
        if let Some(eq) = t.find('=') {
            let key = &t[..eq];
            if RUBY_VALUED_FLAGS.contains(key) {
                i += 1;
                continue;
            }
        }
        if matches!(t, "--help" | "-h" | "--version" | "-v" | "-V") {
            i += 1;
            continue;
        }

        if !t.starts_with('-') {
            // Reject bare-word first positionals that look like K8s subs we
            // didn't allowlist (up, down, ci, deploy, etc.) — without a path
            // shape, Ruby tilt would error and K8s tilt would mutate.
            if !looks_like_template_file(t) {
                return Verdict::Denied;
            }
            if saw_file {
                return Verdict::Denied;
            }
            saw_file = true;
            i += 1;
            continue;
        }

        return Verdict::Denied;
    }
    Verdict::Allowed(SafetyLevel::Inert)
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

        // Valued flag missing value
        type_missing_value: "tilt --type",
        layout_missing_value: "tilt -y",

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
}

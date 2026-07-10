//! Composition / delegation test harness (the "put it through its paces" suite).
//!
//! Leaf-command safety is necessary but not sufficient: many commands **delegate** to a
//! nested command — `find … -exec CMD {}`, `xargs CMD`, `time CMD`, `sudo CMD`, `bash -c
//! CMD`, `env … CMD` — and the safety of the *composition* is not the safety of the leaf.
//! Making the engine authoritative changed the leaf verdicts, and those changes propagate
//! through every delegating handler.
//!
//! Rather than hand-author an expected verdict per composition, this harness derives the
//! oracle: **a wrapper must never be looser than operating directly on the target it
//! delegates to.** `find /etc -exec rm {}` must be no looser than `rm /etc/f`; `time rm
//! /etc/x` no looser than `rm /etc/x`; an `xargs` reading unknowable stdin items no looser
//! than the bare command (items invisible). The reference verdict is itself a
//! `command_verdict` call, so the full cross-product of wrappers × commands × loci is swept
//! automatically, in forced `new` mode — a composition that ALLOWS what its direct form
//! DENIES is laundering, and fails the sweep. This is what caught the `find -exec {}` →
//! system-delete and `xargs -I` → laundered-destroy bugs.

#[cfg(test)]
mod tests {
    use crate::command_verdict;
    use crate::verdict::Verdict;

    /// Verdict with the engine authoritative (now the default and only path).
    fn new_verdict(cmd: &str) -> Verdict {
        command_verdict(cmd)
    }

    /// True when `comp` grants no more than `reference` — wrapping never loosened. A
    /// composition that allows what its direct form denies is laundering.
    fn no_looser(comp: Verdict, reference: Verdict) -> bool {
        match (comp, reference) {
            (Verdict::Denied, _) => true,
            (Verdict::Allowed(_), Verdict::Denied) => false,
            (Verdict::Allowed(c), Verdict::Allowed(r)) => c <= r,
        }
    }

    /// Inner commands, each with a `{P}` path placeholder: destroys, writes, reads, inert.
    const COMMANDS: &[&str] = &[
        "rm {P}", "rm -rf {P}", "rm -f {P}",
        "cp {P} /tmp/dest", "mv {P} /tmp/dest", "ln -sf {P} /tmp/l",
        "sed -i s/a/b/ {P}", "dd if={P} of=/tmp/o", "tee {P}", "truncate -s 0 {P}",
        "cat {P}", "head {P}", "tail {P}", "wc {P}", "grep foo {P}", "sort {P}",
        "echo {P}", "basename {P}", "dirname {P}",
    ];

    /// Loci spanning worktree → worktree-escape → temp → home → system → device/kernel.
    const LOCI: &[&str] = &[
        ".", "./src", "src", "sub/dir",
        "../..", "../sibling", "../../etc",
        "/tmp", "/tmp/sub",
        "~", "~/.ssh", "$HOME", "$HOME/.aws",
        "/etc", "/etc/ssh", "/usr", "/", "/var/log", "/bin", "/root",
        "/dev/sda", "/proc/1",
    ];

    fn target_path(loc: &str) -> String {
        format!("{}/f", loc.trim_end_matches('/'))
    }

    /// Path-projecting wrappers: the composition operates on a path AT `loc`. Whether the
    /// wrapper binds a placeholder (`find`) or passes the path through (`time`/`env`/…), the
    /// honest reference is the inner command run directly on a path at `loc`. `(label, comp)`.
    fn path_wraps(cmd_tpl: &str, loc: &str) -> Vec<(&'static str, String)> {
        let braces = cmd_tpl.replace("{P}", "{}");
        let direct = cmd_tpl.replace("{P}", &target_path(loc));
        vec![
            ("find -exec", format!("find {loc} -exec {braces} \\;")),
            ("find -exec +", format!("find {loc} -exec {braces} +")),
            ("find -execdir", format!("find {loc} -execdir {braces} \\;")),
            ("time", format!("time {direct}")),
            ("timeout", format!("timeout 5 {direct}")),
            ("nice", format!("nice {direct}")),
            ("ionice", format!("ionice {direct}")),
            ("nohup", format!("nohup {direct}")),
            ("env", format!("env FOO=1 {direct}")),
            ("dotenv", format!("dotenv {direct}")),
            ("sudo", format!("sudo {direct}")),
            ("bash -c", format!("bash -c '{direct}'")),
            ("sh -c", format!("sh -c '{direct}'")),
        ]
    }

    #[test]
    fn no_wrapper_launders_a_targeted_command() {
        let mut violations = Vec::new();
        let (mut denied_refs, mut allowed_comps, mut total) = (0u32, 0u32, 0u32);
        for cmd_tpl in COMMANDS {
            for loc in LOCI {
                let direct = cmd_tpl.replace("{P}", &target_path(loc));
                let reference = new_verdict(&direct);
                if reference == Verdict::Denied {
                    denied_refs += 1;
                }
                for (label, comp_cmd) in path_wraps(cmd_tpl, loc) {
                    let comp = new_verdict(&comp_cmd);
                    total += 1;
                    if comp.is_allowed() {
                        allowed_comps += 1;
                    }
                    if !no_looser(comp, reference) {
                        violations.push(format!(
                            "[{label}] `{comp_cmd}` = {comp}  >  direct `{direct}` = {reference}"
                        ));
                    }
                }
            }
        }
        assert!(
            violations.is_empty(),
            "{} composition(s) looser than operating directly on the target:\n{}",
            violations.len(),
            violations.join("\n")
        );
        // Non-vacuity: the property is only meaningful if the grid actually spans both denied
        // targets (system loci) and allowed compositions (worktree), across a wide sweep.
        assert!(
            total > 3000 && denied_refs > 100 && allowed_comps > 100,
            "harness went vacuous: total={total}, denied_refs={denied_refs}, allowed_comps={allowed_comps}"
        );
    }

    /// `xargs` items come from stdin — unknowable and unbounded. Its honest reference is the
    /// bare command (operand invisible): `xargs rm` ~ bare `rm` (denied), `xargs cat` ~ bare
    /// `cat` (reads stdin, allowed). The `-I` replacement string must not smuggle in a fake
    /// worktree operand.
    #[test]
    fn xargs_never_launders_stdin_items() {
        let mut violations = Vec::new();
        for cmd_tpl in COMMANDS {
            let bare = cmd_tpl
                .replace("{P}", "")
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ");
            let reference = new_verdict(&bare);
            let braces = cmd_tpl.replace("{P}", "{}");
            let wraps = [
                ("xargs", format!("xargs {bare}")),
                ("xargs -I{}", format!("xargs -I{{}} {braces}")),
                ("piped xargs -I{}", format!("find / | xargs -I{{}} {braces}")),
            ];
            for (label, comp_cmd) in wraps {
                let comp = new_verdict(&comp_cmd);
                if !no_looser(comp, reference) {
                    violations.push(format!(
                        "[{label}] `{comp_cmd}` = {comp}  >  bare `{bare}` = {reference}"
                    ));
                }
            }
        }
        assert!(
            violations.is_empty(),
            "{} xargs composition(s) launder invisible stdin items:\n{}",
            violations.len(),
            violations.join("\n")
        );
    }

    /// Absolute-deny invariants that do NOT reduce to the non-laundering property above: an
    /// authority escalation must deny even when its inner command is worktree-safe (`sudo rm
    /// ./x` is not "no looser than `rm ./x`" — it must simply deny), and an interpreter
    /// payload that executes (HP-7) is opaque to the flag parser. These are checked
    /// explicitly, not derived.
    const MUST_DENY: &[&str] = &[
        // authority escalation — runs the nested command as another user; never auto-approve,
        // regardless of how benign the inner command's target looks.
        "sudo rm ./x",
        "sudo cat ./notes.md",
        "sudo cat /etc/shadow",
        "sudo tee /etc/hosts",
        "sudo -u root rm ./x",
        "doas rm ./x",
        "sudo find . -exec cat {} \\;",
        // interpreter payloads that execute shell commands — the mini-language is opaque.
        "perl -e 'system(\"rm -rf /\")'",
        "perl -e 'unlink glob \"*\"'",
        "perl -pi -e 's/a/b/' file",
        "awk 'BEGIN{system(\"rm -rf /\")}'",
        "ruby -e 'system(\"rm -rf /\")'",
        "python3 -c 'import os; os.system(\"rm -rf /\")'",
        // unbounded / iconic destroyers that must never slip through a wrapper.
        "find / -exec rm -rf {} \\;",
        "find / -print0 | xargs -0 rm -rf",
        "grep -rl secret / | xargs rm",
        "xargs rm",
        "bash -c 'rm -rf /'",
        "bash -c 'rm -rf ~'",
        "eval 'rm -rf /'",
    ];

    #[test]
    fn absolute_deny_invariants_hold() {
        let leaked: Vec<_> = MUST_DENY
            .iter()
            .filter(|cmd| new_verdict(cmd).is_allowed())
            .collect();
        assert!(leaked.is_empty(), "these must-deny compositions auto-approve:\n{leaked:#?}");
    }

    /// Over-denial floor: the non-laundering property only guards the safety direction, so a
    /// blunt "deny all delegation" would pass it. These legit worktree compositions must keep
    /// auto-approving.
    const SHOULD_ALLOW: &[&str] = &[
        "find . -exec cat {} \\;",
        "find ./src -exec grep foo {} \\;",
        "find . -name '*.tmp' -exec rm {} \\;",
        "find . -exec rm {} \\;",
        "find src -exec head {} \\;",
        "time rm ./stale.log",
        "time cat ./notes.md",
        "nice cat ./x",
        "env FOO=1 cat ./x",
        "xargs -I {} cat {}",
        "xargs -I{} basename {}",
        "xargs cat",
        "xargs grep pattern",
    ];

    #[test]
    fn legit_worktree_compositions_still_auto_approve() {
        let broken: Vec<_> = SHOULD_ALLOW
            .iter()
            .filter(|cmd| !new_verdict(cmd).is_allowed())
            .collect();
        assert!(broken.is_empty(), "legit compositions are (over-)denied:\n{broken:#?}");
    }
}

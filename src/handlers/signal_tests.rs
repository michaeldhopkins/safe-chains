//! Signalling a process the command did not start is never approved, at any level.

use crate::verdict::Verdict;

const SIGNALS: &[&str] = &[
    "pkill -f \"cat\" -U $(id -u) -x", "pkill -f cat", "pkill foo", "pkill -9 -x node", "killall foo", "killall -s foo", "killall -l",
    "kill 123", "kill -9 123", "kill -TERM %1", "kill $!", "sleep 5 & kill $!", "echo 1 | xargs kill", "echo 1 | xargs -I{} kill {}",
    "pgrep node | xargs kill -9", "echo foo | xargs pkill", "echo foo | xargs killall",
];

#[test]
fn signalling_other_processes_is_refused_at_every_level() {
    for level in ["paranoid", "reader", "editor", "developer", "local-admin", "network-admin"] {
        let (ceiling, engine) = crate::level_ceiling(level).expect("known level");
        for cmd in SIGNALS {
            let v = crate::command_verdict_ceilinged(cmd, ceiling, engine);
            assert_eq!(v, Verdict::Denied, "{level}: `{cmd}` approved");
        }
    }
}

#[test]
fn looking_at_processes_stays_approved() {
    for cmd in ["pgrep -f cat", "kill -l", "kill -0 123", "ps aux"] {
        assert!(crate::is_safe_command(cmd), "`{cmd}` refused");
    }
}

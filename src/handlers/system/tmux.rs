use crate::parse::Token;
use crate::verdict::{SafetyLevel, Verdict};

static INERT_SUBS: &[&str] = &[
    "display", "display-message",
    "has", "has-session",
    "info",
    "list-buffers", "list-clients", "list-commands",
    "list-keys", "list-panes", "list-sessions", "list-windows",
    "ls", "lsb", "lsc", "lscm", "lsk", "lsp", "lsw",
    "show", "show-environment", "show-options",
    "showenv",
    "start", "start-server",
];

static SAFE_WRITE_SUBS: &[&str] = &[
    "a", "attach", "attach-session",
    "detach", "detach-client",
    "kill-pane", "kill-server", "kill-session", "kill-window",
    "killp", "killw",
    "new", "new-session", "new-window", "neww",
    "rename", "rename-session", "rename-window", "renamew",
    "resize-pane", "resize-window", "resizep", "resizew",
    "respawn-pane", "respawn-window", "respawnp", "respawnw",
    "select-pane", "select-window", "selectp", "selectw",
    "set", "set-environment", "set-option",
    "setenv",
    "split", "split-window", "splitw",
    "swap-pane", "swap-window", "swapp", "swapw",
    "switch", "switch-client", "switchc",
];

static DELEGATION_SUBS: &[&str] = &[
    "confirm", "confirm-before",
    "if", "if-shell",
    "pipe-pane", "pipep",
    "run", "run-shell",
];

fn is_inert_sub(s: &str) -> bool {
    INERT_SUBS.binary_search(&s).is_ok()
}

fn is_safe_write_sub(s: &str) -> bool {
    SAFE_WRITE_SUBS.binary_search(&s).is_ok()
}

fn is_delegation_sub(s: &str) -> bool {
    DELEGATION_SUBS.binary_search(&s).is_ok()
}

fn find_command_arg(tokens: &[Token], sub: &str) -> Option<usize> {
    match sub {
        "run-shell" | "run" => {
            let mut i = 1;
            while i < tokens.len() {
                let t = tokens[i].as_str();
                if t == "-b" {
                    i += 1;
                    continue;
                }
                if t == "-d" || t == "-t" {
                    i += 2;
                    continue;
                }
                if !t.starts_with('-') {
                    return Some(i);
                }
                return None;
            }
            None
        }
        "if-shell" | "if" => {
            let mut i = 1;
            while i < tokens.len() {
                let t = tokens[i].as_str();
                if t == "-b" {
                    i += 1;
                    continue;
                }
                if t == "-F" || t == "-t" {
                    i += 2;
                    continue;
                }
                if !t.starts_with('-') {
                    return Some(i);
                }
                return None;
            }
            None
        }
        "pipe-pane" | "pipep" => {
            let mut i = 1;
            while i < tokens.len() {
                let t = tokens[i].as_str();
                if t == "-I" || t == "-O" {
                    i += 1;
                    continue;
                }
                if t == "-o" || t == "-t" {
                    i += 2;
                    continue;
                }
                if !t.starts_with('-') {
                    return Some(i);
                }
                return None;
            }
            None
        }
        "confirm-before" | "confirm" => {
            let mut i = 1;
            while i < tokens.len() {
                let t = tokens[i].as_str();
                if t == "-b" {
                    i += 1;
                    continue;
                }
                if t == "-p" || t == "-t" {
                    i += 2;
                    continue;
                }
                if !t.starts_with('-') {
                    return Some(i);
                }
                return None;
            }
            None
        }
        _ => None,
    }
}

pub fn is_safe_tmux(tokens: &[Token]) -> Verdict {
    if tokens.len() < 2 {
        return Verdict::Denied;
    }

    let mut cmd_idx = 1;
    while cmd_idx < tokens.len() {
        let t = tokens[cmd_idx].as_str();
        if t == "-S" || t == "-L" || t == "-f" {
            cmd_idx += 2;
            continue;
        }
        if t == "-l" || t == "-u" || t == "-v" || t == "-T" || t == "-N" {
            cmd_idx += 1;
            continue;
        }
        if matches!(t, "--help" | "-h" | "--version" | "-V") && cmd_idx + 1 >= tokens.len() {
            return Verdict::Allowed(SafetyLevel::Inert);
        }
        break;
    }

    if cmd_idx >= tokens.len() {
        return Verdict::Denied;
    }

    let sub = tokens[cmd_idx].as_str();

    if is_inert_sub(sub) {
        return Verdict::Allowed(SafetyLevel::Inert);
    }

    if is_safe_write_sub(sub) {
        return Verdict::Allowed(SafetyLevel::SafeWrite);
    }

    if is_delegation_sub(sub) {
        let rest = &tokens[cmd_idx..];
        if let Some(arg_idx) = find_command_arg(rest, sub) {
            let inner = rest[arg_idx].as_str();
            let v = crate::command_verdict(inner);
            if (sub == "if-shell" || sub == "if")
                && let Some(then_idx) = rest.get(arg_idx + 1) {
                let then_v = crate::command_verdict(then_idx.as_str());
                if !then_v.is_allowed() {
                    return Verdict::Denied;
                }
                if let Some(else_tok) = rest.get(arg_idx + 2) {
                    let else_v = crate::command_verdict(else_tok.as_str());
                    return v.combine(then_v).combine(else_v);
                }
                return v.combine(then_v);
            }
            return v;
        }
        return Verdict::Denied;
    }

    if sub == "send-keys" || sub == "send" {
        return Verdict::Allowed(SafetyLevel::SafeWrite);
    }

    Verdict::Denied
}

pub(crate) fn dispatch(cmd: &str, tokens: &[Token]) -> Option<Verdict> {
    match cmd {
        "tmux" => Some(is_safe_tmux(tokens)),
        _ => None,
    }
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    vec![
        crate::docs::CommandDoc::handler("tmux",
            "https://man7.org/linux/man-pages/man1/tmux.1.html",
            "Read-only: list-sessions, list-windows, list-panes, list-clients, list-buffers, \
             list-keys, list-commands, show-options, show-environment, display-message, info, \
             has-session, start-server. \
             Session management (SafeWrite): new-session, kill-session, kill-window, kill-pane, \
             kill-server, attach-session, detach-client, switch-client, new-window, split-window, \
             select-window, select-pane, rename-session, rename-window, resize-pane, resize-window, \
             set-option, set-environment, send-keys. \
             Delegation: run-shell, if-shell, pipe-pane, confirm-before \
             (recursively validates inner commands)."),
    ]
}

#[cfg(test)]
pub(in crate::handlers::system) const REGISTRY: &[crate::handlers::CommandEntry] = &[
    crate::handlers::CommandEntry::Positional { cmd: "tmux" },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        tmux_ls: "tmux list-sessions",
        tmux_ls_short: "tmux ls",
        tmux_lsw: "tmux list-windows",
        tmux_lsw_short: "tmux lsw",
        tmux_lsp: "tmux list-panes",
        tmux_lsp_short: "tmux lsp",
        tmux_lsc: "tmux list-clients",
        tmux_lsb: "tmux list-buffers",
        tmux_lsk: "tmux list-keys",
        tmux_lscm: "tmux list-commands",
        tmux_show: "tmux show-options",
        tmux_show_short: "tmux show",
        tmux_showenv: "tmux show-environment",
        tmux_showenv_short: "tmux showenv",
        tmux_display: "tmux display-message",
        tmux_display_short: "tmux display",
        tmux_info: "tmux info",
        tmux_has: "tmux has-session",
        tmux_has_short: "tmux has",
        tmux_start: "tmux start-server",
        tmux_new_session: "tmux new-session",
        tmux_new_short: "tmux new",
        tmux_kill_session: "tmux kill-session",
        tmux_kill_window: "tmux kill-window",
        tmux_kill_pane: "tmux kill-pane",
        tmux_kill_server: "tmux kill-server",
        tmux_attach: "tmux attach-session",
        tmux_attach_short: "tmux attach",
        tmux_attach_a: "tmux a",
        tmux_detach: "tmux detach-client",
        tmux_switch: "tmux switch-client",
        tmux_neww: "tmux new-window",
        tmux_splitw: "tmux split-window",
        tmux_selectw: "tmux select-window",
        tmux_selectp: "tmux select-pane",
        tmux_rename: "tmux rename-session",
        tmux_renamew: "tmux rename-window",
        tmux_resizep: "tmux resize-pane",
        tmux_resizew: "tmux resize-window",
        tmux_set: "tmux set-option",
        tmux_setenv: "tmux set-environment",
        tmux_send_keys: "tmux send-keys",
        tmux_send: "tmux send",
        tmux_socket: "tmux -S /tmp/sock ls",
        tmux_label: "tmux -L test ls",
        tmux_run_safe: "tmux run-shell 'git status'",
        tmux_run_safe_short: "tmux run 'ls -la'",
        tmux_if_shell_safe: "tmux if-shell 'true' 'ls'",
        tmux_pipe_pane_safe: "tmux pipe-pane 'cat'",
        tmux_confirm_safe: "tmux confirm-before 'ls'",
        tmux_if_shell_format: "tmux if-shell -F '#{pane_in_mode}' 'ls'",
        tmux_pipe_pane_output: "tmux pipe-pane -o /tmp/log 'cat'",
        tmux_run_background: "tmux run-shell -b 'git status'",
        tmux_run_delay: "tmux run-shell -d 5 'ls'",
        tmux_help: "tmux --help",
        tmux_swap_pane: "tmux swap-pane",
        tmux_swap_window: "tmux swap-window",
        tmux_respawn_pane: "tmux respawn-pane",
        tmux_respawn_window: "tmux respawn-window",
    }

    denied! {
        tmux_bare_denied: "tmux",
        tmux_source_denied: "tmux source-file ~/.tmux.conf",
        tmux_run_unsafe_denied: "tmux run-shell 'rm -rf /'",
        tmux_if_shell_unsafe_denied: "tmux if-shell 'true' 'rm -rf /'",
        tmux_pipe_pane_unsafe_denied: "tmux pipe-pane 'rm -rf /'",
        tmux_confirm_unsafe_denied: "tmux confirm-before 'rm -rf /'",
        tmux_run_no_cmd_denied: "tmux run-shell",
        tmux_if_shell_format_unsafe_denied: "tmux if-shell -F '#{cond}' 'rm -rf /'",
        tmux_pipe_pane_output_unsafe_denied: "tmux pipe-pane -o /tmp/log 'rm -rf /'",
        tmux_unknown_denied: "tmux load-buffer foo",
    }

    inert! {
        level_tmux_ls: "tmux ls",
        level_tmux_info: "tmux info",
        level_tmux_show: "tmux show-options",
    }

    safe_write! {
        level_tmux_new: "tmux new-session",
        level_tmux_kill: "tmux kill-session",
        level_tmux_attach: "tmux attach-session",
        level_tmux_send: "tmux send-keys",
    }
}

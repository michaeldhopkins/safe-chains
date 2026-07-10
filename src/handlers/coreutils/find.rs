use crate::parse::{Token, WordSet};
use crate::verdict::{SafetyLevel, Verdict};

static FIND_DANGEROUS_FLAGS: WordSet = WordSet::new(&[
    "-delete",
    "-fls",
    "-fprint",
    "-fprint0",
    "-fprintf",
    "-ok",
    "-okdir",
]);

pub(in crate::handlers::coreutils) fn is_safe_find(tokens: &[Token]) -> Verdict {
    // find's `{}` placeholder is substituted with each traversed path, which lives UNDER
    // find's path operand. So the nested command's locus is find's traversal scope, not a
    // bare worktree name: `find /etc -exec rm {}` deletes /etc files, not `./file`. Bind `{}`
    // to a path under each find operand and delegate once per operand (deny-absorbing), so a
    // system/home traversal denies. Default operand `.` when none is given (find's default).
    let bases: Vec<&str> = {
        let leading: Vec<&str> = tokens[1..]
            .iter()
            .take_while(|t| !t.as_str().starts_with('-'))
            .map(Token::as_str)
            .collect();
        if leading.is_empty() { vec!["."] } else { leading }
    };

    let mut level = SafetyLevel::Inert;
    let mut i = 1;
    while i < tokens.len() {
        if FIND_DANGEROUS_FLAGS.contains(&tokens[i]) {
            return Verdict::Denied;
        }
        if tokens[i] == "-exec" || tokens[i] == "-execdir" {
            let cmd_start = i + 1;
            let cmd_end = tokens[cmd_start..]
                .iter()
                .position(|t| *t == ";" || *t == "+")
                .map(|p| cmd_start + p)
                .unwrap_or(tokens.len());
            if cmd_start >= cmd_end {
                return Verdict::Denied;
            }
            for base in &bases {
                let bound = format!("{}/f", base.trim_end_matches('/'));
                let exec_words: Vec<String> = tokens[cmd_start..cmd_end]
                    .iter()
                    .map(|t| t.as_str().replace("{}", &bound))
                    .collect();
                match crate::command_verdict(&shell_words::join(&exec_words)) {
                    Verdict::Denied => return Verdict::Denied,
                    Verdict::Allowed(l) => level = level.max(l),
                }
            }
            i = cmd_end + 1;
            continue;
        }
        i += 1;
    }
    Verdict::Allowed(level)
}

pub(in crate::handlers::coreutils) fn dispatch(cmd: &str, tokens: &[Token]) -> Option<Verdict> {
    match cmd {
        "find" => Some(is_safe_find(tokens)),
        _ => None,
    }
}

pub(in crate::handlers::coreutils) fn command_docs() -> Vec<crate::docs::CommandDoc> {
    vec![
        crate::docs::CommandDoc::handler("find",
            "https://www.gnu.org/software/findutils/manual/html_mono/find.html",
            "Positional predicates allowed. \
             -exec/-execdir allowed when the executed command is itself safe.",
            "fs"),
    ]
}

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        find_name: "find . -name '*.rb'",
        find_type_name: "find . -type f -name '*.py'",
        find_maxdepth: "find /tmp -maxdepth 2",
        find_print: "find . -name '*.log' -print",
        find_print0: "find . -name '*.log' -print0",
        find_exec_grep_l: "find . -name '*.rb' -exec grep -l pattern {} \\;",
        find_exec_grep_l_plus: "find . -name '*.rb' -exec grep -l pattern {} +",
        find_exec_cat: "find . -exec cat {} \\;",
        find_execdir_cat: "find . -execdir cat {} \\;",
        find_execdir_grep: "find . -execdir grep pattern {} \\;",
        find_exec_grep_safe: "find . -name '*.py' -exec grep pattern {} +",
        find_exec_nested_bash_safe: "find . -exec bash -c 'git status' \\;",
    }

    denied! {
        find_delete_denied: "find . -name '*.tmp' -delete",
        find_exec_rm: "find . -exec rm {} \\;",
        find_exec_rm_rf: "find . -exec rm -rf {} +",
        find_execdir_unsafe_denied: "find . -execdir rm {} \\;",
        find_ok_denied: "find . -ok rm {} \\;",
        find_okdir_denied: "find . -okdir rm {} \\;",
        find_exec_nested_bash_chain_denied: "find . -exec bash -c 'ls && rm -rf /' \\;",
        find_type_delete_denied: "find . -type f -name '*.bak' -delete",
        find_fprint_denied: "find . -fprint /tmp/list.txt",
        find_fprint0_denied: "find . -fprint0 /tmp/list.txt",
        find_fls_denied: "find . -fls /tmp/list.txt",
        find_fprintf_denied: "find . -fprintf /tmp/list.txt '%p'",
    }
}

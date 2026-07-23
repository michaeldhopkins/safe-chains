use crate::parse::{Token, WordSet};
use crate::verdict::{SafetyLevel, Verdict};

// find's expression is an ALLOWLIST of READ-ONLY primaries: tests (`-name`/`-type`/`-size`/…),
// read-only actions (`-print`/`-ls`/`-prune`/…), positional + global options, and operators. A
// primary NOT listed — `-delete`, `-ok`/`-okdir`, `-fprint*`/`-fls`, or any new/BSD write primary —
// denies by OMISSION (fail closed), where the old denylist failed open on anything it hadn't
// enumerated. `-exec`/`-execdir` are handled separately (delegate to the inner command). A VALUED
// primary consumes its next token, so a `-`-prefixed VALUE (`-mtime -7`, `-perm -644`) is not
// mistaken for a primary and a filename that looks like an action (`find . -name -delete`) stays a
// value. SAFETY (as in mlr): a value-taking primary MUST be in VALUED, never STANDALONE, or the
// walk would fail to skip its value — over-denying at best, or (if the value were an action)
// swallowing it silently.

/// Read-only find primaries that take NO value (tests, read-only actions, positional/global
/// options, operators, and `--help`/`--version`).
static FIND_SAFE_STANDALONE: WordSet = WordSet::new(&[
    "--help", "--version",
    "-H", "-L", "-P",
    "-a", "-and", "-d", "-daystart", "-depth",
    "-empty", "-executable", "-false", "-follow", "-help",
    "-ignore_readdir_race", "-ls", "-mount",
    "-nogroup", "-noignore_readdir_race", "-noleaf", "-not", "-nouser", "-nowarn",
    "-o", "-or",
    "-print", "-print0", "-prune", "-quit", "-readable", "-true", "-version",
    "-warn", "-writable", "-xdev",
]);

/// Read-only find primaries that consume the NEXT token as a value. `-newer` / `-newerXY` are
/// handled by prefix below. Every entry is read-only (no `-fprintf`, which writes a file).
static FIND_SAFE_VALUED: WordSet = WordSet::new(&[
    "-D",
    "-amin", "-anewer", "-atime",
    "-cmin", "-cnewer", "-context", "-ctime",
    "-fstype",
    "-gid", "-group",
    "-ilname", "-iname", "-inum", "-ipath", "-iregex", "-iwholename",
    "-links", "-lname",
    "-maxdepth", "-mindepth", "-mmin", "-mtime",
    "-name",
    "-path", "-perm", "-printf",
    "-regex", "-regextype",
    "-samefile", "-size",
    "-type",
    "-uid", "-used", "-user",
    "-wholename", "-xtype",
]);

pub(in crate::handlers::coreutils) fn is_safe_find(tokens: &[Token]) -> Verdict {
    // find's `{}` placeholder is substituted with each traversed path, which lives UNDER
    // find's path operand. So the nested command's locus is find's traversal scope, not a
    // bare worktree name: `find /etc -exec rm {}` deletes /etc files, not `./file`. Bind `{}`
    // to a path under each find operand and delegate once per operand (deny-absorbing), so a
    // system/home traversal denies. Default operand `.` when none is given (find's default).
    let bases: Vec<&str> = {
        // Skip the leading GLOBAL options (`-H`/`-L`/`-P`/`-D debugopts`/`-O[level]`), which
        // precede the path operand — otherwise `find -L /etc -exec …` stops at `-L`, defaults
        // the base to the cwd, and binds `{}` to a *worktree* path while find actually
        // traverses /etc. Collect ALL leading path operands (deny-absorbing below), so even a
        // mis-counted option arg still surfaces the real path.
        let mut i = 1;
        while i < tokens.len() {
            match tokens[i].as_str() {
                "-H" | "-L" | "-P" => i += 1,
                "-D" => i += 2,
                s if s.starts_with("-O") => i += 1,
                _ => break,
            }
        }
        // `-D` advances by 2 (option + debugopts arg); a TRAILING `-D` with no arg pushes `i` past
        // the end, so slice with `get` rather than `tokens[i..]` (which would panic out of range).
        let leading: Vec<&str> = tokens
            .get(i..)
            .unwrap_or(&[])
            .iter()
            .take_while(|t| !t.as_str().starts_with('-'))
            .map(Token::as_str)
            .collect();
        if leading.is_empty() { vec!["."] } else { leading }
    };

    let mut level = SafetyLevel::Inert;
    let mut i = 1;
    while i < tokens.len() {
        let s = tokens[i].as_str();
        // `-exec`/`-execdir`: delegate to the inner command, bound to each traversal base.
        if s == "-exec" || s == "-execdir" {
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
        // A path operand, a primary's value, or an operator (`(` `)` `!` `,`).
        if !s.starts_with('-') {
            i += 1;
            continue;
        }
        // Global query-optimisation level `-O2`/`-Ofind` (glued).
        if s.starts_with("-O") {
            i += 1;
            continue;
        }
        // A read-only VALUED primary consumes its value; `-newer`/`-newerXY` take a reference.
        if FIND_SAFE_VALUED.contains(&tokens[i]) || s.starts_with("-newer") {
            i += 2;
            continue;
        }
        if FIND_SAFE_STANDALONE.contains(&tokens[i]) {
            i += 1;
            continue;
        }
        // Anything else — `-delete`, `-ok*`, `-fprint*`/`-fls`, or an unknown/newer primary — denies.
        return Verdict::Denied;
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
            "Read-only predicates and actions allowed (tests like -name/-type/-size, -print/-ls/-prune, \
             operators, positional and global options). -exec/-execdir allowed when the executed \
             command is itself safe (each `{}` binds to the traversal path).",
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
        // engine-authoritative: deleting WORKTREE files via -exec is admitted at developer
        // ({} binds to the find path, so these stay inside the worktree).
        find_exec_rm: "find . -exec rm {} \\;",
        find_exec_rm_rf: "find . -exec rm -rf {} +",
        find_execdir_unsafe: "find . -execdir rm {} \\;",
        // Allowlist coverage: `-`-prefixed VALUES of valued primaries aren't mistaken for primaries.
        find_size_negative: "find . -size -1M",
        find_mtime_negative: "find . -mtime -7 -type f",
        find_perm_dash: "find . -perm -644",
        find_newer: "find . -newer ref.txt",
        find_newermt: "find . -newermt 2024-01-01",
        find_mindepth_maxdepth: "find . -mindepth 1 -maxdepth 3 -type d",
        find_empty: "find . -empty",
        find_not_operator: "find . -not -name '*.tmp'",
        find_or_operator: "find . -name '*.rb' -o -name '*.py'",
        find_printf: "find . -printf '%p\\n'",
        find_help: "find --help",
        // A filename that looks like an action is just the value of `-name` (the old denylist
        // false-DENIED this; the allowlist skips the value correctly).
        find_name_dash_delete_value: "find . -name -delete",
        // Regression (fuzzer, crash-*): a TRAILING `-D` (advances the leading-option scan by 2)
        // overran `tokens[i..]` and panicked the hook. All 6 crash inputs minimize to a bare `-D`
        // (or global opts) as the last token; each is a harmless no-op find that must not panic.
        find_trailing_D: "find -D",
        find_global_L_trailing_D: "find -L -D",
        find_global_H_P_trailing_D: "find -H -P -D",
    }

    denied! {
        find_delete_denied: "find . -name '*.tmp' -delete",
        // a leading global option must not hide the real path: base is /etc, not the cwd.
        find_global_opt_L_system: "find -L /etc -exec rm -rf {} \\;",
        find_global_opt_P_system: "find -P / -exec rm {} \\;",
        find_global_opt_O_system: "find -O3 /etc -exec rm {} \\;",
        find_global_opt_D_system: "find -D tree /etc -exec rm {} \\;",
        find_ok_denied: "find . -ok rm {} \\;",
        find_okdir_denied: "find . -okdir rm {} \\;",
        find_exec_nested_bash_chain_denied: "find . -exec bash -c 'ls && rm -rf /' \\;",
        find_type_delete_denied: "find . -type f -name '*.bak' -delete",
        find_fprint_denied: "find . -fprint /tmp/list.txt",
        find_fprint0_denied: "find . -fprint0 /tmp/list.txt",
        find_fls_denied: "find . -fls /tmp/list.txt",
        find_fprintf_denied: "find . -fprintf /tmp/list.txt '%p'",
        // Allowlist fail-closed wins: an unknown/newer/BSD primary denies by OMISSION (the old
        // denylist would have allowed anything it hadn't enumerated).
        find_unknown_primary_denied: "find . -frobnicate",
        find_delete_after_valued_denied: "find . -mtime -7 -delete",
        find_unknown_action_after_test_denied: "find . -type f -flushcache",
    }
}

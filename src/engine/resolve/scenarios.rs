//! HP-20 end-to-end path scenarios — the region model integrated with the engine, exercised
//! through `command_verdict` in forced `new` mode over the kinds of commands mac and linux
//! users actually run. Every set runs on every host: `with_os` forces the classifier's
//! platform so the linux and macOS scenarios are both validated regardless of where the
//! suite runs (rather than `cfg`-gating half of them away).

#[cfg(test)]
mod tests {
    use crate::command_verdict;
    use crate::engine::bridge::{with_mode, Mode};
    use crate::engine::resolve::regions::with_os;

    fn allows_on(os: &'static str, cmd: &str) -> bool {
        with_os(os, || with_mode(Mode::New, || command_verdict(cmd)).is_allowed())
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
    fn reads_public_configs_denies_secrets_and_private() {
        check(
            &[
                "cat /etc/hosts",
                "cat /etc/os-release",
                "head -n 5 /etc/services",
                "tail /etc/group",
                "wc -l /etc/passwd",
                "grep localhost /etc/hosts",
                "grep -n foo /etc/protocols",
                "cat /usr/bin/python3",
                "cat /usr/share/doc/bash/README",
                "cat /usr/local/lib/thing.so",
                "cat /etc/ssl/certs/ca-certificates.crt",
                "cat ./notes.md",
                "grep -r TODO ./src",
                "cat /tmp/scratch.txt",
            ],
            &[
                "cat /etc/shadow",
                "cat /etc/ssl/private/server.key",
                "cat ~/.ssh/id_rsa",
                "cat ~/.aws/credentials",
                "cat ~/.gnupg/secring.gpg",
                "cat ~/.netrc",
                "cat ~/.kube/config",
                "cat ~/.docker/config.json",
                "grep token ~/.config/gh/hosts.yml",
                "cat ~/notes.txt",
                "cat ~/Documents/taxes.pdf",
                "cat ~/.bashrc",
                "cat /etc/sudoers",
                "cat /var/lib/mysql/ibdata1",
                "cat /srv/app/secret.db",
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
                "cp /etc/hosts ./hosts.bak",
                "cp /etc/os-release ./os.txt",
                "ln -s /etc/hosts ./hosts",
                "dd if=/etc/hosts of=/tmp/h",
            ],
            &[
                "cp ~/.ssh/id_rsa ./stolen",
                "cp /etc/shadow ./x",
                "ln -s /etc/shadow ./x",
                "ln ~/.aws/credentials ./x",
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
    fn linux_kernel_info_logs_and_devices() {
        check_os(
            "linux",
            &[
                "cat /proc/cpuinfo",
                "cat /proc/meminfo",
                "cat /proc/loadavg",
                "cat /proc/mounts",
                "cat /proc/sys/net/ipv4/ip_forward",
                "cat /sys/class/net/eth0/address",
                "cat /var/log/syslog",
                "tail -n 100 /var/log/dmesg",
                "cat /etc/lsb-release",
            ],
            &[
                "cat /proc/self/environ",
                "cat /proc/1/mem",
                "cat /proc/1/cmdline",
                "cat /var/log/auth.log",
                "cat /var/log/secure",
                "grep root /var/log/audit/audit.log",
                "sed -i s/0/1/ /proc/sys/net/ipv4/ip_forward",
                "dd if=./a of=/dev/sda",
                "dd if=/dev/nvme0n1 of=./img",
            ],
        );
    }

    #[test]
    fn macos_system_trees_and_home() {
        check_os(
            "macos",
            &[
                "cat /System/Library/CoreServices/SystemVersion.plist",
                "cat /Library/Preferences/com.apple.loginwindow.plist",
                "cat /private/etc/hosts",
                "cat /usr/bin/swift",
            ],
            &[
                "cat ~/Library/Keychains/login.keychain-db",
                "cat ~/Library/Preferences/secrets.plist",
                "cat /Users/someone/notes.txt",
                "cat /etc/master.passwd",
                "cat /private/etc/master.passwd",
                "rm /System/Library/x",
                "touch /Library/LaunchDaemons/evil.plist",
                "dd if=/dev/rdisk0 of=./img",
            ],
        );
    }

    /// Sanity: `/proc` is linux-only. The SAME command flips by platform — proof the OS scope
    /// is actually consulted, not incidental.
    #[test]
    fn os_scope_is_load_bearing() {
        assert!(allows_on("linux", "cat /proc/cpuinfo"), "linux: /proc readable");
        assert!(!allows_on("macos", "cat /proc/cpuinfo"), "macos: /proc is unknown → deny");
        assert!(allows_on("macos", "cat /System/Library/x"), "macos: /System readable");
        assert!(!allows_on("linux", "cat /System/Library/x"), "linux: /System is unknown → deny");
    }
}

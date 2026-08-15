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

    /// Reads reach the machine rung; the SHIELD is what stops them, not the workspace boundary.
    ///
    /// This is the inverse of what it asserted before. Confining reads to the worktree was
    /// costing far more than it bought: agents legitimately read `~/.zshrc`, a sibling checkout,
    /// `/etc/hosts`, a vendored crate source, and every one of those was a prompt. What actually
    /// needs defending is a much smaller set — the credential stores — so that is what is
    /// defended, and it is defended by NAME rather than by rung.
    #[test]
    fn reads_reach_the_machine_rung_and_stop_at_the_shield() {
        check(
            &[
                "cat ./notes.md",
                "grep -r TODO ./src",
                "cat /tmp/scratch.txt",
                // ordinary machine-rung files: config, binaries, trust stores
                "cat /etc/hosts",
                "cat /etc/passwd",
                "cat /usr/bin/python3",
                "cat /etc/ssl/certs/ca-certificates.crt",
                // ordinary home files, dotfiles included
                "cat ~/notes.txt",
                "cat ~/Documents/taxes.pdf",
                "cat ~/.bashrc",
            ],
            &[
                // secrets — denied and un-grantable (the shield)
                "cat /etc/shadow",
                "cat ~/.ssh/id_rsa",
                "cat ~/.aws/credentials",
                "cat ~/.gnupg/secring.gpg",
                "cat ~/.netrc",
                "cat ~/.kube/config",
                "cat ~/.docker/config.json",
                // another user's home is private whatever its rung
                "cat /root/.bashrc",
                // unpinnable: we cannot tell WHICH file, so the shield cannot clear it
                "cat $SECRET",
                // climbing out does not launder the name it lands on
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
                "cp /etc/hosts ./hosts.bak",
            ],
            &[
                // a transfer is gated on its SOURCE's name, not its rung: copying an ordinary
                // machine file in is fine, copying a credential store in is the same theft it
                // would be to cat it — the worktree copy is readable afterwards.
                "cp ~/.ssh/id_rsa ./stolen",
                "cp /etc/shadow ./x",
                "dd if=~/.ssh/id_rsa of=./x",
                // raw devices are their own rung, above where reads reach
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

    /// Host introspection reads; the per-PROCESS half of /proc does not. `/proc/cpuinfo` describes
    /// the machine, `/proc/<pid>/environ` describes a process — and a process's environment is
    /// where this agent's own API tokens are sitting.
    #[test]
    fn linux_introspection_reads_but_process_state_does_not() {
        check_os(
            "linux",
            &[
                "cat ./notes.md",
                "cat /tmp/x",
                "cat /proc/cpuinfo",
                "cat /proc/sys/net/ipv4/ip_forward",
                "cat /sys/class/net/eth0/address",
                "cat /var/log/syslog",
            ],
            &[
                "cat /proc/self/environ",
                "cat /proc/1234/environ",
                "cat /proc/self/mem",
                "cat /proc/1234/cmdline",
                // auth logs carry credentials outright — a password mistyped at a username
                // prompt is written to them in the clear
                "cat /var/log/auth.log",
                "cat /var/log/secure",
                "dd if=./a of=/dev/sda",
            ],
        );
    }

    /// The same split on macOS: system property lists and shipped binaries read, the keychain and
    /// the password store do not, and writing a LaunchDaemon is still installing a service.
    #[test]
    fn macos_system_reads_but_the_keychain_does_not() {
        check_os(
            "macos",
            &[
                "cat ./notes.md",
                "cat /private/tmp/x",
                "cat /System/Library/CoreServices/SystemVersion.plist",
                "cat /Library/Preferences/com.apple.loginwindow.plist",
                "cat /usr/bin/swift",
            ],
            &[
                "cat ~/Library/Keychains/login.keychain-db",
                "cat /etc/master.passwd",
                // the firmlinked spelling of the same file
                "cat /private/etc/master.passwd",
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

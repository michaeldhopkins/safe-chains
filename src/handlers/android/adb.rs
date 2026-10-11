use crate::parse::{Token, WordSet};
use crate::verdict::{SafetyLevel, Verdict};

static SAFE_BARE_SUBS: WordSet = WordSet::new(&[
    "devices", "get-serialno", "get-state", "help", "start-server", "version",
]);

static SHELL_BARE_CMDS: WordSet = WordSet::new(&[
    "df", "getprop", "id", "ps", "uname", "whoami",
]);

static SHELL_PREFIX_CMDS: WordSet = WordSet::new(&[
    "cat", "dumpsys", "ls", "pm", "settings", "wm",
]);

static LOGCAT_STANDALONE: WordSet = WordSet::new(&[
    "--clear", "-d",
]);

static LOGCAT_VALUED: WordSet = WordSet::new(&[
    "--pid", "-b", "-e", "-t", "-v",
]);

fn check_shell(tokens: &[Token], offset: usize) -> Verdict {
    let Some(cmd) = tokens.get(offset) else {
        return Verdict::Denied;
    };
    if SHELL_BARE_CMDS.contains(cmd) {
        return Verdict::Allowed(SafetyLevel::Inert);
    }
    if !SHELL_PREFIX_CMDS.contains(cmd) {
        return Verdict::Denied;
    }
    let ok = match cmd.as_str() {
        "cat" | "ls" | "dumpsys" => true,
        "pm" => {
            let sub = tokens.get(offset + 1);
            sub.is_some_and(|s| s == "list" || s == "path")
        }
        "settings" => tokens.get(offset + 1).is_some_and(|s| s == "get"),
        "wm" => {
            tokens.get(offset + 1).is_some_and(|s| s == "size" || s == "density")
                && tokens.get(offset + 2).is_none()
        }
        _ => false,
    };
    if ok { Verdict::Allowed(SafetyLevel::Inert) } else { Verdict::Denied }
}

fn check_logcat(tokens: &[Token], start: usize) -> Verdict {
    let mut has_d = false;
    let mut i = start;
    while i < tokens.len() {
        let t = &tokens[i];
        if !t.starts_with('-') {
            i += 1;
            continue;
        }
        if t == "-d" {
            has_d = true;
            i += 1;
            continue;
        }
        if LOGCAT_STANDALONE.contains(t) {
            i += 1;
            continue;
        }
        if LOGCAT_VALUED.contains(t) {
            if i + 1 >= tokens.len() {
                return Verdict::Denied;
            }
            i += 2;
            continue;
        }
        if let Some((flag, _)) = t.as_str().split_once('=')
            && LOGCAT_VALUED.contains(flag)
        {
            i += 1;
            continue;
        }
        return Verdict::Denied;
    }
    if has_d { Verdict::Allowed(SafetyLevel::Inert) } else { Verdict::Denied }
}

pub fn is_safe_adb(tokens: &[Token]) -> Verdict {
    if tokens.len() < 2 {
        return Verdict::Denied;
    }
    let mut i = 1;
    if tokens.get(i).is_some_and(|t| t == "-s") {
        i += 2;
    }
    let Some(sub) = tokens.get(i) else {
        return Verdict::Denied;
    };
    if tokens.len() == i + 1 && (sub == "--help" || sub == "-h" || sub == "--version" || sub == "-V") {
        return Verdict::Allowed(SafetyLevel::Inert);
    }
    match sub.as_str() {
        s if SAFE_BARE_SUBS.contains(s) => Verdict::Allowed(SafetyLevel::Inert),
        "forward" | "reverse" => {
            if tokens.get(i + 1).is_some_and(|t| t == "--list") && tokens.len() == i + 2 {
                Verdict::Allowed(SafetyLevel::Inert)
            } else {
                Verdict::Denied
            }
        }
        "logcat" => check_logcat(tokens, i + 1),
        "shell" => check_shell(tokens, i + 1),
        _ => Verdict::Denied,
    }
}

pub(crate) fn dispatch(cmd: &str, tokens: &[Token]) -> Option<Verdict> {
    if cmd == "adb" {
        Some(is_safe_adb(tokens))
    } else {
        None
    }
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    use crate::docs::CommandDoc;
    vec![
        CommandDoc::handler("adb",
            "https://developer.android.com/tools/adb",
            "Bare subcommands: devices, get-serialno, get-state, help, start-server, version. \
             forward --list, reverse --list. \
             logcat (requires -d). \
             shell: cat, df, dumpsys, getprop, id, ls, pm list/path, ps, settings get, \
             uname, whoami, wm size/density. \
             Prefix flag -s SERIAL is skipped.",
            "android"),
    ]
}

#[cfg(test)]
pub(in crate::handlers::android) const REGISTRY: &[crate::handlers::CommandEntry] = &[];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        adb_devices: "adb devices",
        adb_version: "adb version",
        adb_get_state: "adb get-state",
        adb_get_serialno: "adb get-serialno",
        adb_help: "adb help",
        adb_start_server: "adb start-server",
        adb_forward_list: "adb forward --list",
        adb_reverse_list: "adb reverse --list",
        adb_serial_devices: "adb -s emulator-5554 devices",
        adb_logcat_d: "adb logcat -d",
        adb_logcat_d_verbose: "adb logcat -d -v time",
        adb_logcat_d_buffer: "adb logcat -d -b main",
        adb_logcat_d_pid: "adb logcat -d --pid 1234",
        adb_shell_getprop: "adb shell getprop ro.build.version.sdk",
        adb_shell_pm_list: "adb shell pm list packages",
        adb_shell_pm_path: "adb shell pm path com.example.app",
        adb_shell_dumpsys: "adb shell dumpsys battery",
        adb_shell_settings_get: "adb shell settings get global airplane_mode_on",
        adb_shell_id: "adb shell id",
        adb_shell_whoami: "adb shell whoami",
        adb_shell_uname: "adb shell uname",
        adb_shell_df: "adb shell df",
        adb_shell_ps: "adb shell ps",
        adb_shell_cat: "adb shell cat /proc/cpuinfo",
        adb_shell_ls: "adb shell ls /data/local/tmp",
        adb_shell_wm_size: "adb shell wm size",
        adb_shell_wm_density: "adb shell wm density",
        adb_serial_shell_getprop: "adb -s emulator-5554 shell getprop",
    }

    denied! {
        adb_bare_denied: "adb",
        adb_install_denied: "adb install app.apk",
        adb_uninstall_denied: "adb uninstall com.example.app",
        adb_push_denied: "adb push local.txt /sdcard/",
        adb_pull_denied: "adb pull /sdcard/file.txt .",
        adb_reboot_denied: "adb reboot",
        adb_root_denied: "adb root",
        adb_remount_denied: "adb remount",
        adb_kill_server_denied: "adb kill-server",
        adb_shell_rm_denied: "adb shell rm /sdcard/file.txt",
        adb_shell_am_start_denied: "adb shell am start -n com.example/.Main",
        adb_shell_am_force_stop_denied: "adb shell am force-stop com.example",
        adb_shell_pm_clear_denied: "adb shell pm clear com.example",
        adb_shell_input_denied: "adb shell input keyevent 3",
        adb_logcat_no_d_denied: "adb logcat",
        adb_logcat_unknown_flag_denied: "adb logcat -d --unknown",
        adb_logcat_trailing_valued_denied: "adb logcat -d -v",
        adb_forward_no_list_denied: "adb forward tcp:8080 tcp:8080",
        adb_shell_bare_denied: "adb shell",
        adb_shell_settings_put_denied: "adb shell settings put global airplane_mode_on 1",
        adb_shell_wm_reset_denied: "adb shell wm size reset",
    }

    use proptest::prelude::*;
    use crate::parse::Token;
    use crate::verdict::Verdict;

    proptest! {
        /// `-s SERIAL` is a device-selector prefix that the handler MUST skip without affecting
        /// the verdict on the rest of the line. Synthesising a random serial and arbitrary
        /// trailing tokens, then comparing the verdict of `[adb, -s, serial, rest...]` against
        /// `[adb, rest...]`, guards the prefix-skip code path: any regression that either fails
        /// to skip the prefix (consuming the serial as the subcommand) or over-skips it (eating
        /// the real subcommand as a second serial) changes one verdict without the other.
        #[test]
        fn a_dash_s_prefix_serial_does_not_change_the_verdict(
            serial in "[A-Za-z0-9._:-]{1,20}",
            rest in proptest::collection::vec(any::<String>(), 0..6),
        ) {
            let rest_tokens: Vec<Token> = rest
                .iter()
                .map(|s| Token::from_test(s))
                .collect();
            let mut with_prefix: Vec<Token> = vec![
                Token::from_test("adb"),
                Token::from_test("-s"),
                Token::from_test(&serial),
            ];
            with_prefix.extend(rest_tokens.iter().cloned());

            let mut plain: Vec<Token> = vec![Token::from_test("adb")];
            plain.extend(rest_tokens);

            let with_verdict = super::is_safe_adb(&with_prefix);
            let plain_verdict = super::is_safe_adb(&plain);
            prop_assert_eq!(
                with_verdict,
                plain_verdict,
                "serial prefix must not alter verdict: serial={:?} rest={:?}",
                serial,
                rest
            );
        }

        /// Every bare shell command in `SHELL_BARE_CMDS` (df, getprop, id, ps, uname, whoami)
        /// is inert — `check_shell` returns as soon as it recognises the bare name and ignores
        /// whatever else follows. Suffixes that would change the verdict for any OTHER shell
        /// command (random extra args, args that look like sub-subcommands, anything) must leave
        /// the bare-command verdict untouched. Pinning this guards against a future change that
        /// adds a length check or a "max positional" gate to the bare branch.
        #[test]
        fn a_shell_bare_command_is_inert_regardless_of_trailing_args(
            cmd in proptest::sample::select(super::SHELL_BARE_CMDS.iter().collect::<Vec<_>>()),
            suffix in proptest::collection::vec("[a-zA-Z0-9._/=:-]{1,16}", 0..8),
        ) {
            let mut tokens: Vec<Token> = vec![
                Token::from_test("adb"),
                Token::from_test("shell"),
                Token::from_test(cmd),
            ];
            for s in &suffix {
                tokens.push(Token::from_test(s));
            }
            prop_assert_eq!(
                super::is_safe_adb(&tokens),
                Verdict::Allowed(crate::verdict::SafetyLevel::Inert),
                "bare shell command must stay inert: cmd={} suffix={:?}",
                cmd,
                suffix
            );
        }

        /// `dispatch` is the table-routing facade — `Some(v)` exactly when the command name is
        /// `"adb"` and `v` equals `is_safe_adb(tokens)`; `None` for every other command name.
        /// This is the contract every other handler in the crate relies on, so a regression
        /// that double-routes (returning a verdict for non-adb commands) or drops the adb route
        /// breaks the whole command table; a property over a random command-name proves the
        /// bijection holds for ALL strings, not just the few the example tests enumerate.
        #[test]
        fn dispatch_routes_only_adb_to_is_safe_adb_and_returns_none_otherwise(
            name in any::<String>(),
            rest in proptest::collection::vec(any::<String>(), 0..4),
        ) {
            let tokens: Vec<Token> = rest.iter().map(|s| Token::from_test(s)).collect();
            let dispatched = super::dispatch(&name, &tokens);
            if name == "adb" {
                prop_assert_eq!(
                    dispatched,
                    Some(super::is_safe_adb(&tokens)),
                    "dispatch must delegate to is_safe_adb for cmd=adb"
                );
            } else {
                prop_assert_eq!(
                    dispatched,
                    None,
                    "dispatch must return None for non-adb cmd={:?}",
                    name
                );
            }
        }
    }
}

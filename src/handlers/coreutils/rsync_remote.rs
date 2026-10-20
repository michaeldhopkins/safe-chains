use crate::parse::Token;

/// What `-M` / `--remote-option` hand to the other rsync process. A local copy runs one too, so
/// the options reach this machine either way.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum RemoteOptions {
    None,
    /// A value that is missing, or is not itself an option, which nothing can judge.
    Refuse,
    /// The command with every remote option written as if it were given locally.
    Judge(Vec<Token>),
}

pub(crate) fn as_local_options(tokens: &[Token]) -> RemoteOptions {
    let mut spliced = Vec::with_capacity(tokens.len());
    let mut found = false;
    let mut words = tokens.iter();
    spliced.extend(words.next().cloned());
    while let Some(token) = words.next() {
        let word = token.as_str();
        if word == "--" {
            spliced.push(token.clone());
            spliced.extend(words.by_ref().cloned());
            break;
        }
        let value = if word == "--remote-option" {
            words.next().map(Token::as_str)
        } else if let Some(value) = word.strip_prefix("--remote-option=") {
            Some(value)
        } else if let Some(cluster) = word.strip_prefix('-').filter(|c| !c.starts_with('-') && c.contains('M')) {
            let (before, after) = cluster.split_at(cluster.find('M').unwrap_or(0));
            if !before.is_empty() {
                spliced.push(Token::from_raw(format!("-{before}")));
            }
            let glued = &after[1..];
            if glued.is_empty() { words.next().map(Token::as_str) } else { Some(glued) }
        } else {
            spliced.push(token.clone());
            continue;
        };
        found = true;
        match value {
            Some(option) if option.len() > 1 && option.starts_with('-') && !cannot_stand_alone(option) => {
                spliced.push(Token::from_raw(option.to_string()))
            }
            _ => return RemoteOptions::Refuse,
        }
    }
    if found { RemoteOptions::Judge(spliced) } else { RemoteOptions::None }
}

/// An option the other rsync could not take as written: one whose value would be the next word on
/// the line here but is missing there (`-M -e`, `-M --temp-dir`), or `--server`, which only rsync
/// itself passes.
fn cannot_stand_alone(option: &str) -> bool {
    let takes = |flag: &str| crate::registry::command_valued_flags("rsync").is_some_and(|valued| valued.iter().any(|v| v == flag));
    if option == "--server" {
        return true;
    }
    if option.starts_with("--") {
        return !option.contains('=') && takes(option);
    }
    let letters = &option[1..];
    letters
        .char_indices()
        .find(|(_, letter)| takes(&format!("-{letter}")))
        .is_some_and(|(at, letter)| at + letter.len_utf8() == letters.len())
}

pub(crate) fn refuses_its_remote_options(tokens: &[Token]) -> bool {
    match as_local_options(tokens) {
        RemoteOptions::None => false,
        RemoteOptions::Refuse => true,
        RemoteOptions::Judge(local) => !crate::handlers::dispatch(&local).is_allowed(),
    }
}

#[cfg(test)]
mod tests {
    use super::{RemoteOptions, as_local_options};
    use crate::is_safe_command;
    use crate::parse::Token;
    use proptest::prelude::*;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    fn words(line: &str) -> Vec<Token> {
        line.split(' ').map(Token::from_test).collect()
    }

    safe! {
        rsync_remote_plain_option: "rsync -a -M--fake-super src/ dst/",
        rsync_remote_spaced_option: "rsync -a -M --fake-super src/ dst/",
        rsync_remote_long_option: "rsync -a --remote-option=--fake-super src/ dst/",
        rsync_remote_long_spaced_option: "rsync -a --remote-option --fake-super src/ dst/",
        rsync_remote_plain_chmod: "rsync -a --remote-option=--chmod=go-w src/ dst/",
        rsync_remote_spaced_plain_chmod: "rsync -a -M --chmod=go-w src/ dst/",
        rsync_log_in_worktree: "rsync -a --log-file=./rsync.log src/ dst/",
        rsync_temp_dir_in_worktree: "rsync --temp-dir=./tmp src/ dst/",
        rsync_partial_dir_relative: "rsync --partial-dir=.rsync-partial src/ dst/",
    }

    denied! {
        rsync_remote_setuid_chmod: "rsync -a --remote-option=--chmod=u+s src/ dst/",
        rsync_remote_spaced_setuid_chmod: "rsync -a --remote-option --chmod=u+s src/ dst/",
        rsync_remote_short_setuid_chmod: "rsync -a -M --chmod=Fu+s src/ dst/",
        rsync_remote_glued_setuid_chmod: "rsync -a -M--chmod=g+s src/ dst/",
        rsync_remote_cluster_setuid_chmod: "rsync -aM--chmod=4755 src/ dst/",
        rsync_remote_log_outside: "rsync -a --remote-option=--log-file=/etc/rsync.log src/ dst/",
        rsync_remote_unknown_option: "rsync -a -M--xyzzy src/ dst/",
        rsync_remote_not_an_option: "rsync -a -M foo src/ dst/",
        rsync_remote_missing_value: "rsync -a src/ dst/ --remote-option",
        rsync_log_outside: "rsync -a --log-file=/etc/rsync.log src/ dst/",
        rsync_log_outside_spaced: "rsync -a --log-file /etc/rsync.log src/ dst/",
        rsync_batch_outside: "rsync -a --write-batch=/etc/batch src/ dst/",
        rsync_temp_dir_absolute: "rsync --temp-dir=/abs src/ dst/",
        rsync_temp_dir_spaced_absolute: "rsync --temp-dir /abs src/ dst/",
        rsync_temp_dir_short_absolute: "rsync -T/abs src/ dst/",
        rsync_partial_dir_absolute: "rsync --partial-dir=/abs src/ dst/",
        rsync_partial_dir_spaced_absolute: "rsync --partial-dir /abs src/ dst/",
        rsync_partial_dir_home: "rsync --partial-dir=~/x src/ dst/",
        rsync_remote_bare_shell_flag: "rsync -a -M -e src/ dst/",
        rsync_remote_bare_temp_flag: "rsync -a -M -T src/ dst/",
        rsync_remote_bare_long_flag: "rsync -a -M --temp-dir src/ dst/",
        rsync_remote_bare_cluster_flag: "rsync -a -M -ve src/ dst/",
        rsync_remote_server: "rsync -a -M--server src/ dst/",
    }

    #[test]
    fn a_cluster_keeps_the_letters_before_m() {
        assert_eq!(as_local_options(&words("rsync -avM--fake-super a b")), RemoteOptions::Judge(words("rsync -av --fake-super a b")));
        assert_eq!(as_local_options(&words("rsync -av a b")), RemoteOptions::None);
        assert_eq!(as_local_options(&words("rsync -a -- -Mx b")), RemoteOptions::None);
    }

    /// A value-taking option handed to the other rsync without its value would take the next word
    /// here, so the line judged would not be the line run.
    #[test]
    fn a_remote_option_missing_its_value_is_never_approved() {
        let manifest: toml::Value = toml::from_str(include_str!("../../../commands/tools/rsync.toml")).expect("rsync.toml parses");
        let valued = manifest["command"][0]["valued"].as_array().expect("rsync declares valued flags");
        assert!(valued.len() > 10, "the valued list was not read");
        for flag in valued.iter().filter_map(toml::Value::as_str) {
            for spelling in
                [format!("-M {flag}"), format!("-M{flag}"), format!("--remote-option={flag}"), format!("--remote-option {flag}")]
            {
                let line = format!("rsync -a {spelling} src/ dst/");
                assert!(!is_safe_command(&line), "{line} must not be approved");
            }
        }
    }

    proptest! {
        /// However a remote option is spelled, it is judged as the same option given locally, and
        /// never approved where that option would be refused.
        #[test]
        fn a_remote_option_is_judged_as_the_same_option_given_locally(
            option in prop::sample::select(vec![
                "--fake-super", "--chmod=go-w", "--chmod=u+s", "--chmod=F4755", "--log-file=./x.log",
                "--log-file=/etc/x", "--numeric-ids", "--xyzzy", "--write-batch=/tmp/b",
            ]),
            spelling in 0usize..4,
        ) {
            let remote = match spelling {
                0 => format!("-M{option}"),
                1 => format!("-M {option}"),
                2 => format!("--remote-option={option}"),
                _ => format!("--remote-option {option}"),
            };
            let local = is_safe_command(&format!("rsync -a {option} src/ dst/"));
            let approved = is_safe_command(&format!("rsync -a {remote} src/ dst/"));
            if spelling == 0 {
                // A value glued to a short option is never read past an `=`, so it may refuse more.
                prop_assert!(!approved || local, "{}", remote);
            } else {
                prop_assert_eq!(approved, local, "{}", remote);
            }
        }
    }
}

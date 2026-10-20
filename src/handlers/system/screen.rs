use crate::parse::Token;
use crate::registry;
use crate::verdict::Verdict;

const SESSION_LETTERS: &[char] = &['d', 'D', 'r', 'R', 'x'];
const LISTINGS: &[&str] = &["-ls", "-list", "-wipe"];

pub(crate) fn check_screen(tokens: &[Token]) -> Verdict {
    let Some((standalone, valued)) = registry::fallback_flag_sets("screen") else {
        return Verdict::Denied;
    };
    if starts_a_session(&tokens[1..], valued) {
        return Verdict::Denied;
    }
    let mut kept = Vec::with_capacity(tokens.len());
    let mut words = tokens.iter();
    kept.extend(words.next().cloned());
    while let Some(token) = words.next() {
        kept.push(token.clone());
        if names_a_session_next(token.as_str(), standalone) && words.as_slice().first().is_some_and(|next| !next.starts_with("-")) {
            words.next();
        }
    }
    registry::try_fallback_grammar("screen", &kept).unwrap_or(Verdict::Denied)
}

/// Unless it reattaches (`-r`, `-x`), lists, detaches with `-d` / `-D` alone, or prints its help or
/// version, screen starts a new session running the user's shell, and `-R` / `-RR` do too when
/// there is nothing to resume, even beside `-r` or `-x`: an attached shell that whatever drives the
/// terminal can type into.
fn starts_a_session(arguments: &[Token], valued: &[String]) -> bool {
    let takes_a_value = |c: char| valued.iter().any(|v| v.len() == 2 && v.ends_with(c));
    let mut letters = String::new();
    let mut words = arguments.iter().map(Token::as_str);
    while let Some(word) = words.next() {
        if LISTINGS.contains(&word) || matches!(word, "--help" | "--version") {
            return false;
        }
        let Some(cluster) = word.strip_prefix('-').filter(|c| !c.is_empty() && !c.starts_with('-')) else {
            continue;
        };
        for (at, c) in cluster.char_indices() {
            letters.push(c);
            if takes_a_value(c) {
                if at + c.len_utf8() == cluster.len() {
                    words.next();
                }
                break;
            }
        }
    }
    if letters.contains('R') {
        return true;
    }
    let resumes = letters.contains(['r', 'x', 'v']);
    let only_detaches = letters.contains(['d', 'D']);
    !(resumes || only_detaches)
}

fn names_a_session_next(word: &str, standalone: &[String]) -> bool {
    if LISTINGS.contains(&word) {
        return true;
    }
    let Some(letters) = word.strip_prefix('-').filter(|l| !l.is_empty() && !l.starts_with('-')) else {
        return false;
    };
    letters.contains(SESSION_LETTERS) && letters.chars().all(|c| standalone.iter().any(|s| s.len() == 2 && s.ends_with(c)))
}

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    use proptest::prelude::*;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        screen_reattach_named: "screen -r work",
        screen_attach_shared: "screen -x work",
        screen_detach_reattach: "screen -dr work",
        screen_detach_elsewhere: "screen -D work",
        screen_detach_named: "screen -d -S work",
        screen_help: "screen --help",
        screen_version: "screen -v",
        screen_list_matching: "screen -ls work",
        screen_list_long_matching: "screen -list work",
        screen_wipe_matching: "screen -wipe work",
        screen_scrollback: "screen -h 5000 -r work",
        screen_reattach_bare: "screen -r",
        screen_list: "screen -ls",
    }

    denied! {
        screen_two_operands: "screen -r work sh",
        screen_program_after_name: "screen -R work sh -c id",
        screen_name_then_detached: "screen -r work -m",
        screen_detached_cluster_with_name: "screen -dmr work",
        screen_program_alone: "screen sh",
        screen_program_after_listing_flag: "screen -q sh",
        screen_bare_starts_a_shell: "screen",
        screen_resume_or_create: "screen -R",
        screen_resume_or_create_twice: "screen -RR",
        screen_resume_or_create_named: "screen -R work",
        screen_detach_elsewhere_or_create: "screen -D -R work",
        screen_reattach_or_create: "screen -r -R work",
        screen_reattach_or_create_cluster: "screen -rR work",
        screen_share_or_create: "screen -x -R work",
        screen_detach_reattach_or_create: "screen -d -r -R work",
        screen_new_named_session: "screen -S work",
        screen_new_logged_session: "screen -L",
        screen_new_login_session: "screen -l",
        screen_new_quiet_session: "screen -q",
        screen_new_scrollback_session: "screen -h 5000",
        screen_new_session_glued_name: "screen -Swork",
    }

    proptest! {
        /// Whatever a session-naming option is spelled as, it takes one name and never a program,
        /// and a cluster with `m` never passes.
        #[test]
        fn an_option_takes_one_session_name_and_never_starts_a_detached_session(
            option in prop::sample::select(vec!["-r", "-x", "-dr", "-d -r", "-D -r", "-ls", "-list", "-wipe"]),
            name in "[a-z][a-z0-9.]{0,8}",
            program in prop::sample::select(vec!["sh", "bash -c id", "./x.sh", "/tmp/evil"]),
            m_at in 0usize..2,
        ) {
            prop_assert!(is_safe_command(&format!("screen {option} {name}")), "screen {} {}", option, name);
            prop_assert!(!is_safe_command(&format!("screen {option} {name} {program}")), "screen {} {} {}", option, name, program);
            let detached = if m_at == 0 { format!("screen -m {option} {name}") } else { format!("screen {option}m {name}") };
            prop_assert!(!is_safe_command(&detached), "{}", detached);
        }

        /// Options that only shape a new session never reattach one, so however they are combined
        /// the line starts an attached shell and is refused.
        #[test]
        fn options_that_only_shape_a_new_session_never_pass(
            options in prop::sample::subsequence(vec!["-R", "-RR", "-S work", "-a", "-A", "-U", "-L", "-l", "-q", "-O", "-h 500", "-T xterm", "-t title", "-p 0"], 0..4),
        ) {
            let line = format!("screen {}", options.join(" "));
            prop_assert!(!is_safe_command(&line), "{}", line);
        }

        /// `-R` creates a session when there is none to resume, and a reattach beside it does not
        /// change that, so an R anywhere among the letters is refused.
        #[test]
        fn an_r_that_may_create_a_session_never_passes_beside_a_reattach(
            reattach in prop::sample::select(vec!["-r", "-x", "-d -r", "-D -r", "-dr", "-r -d", ""]),
            create in prop::sample::select(vec!["-R", "-RR", "-rR", "-xR", "-Rx", "-dR", "-DR", "-Rr", "-rRR"]),
            first in proptest::bool::ANY,
        ) {
            let line = if first { format!("screen {create} {reattach} work") } else { format!("screen {reattach} {create} work") };
            prop_assert!(!is_safe_command(&line), "{}", line);
        }
    }
}

use crate::parse::Token;
use crate::registry;
use crate::verdict::Verdict;

const SESSION_LETTERS: &[char] = &['d', 'D', 'r', 'R', 'x'];
const LISTINGS: &[&str] = &["-ls", "-list", "-wipe"];

pub(crate) fn check_screen(tokens: &[Token]) -> Verdict {
    let Some((standalone, _)) = registry::fallback_flag_sets("screen") else {
        return Verdict::Denied;
    };
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
        screen_detach_elsewhere_reattach: "screen -D -R work",
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
    }

    proptest! {
        /// Whatever a session-naming option is spelled as, it takes one name and never a program,
        /// and a cluster with `m` never passes.
        #[test]
        fn an_option_takes_one_session_name_and_never_starts_a_detached_session(
            option in prop::sample::select(vec!["-r", "-R", "-x", "-dr", "-DR", "-d -r", "-ls", "-list", "-wipe"]),
            name in "[a-z][a-z0-9.]{0,8}",
            program in prop::sample::select(vec!["sh", "bash -c id", "./x.sh", "/tmp/evil"]),
            m_at in 0usize..2,
        ) {
            prop_assert!(is_safe_command(&format!("screen {option} {name}")), "screen {} {}", option, name);
            prop_assert!(!is_safe_command(&format!("screen {option} {name} {program}")), "screen {} {} {}", option, name, program);
            let detached = if m_at == 0 { format!("screen -m {option} {name}") } else { format!("screen {option}m {name}") };
            prop_assert!(!is_safe_command(&detached), "{}", detached);
        }
    }
}

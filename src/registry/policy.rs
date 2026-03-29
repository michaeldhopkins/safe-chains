use crate::parse::Token;
use crate::policy::FlagStyle;

use super::types::OwnedPolicy;

pub(super) fn check_owned(tokens: &[Token], policy: &OwnedPolicy) -> bool {
    if tokens.len() == 1 {
        return policy.bare;
    }

    let mut i = 1;
    let mut positionals: usize = 0;
    while i < tokens.len() {
        let t = &tokens[i];

        if *t == "--" {
            positionals += tokens.len() - i - 1;
            break;
        }

        if !t.starts_with('-') {
            positionals += 1;
            i += 1;
            continue;
        }

        if policy.standalone.iter().any(|f| t == f.as_str()) {
            i += 1;
            continue;
        }

        if policy.valued.iter().any(|f| t == f.as_str()) {
            i += 2;
            continue;
        }

        if let Some(flag) = t.as_str().split_once('=').map(|(f, _)| f) {
            if policy.valued.iter().any(|f| f.as_str() == flag) {
                i += 1;
                continue;
            }
            if policy.flag_style == FlagStyle::Positional {
                positionals += 1;
                i += 1;
                continue;
            }
            return false;
        }

        if t.starts_with("--") {
            if policy.flag_style == FlagStyle::Positional {
                positionals += 1;
                i += 1;
                continue;
            }
            return false;
        }

        let bytes = t.as_bytes();
        let mut j = 1;
        while j < bytes.len() {
            let b = bytes[j];
            let is_last = j == bytes.len() - 1;
            if policy.standalone.iter().any(|f| f.len() == 2 && f.as_bytes()[1] == b) {
                j += 1;
                continue;
            }
            if policy.valued.iter().any(|f| f.len() == 2 && f.as_bytes()[1] == b) {
                if is_last {
                    i += 1;
                }
                break;
            }
            if policy.flag_style == FlagStyle::Positional {
                positionals += 1;
                break;
            }
            return false;
        }
        i += 1;
    }
    policy.max_positional.is_none_or(|max| positionals <= max)
}

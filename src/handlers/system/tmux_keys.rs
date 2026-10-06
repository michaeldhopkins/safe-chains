//! `tmux send-keys`: keys typed into another pane, where a shell runs whatever line they submit.

use crate::parse::Token;
use crate::verdict::{SafetyLevel, Verdict};

/// Keys that move or cancel without putting anything on a shell's line. History (`Up`, `C-p`,
/// `C-r`) and yank (`C-y`) are left out: they fill the line with text nobody can read here.
const INERT_KEYS: &[&str] = &["Left", "Right", "Home", "End", "PageUp", "PageDown", "PgUp", "PgDn", "PPage", "NPage", "Escape", "C-c"];

const SUBMIT_KEYS: &[&str] = &["Enter", "KPEnter", "C-m", "C-j", "^M", "^J"];

/// Other names tmux reads as a key rather than as text; each is refused.
const OTHER_KEYS: &[&str] = &["Up", "Down", "IC", "Insert", "DC", "Delete", "BTab", "BSpace", "Tab", "Any"];
const OTHER_KEY_PREFIXES: &[&str] = &["KP", "Mouse", "Wheel", "Double", "Triple", "Paste", "SecondClick"];

enum Key<'a> {
    Inert,
    Submit,
    Space,
    Other,
    Text(&'a str),
}

fn classify(arg: &str) -> Key<'_> {
    let named = |list: &[&str]| list.iter().any(|k| k.eq_ignore_ascii_case(arg));
    if named(INERT_KEYS) {
        return Key::Inert;
    }
    if named(SUBMIT_KEYS) {
        return Key::Submit;
    }
    if arg.eq_ignore_ascii_case("Space") {
        return Key::Space;
    }
    let upper = arg.to_ascii_uppercase();
    let function_key = upper.strip_prefix('F').is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()));
    let modified = ["C-", "M-", "S-"].iter().any(|m| upper.starts_with(m) && upper.len() > 2) || (arg.starts_with('^') && arg.len() == 2);
    let coded = ["0X", "U+"]
        .iter()
        .any(|p| upper.strip_prefix(p).is_some_and(|h| !h.is_empty() && h.bytes().all(|b| b.is_ascii_hexdigit())));
    let prefixed = OTHER_KEY_PREFIXES.iter().any(|p| upper.starts_with(&p.to_ascii_uppercase()) && !arg.contains(' '));
    if named(OTHER_KEYS) || function_key || modified || coded || prefixed {
        return Key::Other;
    }
    Key::Text(arg)
}

/// `send-keys [-FHKlMRX] [-c client] [-N count] [-t pane] key ...`. Approved when it types nothing,
/// or when every piece of text it types is submitted and is itself an approved command. Text after a
/// moving key is refused: `Escape` is readline's Meta prefix, and `Home` puts it ahead of whatever
/// the line holds. `-K` is refused because key bindings apply to keys sent that way.
pub(super) fn send_keys_verdict(rest: &[Token]) -> Verdict {
    let mut i = 1;
    while i < rest.len() {
        let t = rest[i].as_str();
        let Some(cluster) = t.strip_prefix('-').filter(|c| !c.is_empty()) else { break };
        if cluster == "-" {
            i += 1;
            break;
        }
        for (at, c) in cluster.char_indices() {
            match c {
                'R' => {}
                'c' | 'N' | 't' => {
                    if at + 1 == cluster.len() {
                        if i + 1 >= rest.len() {
                            return Verdict::Denied;
                        }
                        i += 1;
                    }
                    break;
                }
                _ => return Verdict::Denied,
            }
        }
        i += 1;
    }
    let mut verdict = Verdict::Allowed(SafetyLevel::SafeWrite);
    let mut line = String::new();
    let mut moved = false;
    for arg in &rest[i..] {
        match classify(arg.as_str()) {
            Key::Inert if line.is_empty() => moved = true,
            Key::Submit if !line.is_empty() => {
                verdict = verdict.combine(crate::command_verdict(&line));
                line.clear();
            }
            Key::Space if !moved => line.push(' '),
            Key::Text(text) if !moved && !text.chars().any(char::is_control) => line.push_str(text),
            _ => return Verdict::Denied,
        }
    }
    if line.is_empty() { verdict } else { Verdict::Denied }
}

#[cfg(test)]
#[path = "tmux_keys_tests.rs"]
mod tests;

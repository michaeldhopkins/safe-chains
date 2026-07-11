//! The getopt-style flag walker used by the declaration-driven behavior resolver
//! (`resolve_behavior`) for fixed-flag-set commands (cat/rm/mkdir/touch/cp/mv/…). `grep` keeps
//! its own parser — its short options carry richer semantics (a value-short can supply a
//! pattern vs a count). Flag lists are lowered from TOML at build time and passed as slices.

use crate::parse::Token;

enum FlagKind {
    Unknown,
    Boolean,
    ValuedGlued, // the value is inside this token (`-tX`, `--dir=X`)
    ValuedNext,  // the value is the following token (`-t X`, `--dir X`)
}

/// The positional operands up to `--`, or `None` if any flag is unrecognized (the caller
/// then worst-cases, §0). A valued flag's value is consumed, never returned as a positional;
/// a bare `-` is a positional (stdin). Slice-taking so the runtime behavior resolver (owned
/// flag lists lowered from TOML) and the const resolvers share ONE walk — a security
/// classifier must not have two flag walkers that can drift.
pub(super) fn walk_positionals<'a>(
    short: &[u8],
    valued_short: &[u8],
    long: &[&str],
    valued_long: &[&str],
    numeric_shorthand: bool,
    tokens: &'a [Token],
) -> Option<Vec<&'a str>> {
    let mut out = Vec::new();
    let mut flags_done = false;
    let mut i = 1;
    while i < tokens.len() {
        let t = tokens[i].as_str();
        if !flags_done && t == "--" {
            flags_done = true;
        } else if flags_done || t == "-" || !t.starts_with('-') {
            out.push(t);
        } else {
            match classify_flag(short, valued_short, long, valued_long, numeric_shorthand, t) {
                FlagKind::Unknown => return None,
                FlagKind::ValuedNext => i += 1, // also skip the value token
                FlagKind::Boolean | FlagKind::ValuedGlued => {}
            }
        }
        i += 1;
    }
    Some(out)
}

/// Classify one flag token against a flag spec (slice form — see `walk_positionals`).
fn classify_flag(
    short: &[u8],
    valued_short: &[u8],
    long: &[&str],
    valued_long: &[&str],
    numeric_shorthand: bool,
    t: &str,
) -> FlagKind {
    if let Some(rest) = t.strip_prefix("--") {
        let name_len = rest.split('=').next().unwrap_or(rest).len();
        let full = &t[..2 + name_len];
        let has_eq = t.len() > 2 + name_len;
        if valued_long.contains(&full) {
            return if has_eq { FlagKind::ValuedGlued } else { FlagKind::ValuedNext };
        }
        if long.contains(&full) {
            // A boolean long never consumes the NEXT token; a glued `=value` is its
            // optional-argument form (`rm --interactive=always`) — accept and consume
            // just this token.
            return if has_eq { FlagKind::ValuedGlued } else { FlagKind::Boolean };
        }
        return FlagKind::Unknown;
    }
    let bytes = t.as_bytes();
    // Obsolete `-NUM` count shorthand (head/tail): the digits are the inline value.
    if numeric_shorthand && bytes.len() > 1 && bytes[1..].iter().all(|b| b.is_ascii_digit()) {
        return FlagKind::Boolean;
    }
    let mut k = 1;
    while k < bytes.len() {
        let b = bytes[k];
        if valued_short.contains(&b) {
            return if k + 1 < bytes.len() { FlagKind::ValuedGlued } else { FlagKind::ValuedNext };
        }
        if short.contains(&b) {
            k += 1;
        } else {
            return FlagKind::Unknown;
        }
    }
    FlagKind::Boolean
}

/// The value of the `short`/`long` valued flag (glued or next-token), if present. `valued_short`
/// is the set of value-taking short flags.
pub(super) fn walk_value<'a>(valued_short: &[u8], tokens: &'a [Token], short: u8, long: &str) -> Option<&'a str> {
    let mut i = 1;
    while i < tokens.len() {
        let t = tokens[i].as_str();
        if t == "--" {
            break;
        }
        if t.starts_with("--") {
            if let Some(v) = t.strip_prefix(long).and_then(|r| r.strip_prefix('=')) {
                return Some(v);
            }
            if t == long {
                return tokens.get(i + 1).map(Token::as_str);
            }
        } else if t.starts_with('-') && t != "-" {
            let bytes = t.as_bytes();
            let mut k = 1;
            while k < bytes.len() {
                let b = bytes[k];
                if b == short && valued_short.contains(&b) {
                    let glued = &t[k + 1..];
                    return if glued.is_empty() { tokens.get(i + 1).map(Token::as_str) } else { Some(glued) };
                }
                if valued_short.contains(&b) {
                    break; // a different valued short consumes the rest of the cluster
                }
                k += 1;
            }
        }
        i += 1;
    }
    None
}

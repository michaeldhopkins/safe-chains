//! The getopt-style flag walker shared by the fixed-flag-set resolvers
//! (cat/rm/mkdir/touch/cp/mv). `grep` keeps its own parser — its short options
//! carry richer semantics (a value-short can supply a pattern vs a count).

use crate::parse::Token;

/// A flag spec for a fixed-flag-set command. `short` flags are single chars that cluster
/// (`-rf`); `valued_*` flags take a value, glued (`-tDIR`, `--dir=X`) or as the next token
/// (`-t DIR`, `--dir X`). A valued short ends its cluster and takes the glued remainder or
/// the next token. `numeric_shorthand` recognizes the obsolete `-NUM` count form
/// (`head -20`, `tail -5`) — the digits are the inline value, so it takes no operand.
pub(super) struct Flags {
    pub(super) short: &'static [u8],
    pub(super) valued_short: &'static [u8],
    pub(super) long: &'static [&'static str],
    pub(super) valued_long: &'static [&'static str],
    pub(super) numeric_shorthand: bool,
}

enum FlagKind {
    Unknown,
    Boolean,
    ValuedGlued, // the value is inside this token (`-tX`, `--dir=X`)
    ValuedNext,  // the value is the following token (`-t X`, `--dir X`)
}

impl Flags {
    /// The positional operands up to `--`, or `None` if any flag is unrecognized (the
    /// caller then worst-cases, §0). A valued flag's value is consumed, never returned as a
    /// positional; a bare `-` is a positional (stdin).
    pub(super) fn positionals<'a>(&self, tokens: &'a [Token]) -> Option<Vec<&'a str>> {
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
                match self.classify(t) {
                    FlagKind::Unknown => return None,
                    FlagKind::ValuedNext => i += 1, // also skip the value token
                    FlagKind::Boolean | FlagKind::ValuedGlued => {}
                }
            }
            i += 1;
        }
        Some(out)
    }

    /// Classify one flag token against the spec.
    fn classify(&self, t: &str) -> FlagKind {
        if let Some(rest) = t.strip_prefix("--") {
            let name_len = rest.split('=').next().unwrap_or(rest).len();
            let full = &t[..2 + name_len];
            let has_eq = t.len() > 2 + name_len;
            if self.valued_long.contains(&full) {
                return if has_eq { FlagKind::ValuedGlued } else { FlagKind::ValuedNext };
            }
            if self.long.contains(&full) {
                // A boolean long never consumes the NEXT token; a glued `=value` is its
                // optional-argument form (`rm --interactive=always`) — accept and consume
                // just this token.
                return if has_eq { FlagKind::ValuedGlued } else { FlagKind::Boolean };
            }
            return FlagKind::Unknown;
        }
        let bytes = t.as_bytes();
        // Obsolete `-NUM` count shorthand (head/tail): the digits are the inline value.
        if self.numeric_shorthand && bytes.len() > 1 && bytes[1..].iter().all(|b| b.is_ascii_digit()) {
            return FlagKind::Boolean;
        }
        let mut k = 1;
        while k < bytes.len() {
            let b = bytes[k];
            if self.valued_short.contains(&b) {
                return if k + 1 < bytes.len() { FlagKind::ValuedGlued } else { FlagKind::ValuedNext };
            }
            if self.short.contains(&b) {
                k += 1;
            } else {
                return FlagKind::Unknown;
            }
        }
        FlagKind::Boolean
    }

    /// The value of a specific valued flag (glued or next-token), if present.
    pub(super) fn value<'a>(&self, tokens: &'a [Token], short: u8, long: &str) -> Option<&'a str> {
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
                    if b == short && self.valued_short.contains(&b) {
                        let glued = &t[k + 1..];
                        return if glued.is_empty() { tokens.get(i + 1).map(Token::as_str) } else { Some(glued) };
                    }
                    if self.valued_short.contains(&b) {
                        break; // a different valued short consumes the rest of the cluster
                    }
                    k += 1;
                }
            }
            i += 1;
        }
        None
    }
}

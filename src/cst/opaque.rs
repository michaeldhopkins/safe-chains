//! Words whose value the classifier cannot know.
//!
//! A command's grammar is judged on the words it is shown. A word bash fills in at run time —
//! `$X`, `$(…)`, a glob, a loop variable, an item piped to `xargs` — reaches the grammar as a
//! stand-in that reads as an ordinary operand, while the command may receive `-delete`. So every
//! such word is assumed to be ANY string, and the command is approved only if it would be
//! approved whatever the word turned out to be.
//!
//! Rather than reason per command about where a flag may go, the command is classified again
//! with the unknown words replaced by probes, and the worst verdict wins:
//!
//! - an unknown flag ([`FLAG_PROBE`]) where the word could begin with `-`. Every grammar refuses
//!   a flag it does not list, so this is refused exactly where a flag would be read: an operand
//!   or flag slot, not the value of a valued flag or the words after `--`;
//! - the word followed by that flag, where it could split into several words;
//! - nothing, where it could expand to no word at all;
//! - the value itself, where every unknown part is a variable whose value IS known.
//!
//! Probing replaces every probed word at once, so a command costs at most six more
//! classifications however many unknown words it holds.

mod declared;
mod items;
mod walk;

pub(crate) use declared::declared_names;
pub(crate) use items::{enter_loop, enter_read_var, enter_stdin, prints_only_paths, stage_shape};
use walk::shape;

use super::{SimpleCmd, WordPart};
use crate::parse::Token;
use crate::pathctx::Facts;
use crate::verdict::Verdict;

pub(crate) const FLAG_PROBE: &str = "--safe-chains-opaque";
/// A short flag no command declares. Some grammars read an unknown long word as an operand (grep
/// takes it for the pattern) while refusing an unknown short one, so both spellings are tried.
pub(crate) const SHORT_PROBE: &str = "-%";
/// Every digit, so a grammar that takes this as a cluster of number flags takes any one of them.
const NUMBER_PROBE: &str = "-1234567890";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Lead {
    #[default]
    No,
    /// Only a number can lead: arithmetic, or a numeric brace sequence (`{-1..-3}`).
    Number,
    Any,
}

/// Marks the facts about one word's parts.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Parts {}

/// It may become several words.
pub(crate) const SPLIT: Facts<Parts> = Facts::bit(0);
/// It may become no word at all.
pub(crate) const VANISH: Facts<Parts> = Facts::bit(1);
/// Some part of it is unknown.
pub(crate) const OPAQUE: Facts<Parts> = Facts::bit(2);
/// It holds a glob.
pub(crate) const GLOB: Facts<Parts> = Facts::bit(3);
/// Some unknown part may hold a blank, so the value splits wherever it is used unquoted.
pub(crate) const BLANK: Facts<Parts> = Facts::bit(4);

#[derive(Debug, Default)]
pub(crate) struct Shape {
    pub(crate) lead: Lead,
    pub(crate) facts: Facts<Parts>,
    /// The words bash makes of the word once every certain variable is substituted, when any was.
    pub(crate) concrete: Option<Vec<String>>,
}

#[derive(Clone, Copy)]
enum Probe {
    Lead(&'static str),
    Split(&'static str),
    Vanish,
    Concrete,
}

impl Shape {
    fn wants(&self, probe: Probe) -> bool {
        match probe {
            Probe::Lead(_) => self.lead != Lead::No,
            Probe::Split(_) => self.facts.has(SPLIT),
            Probe::Vanish => self.facts.has(VANISH),
            Probe::Concrete => self.concrete.is_some(),
        }
    }

    fn tokens(&self, probe: Probe, default: &[Token]) -> Vec<Token> {
        let fallback = || match &self.concrete {
            Some(words) => words.iter().cloned().map(Token::from_raw).collect(),
            None => default.to_vec(),
        };
        match probe {
            Probe::Lead(flag) => match self.lead {
                Lead::Any => vec![Token::from_raw(flag.to_string())],
                Lead::Number => vec![Token::from_raw(NUMBER_PROBE.to_string())],
                Lead::No => fallback(),
            },
            Probe::Split(flag) if self.wants(probe) => default.iter().cloned().chain([Token::from_raw(flag.to_string())]).collect(),
            Probe::Vanish if self.wants(probe) => Vec::new(),
            _ => fallback(),
        }
    }
}

/// The verdict of `classify` over the command's words, worst-cased over what its unknown words
/// could be. `words[i]` holds the tokens `cmd.words[i]` expanded to; the command name is never
/// probed (an unknown name is refused as unknown).
pub(crate) fn probed_verdict(cmd: &SimpleCmd, words: &[Vec<Token>], classify: impl Fn(&[Token]) -> Verdict) -> Verdict {
    let mut verdict = classify(&words.concat());
    if !verdict.is_allowed() {
        return verdict;
    }
    let shapes: Vec<Option<Shape>> = cmd
        .words
        .iter()
        .zip(words)
        .enumerate()
        .map(|(i, (w, toks))| (i > 0).then(|| shape(w, toks.len())))
        .collect();
    let probes = [FLAG_PROBE, SHORT_PROBE]
        .map(Probe::Lead)
        .into_iter()
        .chain([FLAG_PROBE, SHORT_PROBE].map(Probe::Split));
    for probe in probes.chain([Probe::Vanish, Probe::Concrete]) {
        if !shapes.iter().flatten().any(|s| s.wants(probe)) {
            continue;
        }
        let tokens: Vec<Token> = words
            .iter()
            .zip(&shapes)
            .flat_map(|(toks, s)| s.as_ref().map_or_else(|| toks.clone(), |s| s.tokens(probe, toks)))
            .collect();
        // Each probe re-runs the whole classification, so it draws on the same budget as brace
        // fan-out and delegation; spent, the command fails closed rather than stalling the hook.
        if !super::check::charge_classify_work(u32::try_from(tokens.len()).unwrap_or(u32::MAX)) {
            return Verdict::Denied;
        }
        verdict = verdict.combine(classify(&tokens));
        if !verdict.is_allowed() {
            return verdict;
        }
    }
    verdict
}

/// Whether `tokens[idx]` is the probe standing as the value of the valued flag before it, by the
/// grammar's own `valued` list. A grammar refuses a separate value that looks like a flag (`-m
/// --amend` is read as two flags), which is right for text it can see; an unknown word in that slot
/// is the flag's value whatever it holds, as getopt reads it.
pub(crate) fn probe_is_value(tokens: &[Token], idx: usize, valued_flags: &[String]) -> bool {
    let valued = |f: &str| valued_flags.iter().any(|a| a == f);
    let (Some(t), Some(prev)) = (tokens.get(idx), idx.checked_sub(1).and_then(|p| tokens.get(p))) else {
        return false;
    };
    let prev = prev.as_str();
    if !is_flag_probe(t.as_str()) || !prev.starts_with('-') || prev.contains('=') {
        return false;
    }
    valued(prev) || (!prev.starts_with("--") && prev.len() > 2 && prev.chars().last().is_some_and(|c| valued(&format!("-{c}"))))
}

pub(crate) fn is_flag_probe(s: &str) -> bool {
    s == FLAG_PROBE || s == SHORT_PROBE
}

/// Whether an operand hides a FLAG behind an unquoted expansion.
///
/// The word-splitting problem again, on the dimension the locus gate cannot see. One CST word
/// becomes several arguments at run time, and when a piece starts with `-` the command's flag
/// allowlist was simply never shown it:
///
/// ```text
/// VAR="--exec rm"; fd pat $VAR        ran `rm` on every match
/// VAR="-exec rm {} ;"; find . $VAR    deleted the tree
/// ```
///
/// Splitting for LOCUS (see `locus::classify_local`) does not help here, because the danger is not
/// where a path points — it is a capability the grammar would have refused outright.
///
/// This refuses rather than re-tokenizing. Re-tokenizing would be more precise, and the machinery
/// is close at hand (`Word::expand` already turns one word into many for brace expansion) — but a
/// bound value carries SEPARATE read and write representatives for loop variables, so feeding it
/// back into tokenization would have to pick a face before the face is known. Refusing costs a
/// prompt on `VAR="-rf ./sub"; rm $VAR`, which is a rare way to write a command; see TODO.md.
///
/// Only UNQUOTED expansions split, so `cat "$VAR"` with a spacey filename is untouched — a quoted
/// expansion is one word to the shell too.
pub(super) fn smuggles_a_flag(cmd: &SimpleCmd) -> bool {
    cmd.words.iter().skip(1).any(|w| {
        // A top-level `Lit` is the unquoted case; a `DQuote` part is not split by the shell.
        w.0.iter().any(|part| {
            let WordPart::Lit(raw) = part else { return false };
            if !raw.contains('$') {
                return false;
            }
            let expanded = crate::pathctx::expand_vars(raw, false);
            expanded.split([' ', '\t', '\n']).skip(1).any(|piece| piece.starts_with('-'))
                || (expanded.split([' ', '\t', '\n']).count() > 1 && expanded.starts_with('-'))
        })
    })
}

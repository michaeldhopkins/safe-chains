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
//! - the value itself, where every unknown part is a variable whose value IS known;
//! - the word's text with each unknown part read as [`UNKNOWN_CONTENT`](crate::parse::UNKNOWN_CONTENT),
//!   which a handler refuses where it reads code and judges as a relative name where it reads a
//!   path: `awk "$p"` runs whatever `$p` holds.
//!
//! Probing replaces every probed word at once, so a command costs at most seven more
//! classifications however many unknown words it holds.

mod declared;
pub(crate) mod glob;
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
    /// The word with each unknown part replaced by what stands for its content, when any is unknown.
    pub(crate) content: Option<String>,
}

#[derive(Clone, Copy)]
enum Probe {
    Lead(&'static str),
    Split(&'static str),
    Vanish,
    Concrete,
    Content,
}

impl Shape {
    fn wants(&self, probe: Probe) -> bool {
        match probe {
            Probe::Lead(_) => self.lead != Lead::No,
            Probe::Split(_) => self.facts.has(SPLIT),
            Probe::Vanish => self.facts.has(VANISH),
            Probe::Concrete => self.concrete.is_some(),
            Probe::Content => self.content.is_some(),
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
            Probe::Content => match (&self.content, default) {
                (Some(text), [_]) => vec![Token::from_raw(text.clone())],
                (Some(_), _) => default.iter().map(|_| Token::from_raw(crate::parse::UNKNOWN_CONTENT.to_string())).collect(),
                (None, _) => fallback(),
            },
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
        .map(|(i, (w, toks))| (i > 0 && !declares(cmd, w)).then(|| shape(w, toks.len())))
        .collect();
    let probes = [FLAG_PROBE, SHORT_PROBE]
        .map(Probe::Lead)
        .into_iter()
        .chain([FLAG_PROBE, SHORT_PROBE].map(Probe::Split));
    for probe in probes.chain([Probe::Vanish, Probe::Concrete, Probe::Content]) {
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
///
///
/// In a short cluster the first valued letter takes the rest of the cluster as its value, so it
/// takes the next word only when it is the cluster's last letter: `-am "$MSG"` gives `$MSG` to
/// `-m`, while `-rmaster "$X"` and `-Tr "$X"` leave the word free.
pub(crate) fn probe_is_value(tokens: &[Token], idx: usize, valued_flags: &[String]) -> bool {
    probe_follows_valued(tokens, idx, valued_flags, true)
}

/// As [`probe_is_value`], with clusters read only when `clusters` holds. Reading a cluster assumes
/// the letters before its valued one are switches, which holds for a grammar that refuses unknown
/// letters and not for a list shared across subcommands that disagree about a letter.
fn probe_follows_valued(tokens: &[Token], idx: usize, valued_flags: &[String], clusters: bool) -> bool {
    let valued = |f: &str| valued_flags.iter().any(|a| a == f);
    let (Some(t), Some(prev)) = (tokens.get(idx), idx.checked_sub(1).and_then(|p| tokens.get(p))) else {
        return false;
    };
    let prev = prev.as_str();
    if !is_flag_probe(t.as_str()) || !prev.starts_with('-') || prev.contains('=') {
        return false;
    }
    if valued(prev) {
        return true;
    }
    if !clusters || prev.starts_with("--") || prev.len() <= 2 {
        return false;
    }
    let letters: Vec<char> = prev[1..].chars().collect();
    letters.iter().position(|c| valued(&format!("-{c}"))).is_some_and(|at| at + 1 == letters.len())
}

pub(crate) fn is_flag_probe(s: &str) -> bool {
    s == FLAG_PROBE || s == SHORT_PROBE
}

/// Whether a probe stands where the unknown word could be any flag: before `--`, and not the value
/// of a flag in `valued_flags`. A grammar that tolerates unknown flags accepts the probe, so a
/// command's classifying flags count it as present themselves. A probe after a flag that is not
/// known to take a value is a flag of its own: `--no-graph "$X"` passes `$X` as an option.
pub(crate) fn may_be_any_flag(tokens: &[Token], valued_flags: &[String]) -> bool {
    for (idx, t) in tokens.iter().enumerate().skip(1) {
        let s = t.as_str();
        if s == "--" {
            return false;
        }
        if is_flag_probe(s) && !probe_follows_valued(tokens, idx, valued_flags, false) {
            return true;
        }
    }
    false
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

/// `export NAME=$X`: bash reads an assignment given to a declaration builtin as an assignment, so
/// it is neither split nor globbed, and it is no flag whatever `$X` holds.
fn declares(cmd: &SimpleCmd, word: &super::Word) -> bool {
    let builtin = cmd.words.first().map(super::Word::eval);
    let Some(WordPart::Lit(head)) = word.0.first() else { return false };
    let name = head.split('=').next().unwrap_or_default();
    matches!(builtin.as_deref(), Some("export" | "declare" | "typeset" | "local" | "readonly"))
        && head.contains('=')
        && name.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// The words `word` certainly becomes, when no part of it is unknown: its literal text with every
/// variable of certain value substituted. `None` for anything unknown, a glob, a brace fan-out, or
/// a value holding a glob or `~`, which bash would expand again.
pub(crate) fn known_words(word: &super::Word) -> Option<Vec<String>> {
    let expanded = word.expand();
    if expanded.len() != 1 {
        return None;
    }
    let s = shape(word, 1);
    if s.facts.has(OPAQUE | GLOB) || s.lead != Lead::No {
        return None;
    }
    let words = s.concrete.unwrap_or(expanded);
    (!words.iter().any(|w| w.contains(['*', '?', '[', '~']))).then_some(words)
}

use super::{Word, WordPart};

pub fn eval_word(word: &Word) -> String {
    let mut out = String::new();
    for part in &word.0 {
        eval_part(part, &mut out);
    }
    out
}

/// Past this many words from one expansion we stop enumerating and fail CLOSED — a hostile
/// `{a,b}{a,b}…` would otherwise blow up 2^n. The word becomes an unpinnable sentinel (→ machine
/// → deny) rather than a DoS.
pub(super) const BRACE_EXPANSION_CAP: usize = 256;
pub(super) const UNPINNABLE: &str = "__SAFE_CHAINS_CMDSUB__";
/// A brace sequence that was not expanded: it is longer than [`BRACE_EXPANSION_CAP`], or its size
/// cannot be bounded. A destructive or recursive write of a word carrying this marker is refused.
pub(crate) const UNRESOLVED_BRACE: &str = "__SAFE_CHAINS_UNRESOLVED_BRACE__";
/// Prefix of the BOUNDED substitution sentinel. Deliberately not a prefix-match of [`UNPINNABLE`]
/// (`…CMDSUB_WORKTREE__` does not contain `…CMDSUB__`), so the fail-closed guard keyed on the
/// latter can never fire on a declared substitution, nor vice versa.
pub(crate) const TAGGED_PREFIX: &str = "__SAFE_CHAINS_CMDSUB_";

/// The sentinel for a substitution whose every output word is separator-free (`seq 1 4`). It
/// asserts a SHAPE, not a rung, so it is deliberately not spelled like a locus tag: nothing can
/// mistake it for one and admit it by parsing.
pub(crate) const ATOM_SENTINEL: &str = "__SAFE_CHAINS_CMDSUB_ATOM__";

/// Process substitution (`<(cmd)`). Deliberately NOT the unpinnable sentinel: as a data operand a
/// `/dev/fd` pipe is exactly as safe as the inner command, which is checked separately, so
/// `diff <(ls) <(ls)` should classify as an ordinary worktree operand.
///
/// As an EXECUTOR it is the opposite, which is why `execute_file_verdict` refuses it by name:
/// running the pipe runs the inner command's OUTPUT, and a command that is safe to RUN is not the
/// same as one whose output is safe to EXECUTE.
pub(crate) const PROCSUB_SENTINEL: &str = "__SAFE_CHAINS_PROCSUB__";

/// Expand a word to the literal words bash would produce, chiefly via UNQUOTED brace expansion
/// (`{/etc/shadow,x}` → two words). Quoted / escaped / substituted parts are fixed (bash does not
/// brace-expand inside quotes), so only `Lit` parts expand. The classifier checks EVERY produced
/// word, so a hidden alternative can't smuggle a system read/write past the `/`-prefix path gate.
pub fn expand_word(word: &Word) -> Vec<String> {
    let mut acc = vec![String::new()];
    for part in &word.0 {
        let alts: Vec<String> = match part {
            WordPart::Lit(s) => super::brace::expand(s),
            other => {
                let mut t = String::new();
                eval_part(other, &mut t);
                vec![t]
            }
        };
        if alts.len() > BRACE_EXPANSION_CAP || acc.len().saturating_mul(alts.len()) > BRACE_EXPANSION_CAP {
            return vec![UNPINNABLE.to_string()];
        }
        if let [b] = alts.as_slice() {
            // No brace fan-out (the common case — every substitution/quoted part yields one
            // alternative): append in place. Rebuilding via `format!("{a}{b}")` per part re-copies
            // the whole growing accumulator each time, which is O(n²) on a word of many parts
            // (50 000 backticks → a 1.2 MB string built in ~0.6 s). push_str keeps it linear.
            for a in &mut acc {
                a.push_str(b);
            }
        } else {
            let mut next = Vec::with_capacity(acc.len() * alts.len());
            for a in &acc {
                for b in &alts {
                    next.push(format!("{a}{b}"));
                }
            }
            acc = next;
        }
    }
    // Charge only the EXTRA words this expansion created (`acc.len() - 1`), against the same budget
    // delegation and function resolution use. An ordinary word yields one alternative and charges
    // nothing, so normal commands are untouched; a fanned-out word charges its fan-out, so expansion
    // and delegation ADD instead of multiplying. Over budget → the unpinnable sentinel (deny), the
    // same fail-closed result BRACE_EXPANSION_CAP already produces.
    let extra = u32::try_from(acc.len().saturating_sub(1)).unwrap_or(u32::MAX);
    if extra > 0 && !crate::cst::check::charge_classify_work(extra) {
        return vec![UNPINNABLE.to_string()];
    }
    acc
}

/// The placeholder a substitution's value evaluates to: the bounded, locus-tagged form when the
/// inner command declared what its stdout can name, else the opaque worst-cased one.
fn sub_sentinel(inner: &super::Script) -> String {
    use crate::engine::resolve::SubClaim;
    match crate::engine::resolve::substitution_claim(inner) {
        Some(SubClaim::Locus(locus)) => {
            use crate::engine::facet::FacetTerm;
            format!("{TAGGED_PREFIX}{}__", locus.as_str().to_uppercase().replace('-', "_"))
        }
        // Shares the tagged shape so every "is this a substitution value" test keeps seeing it,
        // but `ATOM` is not a locus term — `tagged_substitution` intercepts it before the term
        // parse, and anything that does not intercept it fails closed via `is_unpinnable`.
        Some(SubClaim::Atom) => ATOM_SENTINEL.to_string(),
        None => UNPINNABLE.to_string(),
    }
}

pub(super) fn eval_part(part: &WordPart, out: &mut String) {
    match part {
        WordPart::Lit(s) => out.push_str(s),
        WordPart::Escape(c) => out.push(*c),
        WordPart::SQuote(s) => out.push_str(s),
        WordPart::AnsiC(raw) => out.push_str(&super::ansi_c::decode(raw)),
        WordPart::DQuote(inner) => {
            for p in &inner.0 {
                eval_part(p, out);
            }
        }
        // Command substitution / backtick: the output becomes the operand VALUE. That value is
        // unknowable in general, so the default is a sentinel the classifier worst-cases (see
        // locus::is_unpinnable). When the inner command has DECLARED what its stdout can name
        // (`[command.output]`), the value is bounded by that locus instead and gets a tagged
        // sentinel — see docs/design/behavioral-taxonomy-substitution-locus.md.
        WordPart::CmdSub(inner) => out.push_str(&sub_sentinel(inner)),
        WordPart::Backtick(raw) => match super::parse(&super::escape::backtick_command(raw)) {
            Some(inner) => out.push_str(&sub_sentinel(&inner)),
            None => out.push_str(UNPINNABLE),
        },
        // Process substitution: the operand is a /dev/fd pipe; its safety is the INNER command
        // (checked separately by word_sub_verdict), not an unknowable path — so a distinct
        // placeholder that classifies as an ordinary (worktree) operand.
        WordPart::ProcSub(_) => out.push_str(PROCSUB_SENTINEL),
        WordPart::Arith(_) => out.push_str("__SAFE_CHAINS_ARITH__"),
    }
}

/// The words a redirect opens, judged after brace expansion. `> .gi{t..t}/config` is `> .git/config`.
/// An unresolved sequence refuses the write: the shell would open a list we did not enumerate.
pub(super) fn judge_write(word: &Word, safe: impl Fn(&str) -> bool) -> crate::verdict::Verdict {
    use crate::verdict::{SafetyLevel, Verdict};
    let Ok(words) = opened(word) else {
        return Verdict::Denied;
    };
    let mut level = Verdict::Allowed(SafetyLevel::Inert);
    for target in &words {
        if target == "/dev/null" {
            continue;
        }
        if !safe(target) {
            return Verdict::Denied;
        }
        level = Verdict::Allowed(SafetyLevel::SafeWrite);
    }
    level
}

/// The words a redirect reads, judged after brace expansion. An unresolved sequence names a file
/// we cannot point at, so the read is refused.
pub(super) fn judge_read(word: &Word, opaque: impl Fn(&str) -> bool) -> crate::verdict::Verdict {
    use crate::verdict::{SafetyLevel, Verdict};
    let Ok(words) = opened(word) else {
        return Verdict::Denied;
    };
    let mut level = Verdict::Allowed(SafetyLevel::Inert);
    for target in &words {
        if opaque(target) {
            return Verdict::Denied;
        }
        level = level.combine(crate::engine::resolve::read_content_verdict(target));
    }
    level
}

fn opened(word: &Word) -> Result<Vec<String>, ()> {
    let words = expand_word(word);
    if words.iter().any(|word| word.contains(UNRESOLVED_BRACE)) { Err(()) } else { Ok(words) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn literal() {
        let w = Word(vec![WordPart::Lit("hello".into())]);
        assert_eq!(w.eval(), "hello");
    }

    #[test]
    fn single_quoted() {
        let w = Word(vec![WordPart::SQuote("hello world".into())]);
        assert_eq!(w.eval(), "hello world");
    }

    #[test]
    fn double_quoted() {
        let w = Word(vec![WordPart::DQuote(Word(vec![WordPart::Lit("hello ".into()), WordPart::Lit("world".into())]))]);
        assert_eq!(w.eval(), "hello world");
    }

    #[test]
    fn mixed_parts() {
        let w = Word(vec![
            WordPart::Lit("foo".into()),
            WordPart::DQuote(Word(vec![WordPart::Lit("bar".into())])),
            WordPart::SQuote("baz".into()),
        ]);
        assert_eq!(w.eval(), "foobarbaz");
    }

    #[test]
    fn cmd_sub_placeholder() {
        let w = Word(vec![WordPart::CmdSub(super::super::Script(vec![]))]);
        assert_eq!(w.eval(), "__SAFE_CHAINS_CMDSUB__");
    }

    #[test]
    fn dquote_with_escape() {
        let w =
            Word(vec![WordPart::DQuote(Word(vec![WordPart::Lit("hello".into()), WordPart::Escape('"'), WordPart::Lit("world".into())]))]);
        assert_eq!(w.eval(), "hello\"world");
    }
}

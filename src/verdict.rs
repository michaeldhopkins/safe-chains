use std::fmt;

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum SafetyLevel {
    Inert = 0,
    SafeRead = 1,
    SafeWrite = 2,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Verdict {
    Denied,
    Allowed(SafetyLevel),
}

impl Verdict {
    pub fn combine(self, other: Verdict) -> Verdict {
        match (self, other) {
            (Verdict::Denied, _) | (_, Verdict::Denied) => Verdict::Denied,
            (Verdict::Allowed(a), Verdict::Allowed(b)) => Verdict::Allowed(a.max(b)),
        }
    }

    pub fn is_allowed(self) -> bool {
        matches!(self, Verdict::Allowed(_))
    }
}

impl SafetyLevel {
    /// Resolve a `--level` threshold name to its ceiling. Accepts the current level names
    /// (`paranoid`/`reader`/`editor`/`developer`/`local-admin`/`network-admin`/`yolo`) and the
    /// three legacy names (`inert`/`safe-read`/`safe-write`). Returns the ceiling plus, for a
    /// legacy name, the current name it maps to (so the CLI can print a migration notice).
    /// `None` for an unknown name.
    ///
    /// The engine projects to this coarse 3-value ceiling today, so `editor`/`developer` and the
    /// `local-admin`/`network-admin`/`yolo` levels all share the `SafeWrite` ceiling — they
    /// become functionally distinct thresholds once the engine exposes per-level classification
    /// (the `level.admits(profile)` check) rather than the 3-value projection.
    pub fn resolve_threshold(name: &str) -> Option<(SafetyLevel, Option<&'static str>)> {
        Some(match name {
            "paranoid" => (SafetyLevel::Inert, None),
            "reader" => (SafetyLevel::SafeRead, None),
            "editor" | "developer" | "local-admin" | "network-admin" | "yolo" => (SafetyLevel::SafeWrite, None),
            // Legacy names — kept working so existing setups/muscle-memory don't break; the CLI
            // prints a one-line notice pointing at the current name.
            "inert" => (SafetyLevel::Inert, Some("paranoid")),
            "safe-read" => (SafetyLevel::SafeRead, Some("reader")),
            "safe-write" => (SafetyLevel::SafeWrite, Some("developer")),
            _ => return None,
        })
    }
}

impl fmt::Display for SafetyLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SafetyLevel::Inert => write!(f, "inert"),
            SafetyLevel::SafeRead => write!(f, "safe-read"),
            SafetyLevel::SafeWrite => write!(f, "safe-write"),
        }
    }
}

impl fmt::Display for Verdict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Verdict::Denied => write!(f, "denied"),
            Verdict::Allowed(level) => write!(f, "allowed ({level})"),
        }
    }
}

impl clap::ValueEnum for SafetyLevel {
    fn value_variants<'a>() -> &'a [Self] {
        &[SafetyLevel::Inert, SafetyLevel::SafeRead, SafetyLevel::SafeWrite]
    }

    fn to_possible_value(&self) -> Option<clap::builder::PossibleValue> {
        match self {
            SafetyLevel::Inert => Some(clap::builder::PossibleValue::new("inert")),
            SafetyLevel::SafeRead => Some(clap::builder::PossibleValue::new("safe-read")),
            SafetyLevel::SafeWrite => Some(clap::builder::PossibleValue::new("safe-write")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    /// A `SafetyLevel` strategy: enumerate the three variants so `proptest::sample::select` covers
    /// the whole lattice uniformly and shrinks toward `Inert`. Needed because `Proptest` does not
    /// derive `Arbitrary` for non-`unit`-carrier enums, and the four-arity `Verdict` strategy is
    /// built on top of this.
    fn arb_safety_level() -> proptest::strategy::BoxedStrategy<SafetyLevel> {
        proptest::sample::select(vec![SafetyLevel::Inert, SafetyLevel::SafeRead, SafetyLevel::SafeWrite]).boxed()
    }

    /// A `Verdict` strategy: `Denied` plus every `Allowed(level)`. Uniform coverage matters
    /// because the property tests partition on whether at least one operand is denied — picking
    /// only `Allowed` values would silently degenerate into a single-case lattice check.
    fn arb_verdict() -> proptest::strategy::BoxedStrategy<Verdict> {
        proptest::sample::select(vec![
            Verdict::Denied,
            Verdict::Allowed(SafetyLevel::Inert),
            Verdict::Allowed(SafetyLevel::SafeRead),
            Verdict::Allowed(SafetyLevel::SafeWrite),
        ])
        .boxed()
    }

    #[test]
    fn level_ordering() {
        assert!(SafetyLevel::Inert < SafetyLevel::SafeRead);
        assert!(SafetyLevel::SafeRead < SafetyLevel::SafeWrite);
    }

    #[test]
    fn threshold_names_map_new_and_legacy() {
        // current names — no legacy notice
        assert_eq!(SafetyLevel::resolve_threshold("paranoid"), Some((SafetyLevel::Inert, None)));
        assert_eq!(SafetyLevel::resolve_threshold("reader"), Some((SafetyLevel::SafeRead, None)));
        assert_eq!(SafetyLevel::resolve_threshold("editor"), Some((SafetyLevel::SafeWrite, None)));
        assert_eq!(SafetyLevel::resolve_threshold("developer"), Some((SafetyLevel::SafeWrite, None)));
        assert_eq!(SafetyLevel::resolve_threshold("yolo"), Some((SafetyLevel::SafeWrite, None)));

        // legacy names — same ceiling as before, carrying the current name for the notice
        assert_eq!(SafetyLevel::resolve_threshold("inert"), Some((SafetyLevel::Inert, Some("paranoid"))));
        assert_eq!(SafetyLevel::resolve_threshold("safe-read"), Some((SafetyLevel::SafeRead, Some("reader"))));
        assert_eq!(SafetyLevel::resolve_threshold("safe-write"), Some((SafetyLevel::SafeWrite, Some("developer"))));

        // the legacy inputs preserve their historical ceiling exactly (no setup breaks)
        assert_eq!(SafetyLevel::resolve_threshold("inert").unwrap().0, SafetyLevel::Inert);
        assert_eq!(SafetyLevel::resolve_threshold("safe-write").unwrap().0, SafetyLevel::SafeWrite);

        assert_eq!(SafetyLevel::resolve_threshold("nonsense"), None);
    }

    #[test]
    fn combine_both_allowed() {
        let a = Verdict::Allowed(SafetyLevel::Inert);
        let b = Verdict::Allowed(SafetyLevel::SafeRead);
        assert_eq!(a.combine(b), Verdict::Allowed(SafetyLevel::SafeRead));
    }

    #[test]
    fn combine_one_denied() {
        let a = Verdict::Allowed(SafetyLevel::Inert);
        assert_eq!(a.combine(Verdict::Denied), Verdict::Denied);
        assert_eq!(Verdict::Denied.combine(a), Verdict::Denied);
    }

    #[test]
    fn combine_identity() {
        let a = Verdict::Allowed(SafetyLevel::SafeWrite);
        let identity = Verdict::Allowed(SafetyLevel::Inert);
        assert_eq!(identity.combine(a), a);
    }

    proptest::proptest! {
        // Invariant (differential). `combine` is the lattice join of the verdict monoid: `Denied`
        // is the absorbing element and the `SafetyLevel` carries the max. When both operands are
        // `Allowed` it has to reduce to the `PartialOrd`-derived `max` of the two levels — without
        // that reduction the engine's "weakest writer wins" semantics break.
        #[test]
        fn combine_takes_the_max_of_levels_when_both_allowed(
            a in arb_safety_level(),
            b in arb_safety_level(),
        ) {
            let joined = Verdict::Allowed(a).combine(Verdict::Allowed(b));
            let expected = Verdict::Allowed(a.max(b));
            proptest::prop_assert_eq!(joined, expected);
        }

        // Invariant. `Denied` is the absorbing element of `combine` from either side, so the
        // join is `Denied` iff at least one operand is. The converse (right-to-left) direction is
        // the load-bearing one — `combine` must never synthesise a deny from two allows.
        #[test]
        fn combine_is_denied_iff_either_operand_is_denied(
            a in arb_verdict(),
            b in arb_verdict(),
        ) {
            let joined = a.combine(b);
            let one_denied = a == Verdict::Denied || b == Verdict::Denied;
            proptest::prop_assert_eq!(joined == Verdict::Denied, one_denied);
        }

        // Metamorphic. `combine` is the join of an `Allowed`-monoid-with-bottom lattice, so it
        // must be associative. Proptest canary: a non-associative `combine` would break every
        // caller that folds verdicts in a tree (e.g. per-argument aggregation into a command-wide
        // verdict).
        #[test]
        fn combine_is_associative(
            a in arb_verdict(),
            b in arb_verdict(),
            c in arb_verdict(),
        ) {
            proptest::prop_assert_eq!(a.combine(b).combine(c), a.combine(b.combine(c)));
        }

        // Roundtrip. `SafetyLevel`'s `Display` writes the legacy name. For every variant, that
        // string has to resolve back through `resolve_threshold` to the same level (and never to
        // `None`). Without this, anything that round-trips a level through a string (config files,
        // error messages, persisted state) silently flips safety class.
        #[test]
        fn level_display_string_resolves_back_to_the_same_level(
            level in arb_safety_level(),
        ) {
            let rendered = level.to_string();
            let resolved = SafetyLevel::resolve_threshold(&rendered);
            proptest::prop_assert_eq!(resolved.map(|(ceiling, _)| ceiling), Some(level));
        }
    }
}

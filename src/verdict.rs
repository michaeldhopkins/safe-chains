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
            "editor" | "developer" | "local-admin" | "network-admin" | "yolo" => {
                (SafetyLevel::SafeWrite, None)
            }
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
}

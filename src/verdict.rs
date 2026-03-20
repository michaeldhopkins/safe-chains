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

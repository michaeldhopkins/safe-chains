//! The capability vocabulary — the 12 facets of v1.4 §2.
//!
//! A [`Capability`] is one point in facet-space; a [`Profile`] is the set of
//! capabilities a resolved command line exhibits. Nothing here makes a decision —
//! admissibility (a level predicate over profiles) arrives in a later commit.
//!
//! Two kinds of facet term:
//! - **ordinal** — a severity/trust ladder; `#[derive(Ord)]` gives declaration
//!   order = the ladder, so a level can ceiling it (`facet <= term`) or floor it
//!   (`facet >= term`). The first-declared variant is the least-severe **zero term**
//!   and the [`Default`].
//! - **categorical** — a set with no severity ordering; admissibility is set
//!   membership, never `<=`. Deliberately *not* `Ord`, so a comparison can't be
//!   written by accident (the R25 bug: never order `kernel` vs `remote`).
//!
//! Compound facets (`locus`, `persistence`, `disclosure`, `secret`, `network`) are
//! structs of independent axes, each its own term — never collapsed to one ordinal.

/// A single facet term: the closed vocabulary of one axis, with its TOML spelling.
pub trait FacetTerm: Copy + Eq + Sized + 'static {
    /// Every term, in declaration order (for ordinals, least-severe first).
    fn all() -> &'static [Self];
    /// The term's canonical TOML spelling.
    fn as_str(self) -> &'static str;
    /// Parse a term from its TOML spelling.
    fn from_term(s: &str) -> Option<Self>;
}

macro_rules! ordinal_term {
    (
        $(#[$meta:meta])*
        $name:ident { $first:ident => $fs:literal $(, $rest:ident => $rs:literal)* $(,)? }
    ) => {
        $(#[$meta])*
        #[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub enum $name {
            #[default]
            $first,
            $($rest),*
        }
        impl FacetTerm for $name {
            fn all() -> &'static [Self] { &[Self::$first $(, Self::$rest)*] }
            fn as_str(self) -> &'static str {
                match self { Self::$first => $fs $(, Self::$rest => $rs)* }
            }
            fn from_term(s: &str) -> Option<Self> {
                match s { $fs => Some(Self::$first), $($rs => Some(Self::$rest),)* _ => None }
            }
        }
    };
}

macro_rules! categorical_term {
    (
        $(#[$meta:meta])*
        $name:ident { $first:ident => $fs:literal $(, $rest:ident => $rs:literal)* $(,)? }
    ) => {
        $(#[$meta])*
        #[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
        pub enum $name {
            #[default]
            $first,
            $($rest),*
        }
        impl FacetTerm for $name {
            fn all() -> &'static [Self] { &[Self::$first $(, Self::$rest)*] }
            fn as_str(self) -> &'static str {
                match self { Self::$first => $fs $(, Self::$rest => $rs)* }
            }
            fn from_term(s: &str) -> Option<Self> {
                match s { $fs => Some(Self::$first), $($rs => Some(Self::$rest),)* _ => None }
            }
        }
    };
}

// ── 2.1 The act ────────────────────────────────────────────────────────────────

categorical_term! {
    /// The operation a capability performs (v1.4 §2.1). One per capability.
    Operation {
        Observe => "observe",
        Create => "create",
        Mutate => "mutate",
        Destroy => "destroy",
        Execute => "execute",
        Communicate => "communicate",
        Configure => "configure",   // change settings that alter future commands
        Authorize => "authorize",   // change credentials/trust/access
        Control => "control",       // start/stop/signal processes, services, devices
    }
}

// ── 2.2 Reach ──────────────────────────────────────────────────────────────────

ordinal_term! {
    /// How deep into this host a capability reaches (v1.4 §2.2). `device`/`kernel`
    /// void the abstractions the fs rungs assume and are deny-by-default everywhere.
    LocalLocus {
        Process => "process",
        Temp => "temp",
        SandboxScope => "sandbox-scope",
        Worktree => "worktree",
        WorktreeTrusted => "worktree-trusted",  // .git/, .envrc, hooks, CI configs
        User => "user",                         // ~, keychain
        Machine => "machine",                   // /etc, services, other users
        Device => "device",                     // raw block/char devices
        Kernel => "kernel",                     // module/extension load
    }
}

ordinal_term! {
    /// Which other host a capability reaches (v1.4 §2.2, the reach axis of `locus`).
    RemoteReach {
        None => "none",
        Fixed => "fixed",
        Arbitrary => "arbitrary",
    }
}

categorical_term! {
    /// Whether a remote target is named on the command line or taken from session
    /// state — the pinned-vs-ambient bit `infra` gates on (v1.4 §2.2, HP-12).
    RemoteBinding {
        Na => "n/a",         // no remote reach
        Pinned => "pinned",  // host/context/profile explicit on the command line
        Ambient => "ambient",
    }
}

ordinal_term! {
    /// Breadth of effect (v1.4 §2.2). Modifies `destroy` *and* `disclosure` (R23).
    Scale {
        Single => "single",
        Bounded => "bounded",      // a glob/dir/explicit list
        Unbounded => "unbounded",  // recursion / mass op
    }
}

ordinal_term! {
    /// Privilege the capability runs with (v1.4 §2.2).
    Authority {
        User => "user",
        Elevated => "elevated",      // sudo/doas
        Root => "root",
        OtherUser => "other-user",   // setuid/run-as
    }
}

ordinal_term! {
    /// Isolation strength of an enclosing frame (v1.4 §2.2). A frame clamps nested
    /// `locus` to `sandbox-scope`; breach flags re-add loci (§3.2).
    Isolation {
        None => "none",
        View => "view",             // chroot
        Namespace => "namespace",
        Userns => "userns",
        Vm => "vm",
        Ocap => "ocap",
    }
}

// ── 2.3 Durability ─────────────────────────────────────────────────────────────

ordinal_term! {
    /// How hard the effect is to undo (v1.4 §2.3). Environment-dependent cases
    /// resolve worst-case (HP-8).
    Reversibility {
        None => "none",              // pure observe
        Trivial => "trivial",        // idempotent/undo
        Recoverable => "recoverable",// VCS/recycle/snapshot
        Effortful => "effortful",    // out-of-band backups only
        Irreversible => "irreversible",
    }
}

ordinal_term! {
    /// What the capability leaves behind (v1.4 §2.3, the level axis of `persistence`).
    PersistenceLevel {
        Transient => "transient",
        Data => "data",
        Reconfiguring => "reconfiguring",  // alters future commands
        Installing => "installing",        // adds executables/services/hooks
    }
}

ordinal_term! {
    /// How far execution escapes the check (v1.4 §2.3, R16/R24) — the part levels gate.
    TriggerEscape {
        Immediate => "immediate",  // done on return
        Detached => "detached",    // one instance survives the session (nohup/setsid)
        Recurring => "recurring",  // re-fires until removed
        Boot => "boot",            // re-fires and survives reboot (systemctl enable, @reboot)
    }
}

categorical_term! {
    /// The kind of recurrence (v1.4 §2.3) — for the `because` string, not a severity
    /// rung: a per-save `event` can fire more often than a monthly `clock`.
    TriggerKind {
        None => "none",     // not recurring
        Clock => "clock",   // cron, at
        Event => "event",   // watchexec, git hooks, .envrc on cd
    }
}

// ── 2.4 Information exposure ────────────────────────────────────────────────────

ordinal_term! {
    /// Who ends up able to see disclosed output (v1.4 §2.4). `local-process` is
    /// stdout → the agent/model provider — the HP-15 audience that gates secret reads.
    DisclosureAudience {
        None => "none",
        LocalProcess => "local-process",       // stdout → the agent/model
        LocalPersistent => "local-persistent", // other local users
        TrustedRemote => "trusted-remote",
        SharedRemote => "shared-remote",
        Public => "public",
    }
}

ordinal_term! {
    /// A capability's relationship to secret material (v1.4 §2.4).
    SecretLevel {
        None => "none",
        UsesAmbient => "uses-ambient",
        Reads => "reads",
        Writes => "writes",
        Transmits => "transmits",
    }
}

categorical_term! {
    /// The channel a disclosure or secret flows over (v1.4 §2.4). An **open set**:
    /// an unrecognized/covert channel is `Unknown` and worst-cased by the resolver.
    Channel {
        None => "none",
        Filesystem => "filesystem",
        StdoutToModel => "stdout-to-model",
        Network => "network",
        Clipboard => "clipboard",              // pbcopy/pbpaste
        Ipc => "ipc",
        CredentialStore => "credential-store", // keychain
        CrossProcess => "cross-process",       // lldb -p, /proc/*/mem
        Unknown => "unknown",
    }
}

categorical_term! {
    /// Whose data a read touches (v1.4 §2.4) — a read can cross a principal boundary
    /// on the same host (another process's memory/argv) with no fs or network touch.
    Principal {
        Own => "own",
        Cross => "cross",
    }
}

// ── 2.5 Channel (network) ──────────────────────────────────────────────────────

ordinal_term! {
    /// Network direction (v1.4 §2.5).
    NetDirection {
        None => "none",
        Loopback => "loopback",
        Outbound => "outbound",
        InboundListen => "inbound-listen",
    }
}

ordinal_term! {
    /// Network destination (v1.4 §2.5). Same axis as `locus.remote` reach.
    NetDestination {
        Na => "n/a",
        Fixed => "fixed",
        Arbitrary => "arbitrary",
    }
}

ordinal_term! {
    /// What a network capability carries (v1.4 §2.5).
    NetPayload {
        None => "none",
        Fetches => "fetches",
        SendsHostData => "sends-host-data",
    }
}

// ── 2.6 Code provenance ────────────────────────────────────────────────────────

ordinal_term! {
    /// Where executed code comes from (v1.4 §2.6, local-trust ladder). When
    /// `NetworkSourced`, the supply-chain sub-facets ([`SupplyChain`]) refine it.
    ExecutionTrust {
        None => "none",
        SelfCode => "self",
        CallerInline => "caller-inline",
        CallerFile => "caller-file",
        AmbientConfig => "ambient-config",   // Makefile/hooks/.envrc/plugins
        NetworkSourced => "network-sourced",
    }
}

categorical_term! {
    /// Where network-sourced code came from (v1.4 §2.6). Categorical — a level lists
    /// the sources it accepts rather than assuming a severity order.
    SupplySource {
        UnverifiedUrl => "unverified-url",
        PublicRegistry => "public-registry",
        SignedRepo => "signed-repo",
        PrivateRegistry => "private-registry",
        Vendored => "vendored",
    }
}

ordinal_term! {
    /// How tightly a fetched artifact is pinned (v1.4 §2.6). A *trust* ladder: higher
    /// is safer, so a level floors it (`>= version`) rather than ceilings it.
    Pinning {
        Floating => "floating",
        Version => "version",
        HashVerified => "hash-verified",
        Digest => "digest",
    }
}

categorical_term! {
    /// When/what fetched code runs (v1.4 §2.6). Categorical — the risk order across
    /// install-hook / build-script / call-time / run-artifact is genuinely unclear, so
    /// a level lists the surfaces it accepts instead of ceiling-ing a false ladder.
    ExecSurface {
        None => "none",
        InstallHook => "install-hook",   // code on install (npm lifecycle, pip setup.py)
        BuildScript => "build-script",   // code on build (cargo build.rs, node-gyp)
        CallTime => "call-time",         // deps' code runs only when your program runs
        RunArtifact => "run-artifact",   // you execute the fetched binary/image
    }
}

// ── 2.7 Resource ───────────────────────────────────────────────────────────────

ordinal_term! {
    /// Resource/billing cost (v1.4 §2.7). Populated for provisioning tools.
    Cost {
        None => "none",
        LocalResource => "local-resource",
        Metered => "metered",   // billable
        Quota => "quota",
    }
}

// ── 2.8 Compound facets & the capability record ────────────────────────────────

/// Reach — two independent axes (v1.4 §2.2, R25).
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Locus {
    pub local: LocalLocus,
    pub remote: RemoteReach,
    pub binding: RemoteBinding,
}

/// Durability trigger — how far execution escapes, and (if recurring) what kind.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Trigger {
    pub escape: TriggerEscape,
    pub kind: TriggerKind,
}

/// What a capability leaves behind, and when it re-fires (v1.4 §2.3).
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Persistence {
    pub level: PersistenceLevel,
    pub trigger: Trigger,
}

/// Where disclosed output goes, over which channel, whose data (v1.4 §2.4).
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Disclosure {
    pub audience: DisclosureAudience,
    pub channel: Channel,
    pub principal: Principal,
}

/// A capability's relationship to secrets, over which channel, whose (v1.4 §2.4).
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Secret {
    pub level: SecretLevel,
    pub channel: Channel,
    pub principal: Principal,
}

/// A network capability's shape (v1.4 §2.5).
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Network {
    pub direction: NetDirection,
    pub destination: NetDestination,
    pub payload: NetPayload,
}

/// The provenance of network-sourced code (v1.4 §2.6).
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct SupplyChain {
    pub source: SupplySource,
    pub pinning: Pinning,
    pub exec_surface: ExecSurface,
}

/// Code provenance: the local-trust rung, plus supply-chain detail when the code is
/// network-sourced (v1.4 §2.6). `supply_chain` is present only for network-sourced
/// execution — a command running no downloaded code leaves it `None`, and a level's
/// supply-chain constraints are then vacuously satisfied.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Execution {
    pub trust: ExecutionTrust,
    pub supply_chain: Option<SupplyChain>,
}

/// One capability — a single point in facet-space (v1.4 §2.8). Facets left unset
/// default to their zero term. `because` cites the discriminator (§5); the nested
/// delegate profile and supply-chain sub-facets arrive with the mechanisms that
/// need them.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Capability {
    pub operation: Operation,
    pub locus: Locus,
    pub scale: Scale,
    pub authority: Authority,
    pub isolation: Isolation,
    pub reversibility: Reversibility,
    pub persistence: Persistence,
    pub disclosure: Disclosure,
    pub secret: Secret,
    pub network: Network,
    pub execution: Execution,
    pub cost: Cost,
    pub because: String,
}

impl Capability {
    /// A capability performing `operation`, every other facet at its zero term.
    pub fn new(operation: Operation) -> Self {
        Self { operation, ..Self::default() }
    }

    /// A maximally-severe capability — every axis at its worst term, so no well-formed
    /// level admits it (`locus.local = kernel` alone denies it everywhere, v1.4 §4.3).
    /// The resolver returns this when it cannot certify something (§0), keeping the
    /// engine from being *looser* than a strict classifier on a form it doesn't
    /// understand. Ordinal worsts are the ladder tops; categorical worsts are the
    /// hazardous term (`Channel::Unknown`, `Principal::Cross`).
    pub fn worst(because: impl Into<String>) -> Self {
        Self {
            operation: Operation::Execute,
            locus: Locus {
                local: LocalLocus::Kernel,
                remote: RemoteReach::Arbitrary,
                binding: RemoteBinding::Ambient,
            },
            scale: Scale::Unbounded,
            authority: Authority::OtherUser,
            isolation: Isolation::None,
            reversibility: Reversibility::Irreversible,
            persistence: Persistence {
                level: PersistenceLevel::Installing,
                trigger: Trigger { escape: TriggerEscape::Boot, kind: TriggerKind::None },
            },
            disclosure: Disclosure {
                audience: DisclosureAudience::Public,
                channel: Channel::Unknown,
                principal: Principal::Cross,
            },
            secret: Secret {
                level: SecretLevel::Transmits,
                channel: Channel::Unknown,
                principal: Principal::Cross,
            },
            network: Network {
                direction: NetDirection::InboundListen,
                destination: NetDestination::Arbitrary,
                payload: NetPayload::SendsHostData,
            },
            execution: Execution { trust: ExecutionTrust::NetworkSourced, supply_chain: None },
            cost: Cost::Quota,
            because: because.into(),
        }
    }
}

/// The set of capabilities a resolved command line exhibits (v1.4 §2.8, §4.1). A
/// profile passes a level iff *every* capability is admissible.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Profile {
    pub capabilities: Vec<Capability>,
}

impl Profile {
    /// A profile of exactly these capabilities.
    pub fn of(capabilities: Vec<Capability>) -> Self {
        Self { capabilities }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_term_strings_roundtrip<T: FacetTerm + std::fmt::Debug>() {
        for &term in T::all() {
            assert_eq!(
                T::from_term(term.as_str()),
                Some(term),
                "term {:?} did not round-trip through {:?}",
                term,
                term.as_str(),
            );
        }
        for (i, &a) in T::all().iter().enumerate() {
            for &b in &T::all()[i + 1..] {
                assert_ne!(a.as_str(), b.as_str(), "two variants share a TOML spelling");
            }
        }
        assert_eq!(T::from_term("definitely-not-a-term"), None);
    }

    fn assert_zero_is_minimum<T: FacetTerm + Ord + Default + std::fmt::Debug>() {
        let zero = T::all()[0];
        assert_eq!(T::default(), zero, "Default must be the zero term (first variant)");
        for &term in T::all() {
            assert!(zero <= term, "zero term {zero:?} is not <= {term:?}");
        }
    }

    #[test]
    fn every_term_roundtrips_and_is_uniquely_spelled() {
        assert_term_strings_roundtrip::<Operation>();
        assert_term_strings_roundtrip::<LocalLocus>();
        assert_term_strings_roundtrip::<RemoteReach>();
        assert_term_strings_roundtrip::<RemoteBinding>();
        assert_term_strings_roundtrip::<Scale>();
        assert_term_strings_roundtrip::<Authority>();
        assert_term_strings_roundtrip::<Isolation>();
        assert_term_strings_roundtrip::<Reversibility>();
        assert_term_strings_roundtrip::<PersistenceLevel>();
        assert_term_strings_roundtrip::<TriggerEscape>();
        assert_term_strings_roundtrip::<TriggerKind>();
        assert_term_strings_roundtrip::<DisclosureAudience>();
        assert_term_strings_roundtrip::<SecretLevel>();
        assert_term_strings_roundtrip::<Channel>();
        assert_term_strings_roundtrip::<Principal>();
        assert_term_strings_roundtrip::<NetDirection>();
        assert_term_strings_roundtrip::<NetDestination>();
        assert_term_strings_roundtrip::<NetPayload>();
        assert_term_strings_roundtrip::<ExecutionTrust>();
        assert_term_strings_roundtrip::<SupplySource>();
        assert_term_strings_roundtrip::<Pinning>();
        assert_term_strings_roundtrip::<ExecSurface>();
        assert_term_strings_roundtrip::<Cost>();
    }

    #[test]
    fn ordinal_zero_terms_are_the_minimum() {
        assert_zero_is_minimum::<LocalLocus>();
        assert_zero_is_minimum::<RemoteReach>();
        assert_zero_is_minimum::<Scale>();
        assert_zero_is_minimum::<Authority>();
        assert_zero_is_minimum::<Isolation>();
        assert_zero_is_minimum::<Reversibility>();
        assert_zero_is_minimum::<PersistenceLevel>();
        assert_zero_is_minimum::<TriggerEscape>();
        assert_zero_is_minimum::<DisclosureAudience>();
        assert_zero_is_minimum::<SecretLevel>();
        assert_zero_is_minimum::<NetDirection>();
        assert_zero_is_minimum::<NetDestination>();
        assert_zero_is_minimum::<NetPayload>();
        assert_zero_is_minimum::<ExecutionTrust>();
        assert_zero_is_minimum::<Pinning>();
        assert_zero_is_minimum::<Cost>();
    }

    #[test]
    fn ordinal_ladders_match_the_spec() {
        assert!(LocalLocus::Process < LocalLocus::Worktree);
        assert!(LocalLocus::Worktree < LocalLocus::Machine);
        assert!(LocalLocus::Machine < LocalLocus::Device);
        assert!(LocalLocus::Device < LocalLocus::Kernel);
        assert!(Scale::Single < Scale::Bounded && Scale::Bounded < Scale::Unbounded);
        assert!(Authority::User < Authority::Root && Authority::Root < Authority::OtherUser);
        assert!(Reversibility::Recoverable < Reversibility::Irreversible);
        assert!(PersistenceLevel::Data < PersistenceLevel::Installing);
        assert!(TriggerEscape::Immediate < TriggerEscape::Boot);
        assert!(DisclosureAudience::LocalProcess < DisclosureAudience::Public);
        assert!(SecretLevel::Reads < SecretLevel::Transmits);
        assert!(ExecutionTrust::SelfCode < ExecutionTrust::NetworkSourced);
        assert!(Pinning::Floating < Pinning::HashVerified);
    }

    #[test]
    fn capability_new_leaves_all_other_facets_at_zero() {
        let cap = Capability::new(Operation::Destroy);
        assert_eq!(cap.operation, Operation::Destroy);
        assert_eq!(cap.locus, Locus::default());
        assert_eq!(cap.locus.local, LocalLocus::Process);
        assert_eq!(cap.scale, Scale::Single);
        assert_eq!(cap.authority, Authority::User);
        assert_eq!(cap.reversibility, Reversibility::None);
        assert_eq!(cap.secret.level, SecretLevel::None);
        assert_eq!(cap.disclosure.audience, DisclosureAudience::None);
        assert_eq!(cap.network.direction, NetDirection::None);
        assert_eq!(cap.execution.trust, ExecutionTrust::None);
        assert!(cap.execution.supply_chain.is_none());
        assert_eq!(cap.cost, Cost::None);
        assert!(cap.because.is_empty());
    }

    #[test]
    fn default_capability_is_a_zero_observe() {
        assert_eq!(Capability::default().operation, Operation::Observe);
        assert_eq!(Capability::default(), Capability::new(Operation::Observe));
    }
}

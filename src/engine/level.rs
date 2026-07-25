//! Safety levels as predicates over profiles (v1.4 §4.1).
//!
//! A [`Level`] is a disjunction of allow [`Clause`]s plus an allow-only set of deny
//! clauses (the `yolo` subtractive primitive, §4.1). A [`Clause`] is a conjunction of
//! per-facet bounds — an ordinal ceiling/floor ([`OrdBound`]) or a categorical set —
//! and an omitted facet is unconstrained. A capability is admissible iff **some**
//! allow clause admits every one of its facets and **no** deny clause matches it; a
//! profile passes iff every capability is admissible.
//!
//! Nothing here parses TOML or ships a default level yet — this is the algebra and
//! its contract. Level authoring (TOML → `Level`) and the default set arrive next.

use super::facet::*;

/// An ordinal bound: `min ≤ term ≤ max`, either side optional. `at_most` is a
/// ceiling (the common risk-facet form); `at_least` a floor (trust facets like
/// pinning). An omitted side is unconstrained.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct OrdBound<T> {
    pub min: Option<T>,
    pub max: Option<T>,
}

impl<T: Ord + Copy> OrdBound<T> {
    /// `term ≤ ceiling`.
    pub fn at_most(ceiling: T) -> Self {
        Self { min: None, max: Some(ceiling) }
    }
    /// `term ≥ floor`.
    pub fn at_least(floor: T) -> Self {
        Self { min: Some(floor), max: None }
    }
    /// `term == exact`.
    pub fn exactly(exact: T) -> Self {
        Self { min: Some(exact), max: Some(exact) }
    }
    /// Whether `term` falls within the bound.
    pub fn admits(self, term: T) -> bool {
        self.min.is_none_or(|lo| lo <= term) && self.max.is_none_or(|hi| term <= hi)
    }
}

fn ord_admits<T: Ord + Copy>(bound: Option<OrdBound<T>>, term: T) -> bool {
    bound.is_none_or(|b| b.admits(term))
}

fn set_admits<T: Eq + Copy>(set: Option<&[T]>, term: T) -> bool {
    set.is_none_or(|s| s.contains(&term))
}

/// The single facet constraint a capability failed, named in the taxonomy's own vocabulary.
///
/// Exists because the engine was write-only: `admits` answered yes/no and nothing reported WHICH
/// axis said no, so both a user asking "why was this denied" and an author debugging a resolver had
/// to bisect by editing facets and re-running. That is not a hypothetical cost — it is how an
/// incomplete loopback delta got mistaken for a flawed approach.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FacetMismatch {
    /// Dotted facet path as the taxonomy spells it (`network.payload`, `locus.remote`).
    pub facet: &'static str,
    /// The capability's term (`sends-host-data`).
    pub actual: &'static str,
    /// The constraint it failed (`<= fetches`, `one of [pinned, na]`).
    pub bound: String,
}

impl std::fmt::Display for FacetMismatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} = {} (allowed: {})", self.facet, self.actual, self.bound)
    }
}

fn ord_mismatch<T: Ord + Copy + FacetTerm>(
    facet: &'static str,
    bound: Option<OrdBound<T>>,
    term: T,
) -> Option<FacetMismatch> {
    let b = bound?;
    if b.admits(term) {
        return None;
    }
    let bound = match (b.min, b.max) {
        (None, Some(hi)) => format!("<= {}", hi.as_str()),
        (Some(lo), None) => format!(">= {}", lo.as_str()),
        (Some(lo), Some(hi)) => format!("{}..={}", lo.as_str(), hi.as_str()),
        (None, None) => "any".to_string(),
    };
    Some(FacetMismatch { facet, actual: term.as_str(), bound })
}

fn set_mismatch<T: PartialEq + Copy + FacetTerm>(
    facet: &'static str,
    set: Option<&[T]>,
    term: T,
) -> Option<FacetMismatch> {
    let s = set?;
    if s.contains(&term) {
        return None;
    }
    let bound = format!(
        "one of [{}]",
        s.iter().map(|t| t.as_str()).collect::<Vec<_>>().join(", "),
    );
    Some(FacetMismatch { facet, actual: term.as_str(), bound })
}

/// A conjunction of per-facet constraints. A default (all-`None`) clause admits every
/// capability. Each ordinal facet takes an [`OrdBound`]; each categorical facet an
/// allowed set. Fields are flattened per axis — a compound facet is never a single
/// constraint (the R25 discipline).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Clause {
    pub operation: Option<Vec<Operation>>,
    pub local_locus: Option<OrdBound<LocalLocus>>,
    pub remote_reach: Option<OrdBound<RemoteReach>>,
    pub remote_binding: Option<Vec<RemoteBinding>>,
    pub provenance: Option<OrdBound<Provenance>>,
    pub scale: Option<OrdBound<Scale>>,
    pub retrieval: Option<OrdBound<RetrievalGranularity>>,
    pub authority: Option<OrdBound<Authority>>,
    pub isolation: Option<OrdBound<Isolation>>,
    pub reversibility: Option<OrdBound<Reversibility>>,
    pub persistence_level: Option<OrdBound<PersistenceLevel>>,
    pub trigger_escape: Option<OrdBound<TriggerEscape>>,
    pub trigger_kind: Option<Vec<TriggerKind>>,
    pub disclosure_audience: Option<OrdBound<DisclosureAudience>>,
    pub disclosure_channel: Option<Vec<Channel>>,
    pub disclosure_principal: Option<Vec<Principal>>,
    pub secret_level: Option<OrdBound<SecretLevel>>,
    pub secret_channel: Option<Vec<Channel>>,
    pub secret_principal: Option<Vec<Principal>>,
    pub net_direction: Option<OrdBound<NetDirection>>,
    pub net_destination: Option<OrdBound<NetDestination>>,
    pub net_payload: Option<OrdBound<NetPayload>>,
    pub execution_trust: Option<OrdBound<ExecutionTrust>>,
    pub supply_source: Option<Vec<SupplySource>>,
    pub pinning: Option<OrdBound<Pinning>>,
    pub exec_surface: Option<Vec<ExecSurface>>,
    pub cost: Option<OrdBound<Cost>>,
}

impl Clause {
    /// Whether this clause **admits** `cap` (allow-clause semantics).
    pub fn admits(&self, cap: &Capability) -> bool {
        self.check(cap, Role::Allow)
    }

    /// Whether this clause **matches** `cap` for removal (deny-clause semantics). Differs
    /// from `admits` only on the optional `supply_chain`: a deny constrained on a
    /// supply-chain facet does **not** match a capability that has no supply chain (it is
    /// not in the denied corner), whereas an allow treats the absence as vacuously
    /// satisfied. Sharing one body with a naive vacuous-true rule would make a
    /// supply-chain-only deny wrongly match every non-supply-chain capability.
    fn matches_as_deny(&self, cap: &Capability) -> bool {
        self.check(cap, Role::Deny)
    }

    fn check(&self, cap: &Capability, role: Role) -> bool {
        self.first_mismatch(cap, role).is_none()
    }

    /// The first facet constraint `cap` fails, or `None` if this clause admits it.
    ///
    /// `check` is defined in terms of THIS rather than the other way round. A parallel
    /// "explain" walk beside a separate `&&` chain would drift the moment a facet is added, and a
    /// diagnostic that disagrees with the predicate is worse than none — it would send an author
    /// chasing the wrong axis. `clause_diagnostic_agrees_with_admits` pins the equivalence.
    ///
    /// Order is the taxonomy's axis order, not severity: it reports A failing facet, not the most
    /// important one, since a clause is a conjunction and every failure is disqualifying.
    /// Test-only view of `first_mismatch`. Takes the ROLE: the allow/deny asymmetry on the supply
    /// chain means an allow-only equivalence proptest cannot see a deny-side inversion (it did not
    /// — a `?` that reported "no complaint" for a definitely-failing deny got through it).
    #[cfg(test)]
    pub(crate) fn first_mismatch_for_test(
        &self,
        cap: &Capability,
        deny_role: bool,
    ) -> Option<FacetMismatch> {
        self.first_mismatch(cap, if deny_role { Role::Deny } else { Role::Allow })
    }

    #[cfg(test)]
    pub(crate) fn matches_as_deny_for_test(&self, cap: &Capability) -> bool {
        self.matches_as_deny(cap)
    }

    fn first_mismatch(&self, cap: &Capability, role: Role) -> Option<FacetMismatch> {
        set_mismatch("operation", self.operation.as_deref(), cap.operation)
            .or_else(|| ord_mismatch("locus.local", self.local_locus, cap.locus.local))
            .or_else(|| ord_mismatch("locus.remote", self.remote_reach, cap.locus.remote))
            .or_else(|| set_mismatch("locus.binding", self.remote_binding.as_deref(), cap.locus.binding))
            .or_else(|| ord_mismatch("locus.provenance", self.provenance, cap.locus.provenance))
            .or_else(|| ord_mismatch("scale", self.scale, cap.scale))
            .or_else(|| ord_mismatch("retrieval", self.retrieval, cap.retrieval))
            .or_else(|| ord_mismatch("authority", self.authority, cap.authority))
            .or_else(|| ord_mismatch("isolation", self.isolation, cap.isolation))
            .or_else(|| ord_mismatch("reversibility", self.reversibility, cap.reversibility))
            .or_else(|| ord_mismatch("persistence.level", self.persistence_level, cap.persistence.level))
            .or_else(|| ord_mismatch("persistence.trigger.escape", self.trigger_escape, cap.persistence.trigger.escape))
            .or_else(|| set_mismatch("persistence.trigger.kind", self.trigger_kind.as_deref(), cap.persistence.trigger.kind))
            .or_else(|| ord_mismatch("disclosure.audience", self.disclosure_audience, cap.disclosure.audience))
            .or_else(|| set_mismatch("disclosure.channel", self.disclosure_channel.as_deref(), cap.disclosure.channel))
            .or_else(|| set_mismatch("disclosure.principal", self.disclosure_principal.as_deref(), cap.disclosure.principal))
            .or_else(|| ord_mismatch("secret.level", self.secret_level, cap.secret.level))
            .or_else(|| set_mismatch("secret.channel", self.secret_channel.as_deref(), cap.secret.channel))
            .or_else(|| set_mismatch("secret.principal", self.secret_principal.as_deref(), cap.secret.principal))
            .or_else(|| ord_mismatch("network.direction", self.net_direction, cap.network.direction))
            .or_else(|| ord_mismatch("network.destination", self.net_destination, cap.network.destination))
            .or_else(|| ord_mismatch("network.payload", self.net_payload, cap.network.payload))
            .or_else(|| ord_mismatch("execution.trust", self.execution_trust, cap.execution.trust))
            .or_else(|| self.supply_chain_mismatch(cap.execution.supply_chain, role))
            .or_else(|| ord_mismatch("cost", self.cost, cap.cost))
    }

    /// Supply-chain constraints apply only to network-sourced code. For an **allow**, a
    /// capability with no supply chain (`None`) satisfies them vacuously. For a **deny**,
    /// a clause that constrains the supply chain does **not** match a `None` capability —
    /// it is not in the denied corner — while an unconstrained deny is unaffected.
    fn supply_chain_mismatch(&self, sc: Option<SupplyChain>, role: Role) -> Option<FacetMismatch> {
        if self.supply_chain_admits(sc, role) {
            return None;
        }
        let Some(sc) = sc else {
            // A DENY constrained on the supply chain does not match a capability that has none.
            // `?` here would report "no complaint", which `check` reads as ADMITS — inverting the
            // rule for exactly the case the asymmetry exists to handle.
            return Some(FacetMismatch {
                facet: "execution.supply_chain",
                actual: "absent",
                bound: "this clause constrains the supply chain, which this capability has none of"
                    .to_string(),
            });
        };
        set_mismatch("supply_chain.source", self.supply_source.as_deref(), sc.source)
            .or_else(|| ord_mismatch("supply_chain.pinning", self.pinning, sc.pinning))
            .or_else(|| set_mismatch("supply_chain.exec_surface", self.exec_surface.as_deref(), sc.exec_surface))
    }

    fn supply_chain_admits(&self, sc: Option<SupplyChain>, role: Role) -> bool {
        match sc {
            None => match role {
                Role::Allow => true,
                Role::Deny => {
                    self.supply_source.is_none()
                        && self.pinning.is_none()
                        && self.exec_surface.is_none()
                }
            },
            Some(sc) => {
                set_admits(self.supply_source.as_deref(), sc.source)
                    && ord_admits(self.pinning, sc.pinning)
                    && set_admits(self.exec_surface.as_deref(), sc.exec_surface)
            }
        }
    }
}

/// Which side a clause is evaluated on — the two differ only on absent `supply_chain`.
#[derive(Copy, Clone, PartialEq, Eq)]
enum Role {
    Allow,
    Deny,
}

/// A safety level: a name, its allow clauses (disjunction), and its deny clauses
/// (allow-only subtractive corners, §4.1).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Level {
    pub name: String,
    pub allow: Vec<Clause>,
    pub deny: Vec<Clause>,
}

impl Level {
    /// An empty level (admits only the empty profile). Build it up with
    /// [`Level::allowing`] / [`Level::denying`].
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into(), allow: Vec::new(), deny: Vec::new() }
    }

    /// Add an allow clause (widens the admissible region).
    #[must_use]
    pub fn allowing(mut self, clause: Clause) -> Self {
        self.allow.push(clause);
        self
    }

    /// Add a deny clause (subtracts a corner; never grants).
    #[must_use]
    pub fn denying(mut self, clause: Clause) -> Self {
        self.deny.push(clause);
        self
    }

    /// Whether a single capability is admissible: some allow clause admits it and no
    /// deny clause matches it.
    /// Why this level does not admit `cap`, or `None` if it does.
    ///
    /// A level is a DISJUNCTION of allow clauses, so a rejected capability failed all of them and
    /// there is a choice of which complaint to report. Clauses are usually split by operation
    /// (`editor` allows observes under one clause and mutates under another), so the first clause's
    /// gripe is frequently "operation = mutate, allowed: [observe]" — true, useless, and it hides
    /// the facet the author actually needs to change. Preferring a clause whose OPERATION already
    /// matches surfaces the real blocker.
    pub fn nearest_miss(&self, cap: &Capability) -> Option<FacetMismatch> {
        if self.allow.iter().any(|c| c.admits(cap)) {
            // Admitted by an allow, so if the level still rejects it a deny clause removed it.
            return self.deny.iter().find(|c| c.matches_as_deny(cap)).map(|_| FacetMismatch {
                facet: "deny-clause",
                actual: "matched",
                bound: "removed by an explicit deny clause".to_string(),
            });
        }
        if self.allow.is_empty() {
            return Some(FacetMismatch {
                facet: "level",
                actual: "any capability",
                bound: "nothing — this level declares no allow clause".to_string(),
            });
        }
        let on_topic = |c: &&Clause| {
            c.operation.as_ref().is_none_or(|ops| ops.contains(&cap.operation))
        };
        self.allow
            .iter()
            .filter(on_topic)
            .chain(self.allow.iter())
            .find_map(|c| c.first_mismatch(cap, Role::Allow))
    }

    pub fn admits_capability(&self, cap: &Capability) -> bool {
        self.allow.iter().any(|c| c.admits(cap)) && !self.deny.iter().any(|c| c.matches_as_deny(cap))
    }

    /// Whether a whole profile passes: every capability is admissible. The empty
    /// profile passes vacuously.
    pub fn admits(&self, profile: &Profile) -> bool {
        profile.capabilities.iter().all(|c| self.admits_capability(c))
    }

    /// Author a level by extending `base` (R27: `extends` only loosens). The result
    /// inherits `base`'s allow *and* deny clauses unchanged and adds only allow
    /// clauses — it cannot drop an allow or add a deny, so `extends ⇒ superset` holds
    /// by construction. `yolo`'s subtractive `deny` is authored directly, never via
    /// `extend`.
    #[must_use]
    pub fn extend(base: &Level, name: impl Into<String>, extra_allow: Vec<Clause>) -> Level {
        let mut allow = base.allow.clone();
        allow.extend(extra_allow);
        Level { name: name.into(), allow, deny: base.deny.clone() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn cap(op: Operation) -> Capability {
        Capability::new(op)
    }

    #[test]
    fn empty_clause_admits_everything_empty_allow_admits_nothing() {
        let all = Level::new("all").allowing(Clause::default());
        let nothing = Level::new("nothing");
        let destroy = Profile::of(vec![cap(Operation::Destroy)]);
        assert!(all.admits(&destroy));
        assert!(!nothing.admits(&destroy));
        // both admit the empty profile vacuously
        assert!(all.admits(&Profile::default()));
        assert!(nothing.admits(&Profile::default()));
    }

    #[test]
    fn read_local_admits_a_read_rejects_a_secret_and_a_destroy() {
        let read_local = Level::new("read-local").allowing(Clause {
            operation: Some(vec![Operation::Observe]),
            local_locus: Some(OrdBound::at_most(LocalLocus::User)),
            secret_level: Some(OrdBound::at_most(SecretLevel::UsesAmbient)),
            net_direction: Some(OrdBound::at_most(NetDirection::Loopback)),
            ..Default::default()
        });

        let plain_read = Profile::of(vec![{
            let mut c = cap(Operation::Observe);
            c.locus.local = LocalLocus::Worktree;
            c
        }]);
        assert!(read_local.admits(&plain_read));

        let secret_read = Profile::of(vec![{
            let mut c = cap(Operation::Observe);
            c.locus.local = LocalLocus::User;
            c.secret.level = SecretLevel::Reads;
            c
        }]);
        assert!(!read_local.admits(&secret_read), "cat ~/.ssh/id_rsa must not pass read-local");

        let destroy = Profile::of(vec![cap(Operation::Destroy)]);
        assert!(!read_local.admits(&destroy));
    }

    #[test]
    fn yolo_deny_carves_out_the_catastrophe_corner() {
        let yolo = Level::new("yolo").allowing(Clause::default()).denying(Clause {
            operation: Some(vec![Operation::Destroy]),
            reversibility: Some(OrdBound::at_least(Reversibility::Irreversible)),
            scale: Some(OrdBound::at_least(Scale::Unbounded)),
            ..Default::default()
        });

        // rm -rf ./node_modules — destroy·bounded·recoverable → admitted
        let bounded = Profile::of(vec![{
            let mut c = cap(Operation::Destroy);
            c.scale = Scale::Bounded;
            c.reversibility = Reversibility::Recoverable;
            c
        }]);
        assert!(yolo.admits(&bounded));

        // rm -rf / — destroy·unbounded·irreversible → denied by the corner
        let wipe = Profile::of(vec![{
            let mut c = cap(Operation::Destroy);
            c.scale = Scale::Unbounded;
            c.reversibility = Reversibility::Irreversible;
            c
        }]);
        assert!(!yolo.admits(&wipe));
    }

    #[test]
    fn supply_chain_gates_network_sourced_code_and_is_vacuous_otherwise() {
        // a developer-shaped clause: network-sourced execution is fine, but only from a
        // recognized registry (not an unverified URL).
        let dev = Level::new("dev").allowing(Clause {
            execution_trust: Some(OrdBound::at_most(ExecutionTrust::NetworkSourced)),
            supply_source: Some(vec![
                SupplySource::PublicRegistry,
                SupplySource::SignedRepo,
                SupplySource::PrivateRegistry,
                SupplySource::Vendored,
            ]),
            ..Default::default()
        });

        // cargo build — network-sourced from a public registry
        let build = Profile::of(vec![{
            let mut c = cap(Operation::Execute);
            c.execution = Execution {
                trust: ExecutionTrust::NetworkSourced,
                supply_chain: Some(SupplyChain {
                    source: SupplySource::PublicRegistry,
                    pinning: Pinning::HashVerified,
                    exec_surface: ExecSurface::BuildScript,
                }),
            };
            c
        }]);
        assert!(dev.admits(&build), "cargo build from a registry");

        // curl | sh — network-sourced from an unverified URL
        let curl_sh = Profile::of(vec![{
            let mut c = cap(Operation::Execute);
            c.execution = Execution {
                trust: ExecutionTrust::NetworkSourced,
                supply_chain: Some(SupplyChain {
                    source: SupplySource::UnverifiedUrl,
                    pinning: Pinning::Floating,
                    exec_surface: ExecSurface::RunArtifact,
                }),
            };
            c
        }]);
        assert!(!dev.admits(&curl_sh), "curl | sh is an unverified-url source");

        // cat file — no downloaded code, so the supply-chain constraint is vacuous
        let plain = Profile::of(vec![cap(Operation::Observe)]);
        assert!(dev.admits(&plain), "a command with no supply chain passes vacuously");
    }

    #[test]
    fn a_supply_chain_deny_does_not_match_a_capability_without_one() {
        // "deny unverified-url sources" must NOT accidentally deny `cat file` (no supply
        // chain) — the vacuous-truth asymmetry between allow and deny (review finding #1).
        let level = Level::new("x").allowing(Clause::default()).denying(Clause {
            supply_source: Some(vec![SupplySource::UnverifiedUrl]),
            ..Default::default()
        });

        let plain = Profile::of(vec![cap(Operation::Observe)]);
        assert!(level.admits(&plain), "a supply-chain deny must not match a no-supply-chain cap");

        // but curl|sh (network-sourced, unverified-url) IS still caught by the deny corner
        let curl_sh = Profile::of(vec![{
            let mut c = cap(Operation::Execute);
            c.execution = Execution {
                trust: ExecutionTrust::NetworkSourced,
                supply_chain: Some(SupplyChain {
                    source: SupplySource::UnverifiedUrl,
                    pinning: Pinning::Floating,
                    exec_surface: ExecSurface::RunArtifact,
                }),
            };
            c
        }]);
        assert!(!level.admits(&curl_sh), "the deny corner still catches the unverified-url source");
    }

    #[test]
    fn extend_inherits_deny_and_adds_allow() {
        let base = Level::new("base")
            .allowing(Clause { operation: Some(vec![Operation::Observe]), ..Default::default() })
            .denying(Clause {
                local_locus: Some(OrdBound::at_least(LocalLocus::Device)),
                ..Default::default()
            });
        let child = Level::extend(
            &base,
            "child",
            vec![Clause {
                operation: Some(vec![Operation::Create, Operation::Mutate]),
                ..Default::default()
            }],
        );

        assert!(child.admits(&Profile::of(vec![cap(Operation::Create)])), "added allow");
        assert!(child.admits(&Profile::of(vec![cap(Operation::Observe)])), "inherited allow");

        let device = Profile::of(vec![{
            let mut c = cap(Operation::Mutate);
            c.locus.local = LocalLocus::Device;
            c
        }]);
        assert!(!child.admits(&device), "inherited deny still bites");
    }

    // ── the algebra contract ──────────────────────────────────────────────────────
    //
    // Generators live in `super::super::testgen` (shared with the authoring tests).
    use crate::engine::testgen::{arb_clause, arb_level, arb_profile};

    proptest! {
        /// Totality: every level yields a decision on every profile, deterministically,
        /// never a panic (the worst-case rule guarantees a total function).
        #[test]
        fn totality(level in arb_level(), profile in arb_profile()) {
            let first = level.admits(&profile);
            prop_assert_eq!(first, level.admits(&profile));
        }

        /// extends ⇒ superset: a level built by `extend` admits everything its base
        /// admits (R27, encoded structurally).
        #[test]
        fn extends_is_a_superset(
            base in arb_level(),
            extra in prop::collection::vec(arb_clause(), 0..3),
            profile in arb_profile(),
        ) {
            let extended = Level::extend(&base, "child", extra);
            prop_assert!(!base.admits(&profile) || extended.admits(&profile));
        }

        /// deny-monotonicity: adding a deny clause can only *shrink* the admitted set —
        /// a deny never grants. This is what makes the subtractive primitive safe.
        #[test]
        fn deny_only_shrinks(
            level in arb_level(),
            extra_deny in arb_clause(),
            profile in arb_profile(),
        ) {
            let stricter = level.clone().denying(extra_deny);
            prop_assert!(!stricter.admits(&profile) || level.admits(&profile));
        }
    }
}

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
    pub scale: Option<OrdBound<Scale>>,
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
    pub execution: Option<OrdBound<ExecutionTrust>>,
    pub cost: Option<OrdBound<Cost>>,
}

impl Clause {
    /// Whether every constrained facet of this clause admits `cap`.
    pub fn admits(&self, cap: &Capability) -> bool {
        set_admits(self.operation.as_deref(), cap.operation)
            && ord_admits(self.local_locus, cap.locus.local)
            && ord_admits(self.remote_reach, cap.locus.remote)
            && set_admits(self.remote_binding.as_deref(), cap.locus.binding)
            && ord_admits(self.scale, cap.scale)
            && ord_admits(self.authority, cap.authority)
            && ord_admits(self.isolation, cap.isolation)
            && ord_admits(self.reversibility, cap.reversibility)
            && ord_admits(self.persistence_level, cap.persistence.level)
            && ord_admits(self.trigger_escape, cap.persistence.trigger.escape)
            && set_admits(self.trigger_kind.as_deref(), cap.persistence.trigger.kind)
            && ord_admits(self.disclosure_audience, cap.disclosure.audience)
            && set_admits(self.disclosure_channel.as_deref(), cap.disclosure.channel)
            && set_admits(self.disclosure_principal.as_deref(), cap.disclosure.principal)
            && ord_admits(self.secret_level, cap.secret.level)
            && set_admits(self.secret_channel.as_deref(), cap.secret.channel)
            && set_admits(self.secret_principal.as_deref(), cap.secret.principal)
            && ord_admits(self.net_direction, cap.network.direction)
            && ord_admits(self.net_destination, cap.network.destination)
            && ord_admits(self.net_payload, cap.network.payload)
            && ord_admits(self.execution, cap.execution)
            && ord_admits(self.cost, cap.cost)
    }
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
    pub fn admits_capability(&self, cap: &Capability) -> bool {
        self.allow.iter().any(|c| c.admits(cap)) && !self.deny.iter().any(|c| c.admits(cap))
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

    // ── generators ──────────────────────────────────────────────────────────────

    fn arb_term<T: FacetTerm + std::fmt::Debug>() -> impl Strategy<Value = T> {
        proptest::sample::select(T::all().to_vec())
    }

    fn arb_bound<T: FacetTerm + Ord + std::fmt::Debug>() -> impl Strategy<Value = OrdBound<T>> {
        (prop::option::of(arb_term::<T>()), prop::option::of(arb_term::<T>())).prop_map(
            |(a, b)| match (a, b) {
                (Some(lo), Some(hi)) if lo > hi => OrdBound { min: Some(hi), max: Some(lo) },
                (min, max) => OrdBound { min, max },
            },
        )
    }

    fn arb_opt_bound<T: FacetTerm + Ord + std::fmt::Debug>()
    -> impl Strategy<Value = Option<OrdBound<T>>> {
        prop::option::of(arb_bound::<T>())
    }

    fn arb_set<T: FacetTerm + std::fmt::Debug>() -> impl Strategy<Value = Vec<T>> {
        proptest::sample::subsequence(T::all().to_vec(), 0..=T::all().len())
    }

    fn arb_opt_set<T: FacetTerm + std::fmt::Debug>() -> impl Strategy<Value = Option<Vec<T>>> {
        prop::option::of(arb_set::<T>())
    }

    prop_compose! {
        fn arb_capability()(
            g1 in (
                arb_term::<Operation>(), arb_term::<LocalLocus>(), arb_term::<RemoteReach>(),
                arb_term::<RemoteBinding>(), arb_term::<Scale>(), arb_term::<Authority>(),
                arb_term::<Isolation>(), arb_term::<Reversibility>(),
            ),
            g2 in (
                arb_term::<PersistenceLevel>(), arb_term::<TriggerEscape>(), arb_term::<TriggerKind>(),
                arb_term::<DisclosureAudience>(), arb_term::<Channel>(), arb_term::<Principal>(),
                arb_term::<SecretLevel>(), arb_term::<Channel>(),
            ),
            g3 in (
                arb_term::<Principal>(), arb_term::<NetDirection>(), arb_term::<NetDestination>(),
                arb_term::<NetPayload>(), arb_term::<ExecutionTrust>(), arb_term::<Cost>(),
            ),
        ) -> Capability {
            Capability {
                operation: g1.0,
                locus: Locus { local: g1.1, remote: g1.2, binding: g1.3 },
                scale: g1.4,
                authority: g1.5,
                isolation: g1.6,
                reversibility: g1.7,
                persistence: Persistence {
                    level: g2.0,
                    trigger: Trigger { escape: g2.1, kind: g2.2 },
                },
                disclosure: Disclosure { audience: g2.3, channel: g2.4, principal: g2.5 },
                secret: Secret { level: g2.6, channel: g2.7, principal: g3.0 },
                network: Network { direction: g3.1, destination: g3.2, payload: g3.3 },
                execution: g3.4,
                cost: g3.5,
                because: String::new(),
            }
        }
    }

    prop_compose! {
        fn arb_clause()(
            g1 in (
                arb_opt_set::<Operation>(), arb_opt_bound::<LocalLocus>(),
                arb_opt_bound::<RemoteReach>(), arb_opt_set::<RemoteBinding>(),
                arb_opt_bound::<Scale>(), arb_opt_bound::<Authority>(),
                arb_opt_bound::<Isolation>(), arb_opt_bound::<Reversibility>(),
            ),
            g2 in (
                arb_opt_bound::<PersistenceLevel>(), arb_opt_bound::<TriggerEscape>(),
                arb_opt_set::<TriggerKind>(), arb_opt_bound::<DisclosureAudience>(),
                arb_opt_set::<Channel>(), arb_opt_set::<Principal>(),
                arb_opt_bound::<SecretLevel>(), arb_opt_set::<Channel>(),
            ),
            g3 in (
                arb_opt_set::<Principal>(), arb_opt_bound::<NetDirection>(),
                arb_opt_bound::<NetDestination>(), arb_opt_bound::<NetPayload>(),
                arb_opt_bound::<ExecutionTrust>(), arb_opt_bound::<Cost>(),
            ),
        ) -> Clause {
            Clause {
                operation: g1.0, local_locus: g1.1, remote_reach: g1.2, remote_binding: g1.3,
                scale: g1.4, authority: g1.5, isolation: g1.6, reversibility: g1.7,
                persistence_level: g2.0, trigger_escape: g2.1, trigger_kind: g2.2,
                disclosure_audience: g2.3, disclosure_channel: g2.4, disclosure_principal: g2.5,
                secret_level: g2.6, secret_channel: g2.7, secret_principal: g3.0,
                net_direction: g3.1, net_destination: g3.2, net_payload: g3.3,
                execution: g3.4, cost: g3.5,
            }
        }
    }

    fn arb_profile() -> impl Strategy<Value = Profile> {
        prop::collection::vec(arb_capability(), 0..4).prop_map(Profile::of)
    }

    fn arb_level() -> impl Strategy<Value = Level> {
        (prop::collection::vec(arb_clause(), 0..3), prop::collection::vec(arb_clause(), 0..2))
            .prop_map(|(allow, deny)| Level { name: "generated".into(), allow, deny })
    }

    // ── the algebra contract ──────────────────────────────────────────────────────

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

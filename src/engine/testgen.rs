//! Shared proptest generators and helpers for the engine's property tests
//! (test-only). Consumed by `level` (the algebra contract) and `authoring` (the
//! facet-monotonicity coherence check on the authored levels).

use proptest::prelude::*;

use super::facet::*;
use super::level::{Clause, Level, OrdBound};

/// A uniform term of an ordinal or categorical facet.
pub(crate) fn arb_term<T: FacetTerm + std::fmt::Debug>() -> impl Strategy<Value = T> {
    proptest::sample::select(T::all().to_vec())
}

fn arb_bound<T: FacetTerm + Ord + std::fmt::Debug>() -> impl Strategy<Value = OrdBound<T>> {
    (prop::option::of(arb_term::<T>()), prop::option::of(arb_term::<T>())).prop_map(|(a, b)| {
        match (a, b) {
            (Some(lo), Some(hi)) if lo > hi => OrdBound { min: Some(hi), max: Some(lo) },
            (min, max) => OrdBound { min, max },
        }
    })
}

fn arb_opt_bound<T: FacetTerm + Ord + std::fmt::Debug>()
-> impl Strategy<Value = Option<OrdBound<T>>> {
    prop::option::of(arb_bound::<T>())
}

fn arb_opt_set<T: FacetTerm + std::fmt::Debug>() -> impl Strategy<Value = Option<Vec<T>>> {
    prop::option::of(proptest::sample::subsequence(T::all().to_vec(), 0..=T::all().len()))
}

prop_compose! {
    pub(crate) fn arb_capability()(
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
            persistence: Persistence { level: g2.0, trigger: Trigger { escape: g2.1, kind: g2.2 } },
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
    pub(crate) fn arb_clause()(
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

pub(crate) fn arb_profile() -> impl Strategy<Value = Profile> {
    prop::collection::vec(arb_capability(), 0..4).prop_map(Profile::of)
}

pub(crate) fn arb_level() -> impl Strategy<Value = Level> {
    (prop::collection::vec(arb_clause(), 0..3), prop::collection::vec(arb_clause(), 0..2))
        .prop_map(|(allow, deny)| Level { name: "generated".into(), allow, deny })
}

/// The next-lower term on an ordinal ladder, or `None` at the zero term.
pub(crate) fn predecessor<T: FacetTerm>(term: T) -> Option<T> {
    let all = T::all();
    let i = all.iter().position(|&x| x == term)?;
    i.checked_sub(1).and_then(|j| all.get(j).copied())
}

/// Every capability obtained by lowering exactly one ordinal facet by one rung.
/// Categorical facets have no severity order and are not lowered.
pub(crate) fn lowered_variants(cap: &Capability) -> Vec<Capability> {
    fn lower<T: FacetTerm>(
        cap: &Capability,
        get: impl Fn(&Capability) -> T,
        set: impl Fn(&mut Capability, T),
    ) -> Option<Capability> {
        let p = predecessor(get(cap))?;
        let mut c = cap.clone();
        set(&mut c, p);
        Some(c)
    }
    [
        lower(cap, |c| c.locus.local, |c, v| c.locus.local = v),
        lower(cap, |c| c.locus.remote, |c, v| c.locus.remote = v),
        lower(cap, |c| c.scale, |c, v| c.scale = v),
        lower(cap, |c| c.authority, |c, v| c.authority = v),
        lower(cap, |c| c.isolation, |c, v| c.isolation = v),
        lower(cap, |c| c.reversibility, |c, v| c.reversibility = v),
        lower(cap, |c| c.persistence.level, |c, v| c.persistence.level = v),
        lower(cap, |c| c.persistence.trigger.escape, |c, v| c.persistence.trigger.escape = v),
        lower(cap, |c| c.disclosure.audience, |c, v| c.disclosure.audience = v),
        lower(cap, |c| c.secret.level, |c, v| c.secret.level = v),
        lower(cap, |c| c.network.direction, |c, v| c.network.direction = v),
        lower(cap, |c| c.network.destination, |c, v| c.network.destination = v),
        lower(cap, |c| c.network.payload, |c, v| c.network.payload = v),
        lower(cap, |c| c.execution, |c, v| c.execution = v),
        lower(cap, |c| c.cost, |c, v| c.cost = v),
    ]
    .into_iter()
    .flatten()
    .collect()
}

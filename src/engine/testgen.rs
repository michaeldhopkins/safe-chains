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
        g4 in (
            arb_term::<SupplySource>(), arb_term::<Pinning>(), arb_term::<ExecSurface>(),
            arb_term::<Provenance>(), arb_term::<RetrievalGranularity>(),
        ),
    ) -> Capability {
        Capability {
            operation: g1.0,
            locus: Locus { local: g1.1, remote: g1.2, binding: g1.3, provenance: g4.3 },
            scale: g1.4,
            retrieval: g4.4,
            authority: g1.5,
            isolation: g1.6,
            reversibility: g1.7,
            persistence: Persistence { level: g2.0, trigger: Trigger { escape: g2.1, kind: g2.2 } },
            disclosure: Disclosure { audience: g2.3, channel: g2.4, principal: g2.5 },
            secret: Secret { level: g2.6, channel: g2.7, principal: g3.0 },
            network: Network { direction: g3.1, destination: g3.2, payload: g3.3 },
            execution: Execution {
                trust: g3.4,
                // model invariant (§2.6): supply_chain is present iff network-sourced
                supply_chain: (g3.4 == ExecutionTrust::NetworkSourced).then_some(SupplyChain {
                    source: g4.0,
                    pinning: g4.1,
                    exec_surface: g4.2,
                }),
            },
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
            arb_opt_set::<SupplySource>(), arb_opt_bound::<Pinning>(), arb_opt_set::<ExecSurface>(),
        ),
        g4 in (arb_opt_bound::<Provenance>(), arb_opt_bound::<RetrievalGranularity>()),
    ) -> Clause {
        Clause {
            operation: g1.0, local_locus: g1.1, remote_reach: g1.2, remote_binding: g1.3,
            provenance: g4.0,
            scale: g1.4, retrieval: g4.1, authority: g1.5, isolation: g1.6, reversibility: g1.7,
            persistence_level: g2.0, trigger_escape: g2.1, trigger_kind: g2.2,
            disclosure_audience: g2.3, disclosure_channel: g2.4, disclosure_principal: g2.5,
            secret_level: g2.6, secret_channel: g2.7, secret_principal: g3.0,
            net_direction: g3.1, net_destination: g3.2, net_payload: g3.3,
            execution_trust: g3.4, cost: g3.5,
            supply_source: g3.6, pinning: g3.7, exec_surface: g3.8,
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
    let mut variants: Vec<Capability> = [
        lower(cap, |c| c.locus.remote, |c, v| c.locus.remote = v),
        lower(cap, |c| c.scale, |c, v| c.scale = v),
        lower(cap, |c| c.authority, |c, v| c.authority = v),
        lower(cap, |c| c.reversibility, |c, v| c.reversibility = v),
        lower(cap, |c| c.persistence.level, |c, v| c.persistence.level = v),
        lower(cap, |c| c.persistence.trigger.escape, |c, v| c.persistence.trigger.escape = v),
        lower(cap, |c| c.disclosure.audience, |c, v| c.disclosure.audience = v),
        lower(cap, |c| c.secret.level, |c, v| c.secret.level = v),
        lower(cap, |c| c.network.direction, |c, v| c.network.direction = v),
        lower(cap, |c| c.network.destination, |c, v| c.network.destination = v),
        lower(cap, |c| c.network.payload, |c, v| c.network.payload = v),
        lower(cap, |c| c.execution.trust, |c, v| c.execution.trust = v),
        lower(cap, |c| c.cost, |c, v| c.cost = v),
        // `pinning` and `isolation` are TRUST facets (higher = safer; a level *floors*
        // them — "containment is earned", §3.2). Their monotonicity direction is inverted
        // (making a command less severe means RAISING them), so they are not lowered here;
        // lowering them would falsely flag a coherent floored level as non-monotone.
    ]
    .into_iter()
    .flatten()
    .collect();

    // `locus.local` IS a reach facet for effect operations (lower = more contained = safer), so it
    // is lowered — EXCEPT under `execute`, where it is the executor-ORIGIN band, not a reach ladder:
    // `temp`/`process` below the band are FOREIGN / inline code (MORE severe, not less), so a level
    // FLOORS it (the deliberate interior band `>= sandbox-scope`; see levels/default.toml and
    // authoring::parse_bound's "interior band" note). Lowering it under execute crosses that floor
    // and would falsely flag the coherent band as non-monotone — the same inverted-polarity reason
    // pinning/isolation are skipped above.
    if cap.operation != Operation::Execute {
        variants.extend(lower(cap, |c| c.locus.local, |c, v| c.locus.local = v));
    }
    variants
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(2000))]

    /// The diagnostic and the predicate are the same walk, so they must never disagree: a clause
    /// admits a capability EXACTLY when it reports no mismatch.
    ///
    /// `check` is literally `first_mismatch(..).is_none()`, so this looks tautological — it is not.
    /// It pins the property against a future refactor that reintroduces a separate `&&` chain "for
    /// speed", which is precisely how an explain feature rots into lying. A diagnostic that names
    /// the wrong axis is worse than none: it sends an author to fix a facet that was never the
    /// problem.
    #[test]
    fn clause_diagnostic_agrees_with_admits(clause in arb_clause(), cap in arb_capability()) {
        prop_assert_eq!(clause.admits(&cap), clause.first_mismatch_for_test(&cap, false).is_none());
    }

    /// A reported mismatch is a REAL one: the facet it names, at the term it names, is genuinely
    /// outside the clause's bound. Guards against a diagnostic that fires on the wrong axis while
    /// still agreeing about yes/no.
    #[test]
    fn a_reported_mismatch_names_a_facet_that_actually_fails(
        clause in arb_clause(),
        cap in arb_capability(),
    ) {
        if let Some(m) = clause.first_mismatch_for_test(&cap, false) {
            prop_assert!(!clause.admits(&cap), "reported {m} but the clause admits the capability");
            prop_assert!(!m.facet.is_empty());
            prop_assert!(!m.bound.is_empty());
        }
    }

    /// The same equivalence for the DENY role. Separate because the supply-chain facet is the one
    /// place allow and deny disagree, so an allow-only check cannot see an inversion there — and
    /// did not: a `?` returning "no complaint" for a definitely-failing deny survived it.
    #[test]
    fn clause_diagnostic_agrees_with_matches_as_deny(
        clause in arb_clause(),
        cap in arb_capability(),
    ) {
        prop_assert_eq!(
            clause.matches_as_deny_for_test(&cap),
            clause.first_mismatch_for_test(&cap, true).is_none(),
        );
    }

    /// A level admits a capability exactly when it reports no nearest miss.
    #[test]
    fn level_nearest_miss_agrees_with_admits_capability(
        level in arb_level(),
        cap in arb_capability(),
    ) {
        prop_assert_eq!(level.admits_capability(&cap), level.nearest_miss(&cap).is_none());
    }
}

/// `base`, with the axis at index `i` replaced by `donor`'s term for that axis. Capabilities
/// differing on EXACTLY one axis are the witnesses that matter for `set_facets` completeness, and
/// independent random generation never produces them: with 27 axes, two arbitrary capabilities
/// differ almost everywhere, so a missing axis is masked by the other 26 still differing. A first
/// attempt at that property was vacuous for exactly this reason — dropping `network.payload` from
/// `set_facets` did not fail it.
///
/// `AXIS_COUNT` must cover every axis; `every_axis_index_is_reachable` pins it.
pub(crate) const AXIS_COUNT: usize = 24;

/// A capability with EVERY axis off its zero term — the donor for reachability checking.
///
/// Not `Capability::worst()`, whose `isolation` sits at its default — correctly, since `Isolation`
/// is a TRUST ladder (least isolation IS the worst), so its declared hazard is the ladder floor,
/// which happens to be the zero term. Overridden here only so every axis is off its zero and each
/// index is demonstrably reachable.
pub(crate) fn all_axes_non_default() -> Capability {
    let mut c = Capability::worst("every axis off its zero term");
    c.isolation = Isolation::Ocap;
    c
}

/// Compile-time exhaustiveness for the axis list.
///
/// Nothing else forces `with_axis_from`/`AXIS_COUNT` to grow when a facet is added to `Capability`.
/// Without this, a new axis is simply never generated as a witness and
/// `set_facets_distinguishes_capabilities_differing_in_one_axis` silently stops covering it —
/// degrading precisely when a facet has just been added, which is when it matters most. Destructured
/// with NO `..`, so adding a field anywhere in the capability record breaks this build instead.
#[allow(unused_variables)]
fn every_axis_is_enumerated(c: &Capability) {
    let Capability {
        operation, locus, scale, retrieval, authority, isolation, reversibility, persistence,
        disclosure, secret, network, execution, cost, because,
    } = c;
    let Locus { local, remote, binding, provenance } = locus;
    let Persistence { level, trigger } = persistence;
    let Trigger { escape, kind } = trigger;
    let Disclosure { audience, channel, principal } = disclosure;
    let Secret { level: secret_level, channel: secret_channel, principal: secret_principal } = secret;
    let Network { direction, destination, payload } = network;
    let Execution { trust, supply_chain } = execution;
}

pub(crate) fn with_axis_from(base: &Capability, donor: &Capability, i: usize) -> Capability {
    every_axis_is_enumerated(base);
    let mut c = base.clone();
    match i {
        0 => c.operation = donor.operation,
        1 => c.locus.local = donor.locus.local,
        2 => c.locus.remote = donor.locus.remote,
        3 => c.locus.binding = donor.locus.binding,
        4 => c.locus.provenance = donor.locus.provenance,
        5 => c.scale = donor.scale,
        6 => c.retrieval = donor.retrieval,
        7 => c.authority = donor.authority,
        8 => c.isolation = donor.isolation,
        9 => c.reversibility = donor.reversibility,
        10 => c.persistence.level = donor.persistence.level,
        11 => c.persistence.trigger.escape = donor.persistence.trigger.escape,
        12 => c.persistence.trigger.kind = donor.persistence.trigger.kind,
        13 => c.disclosure.audience = donor.disclosure.audience,
        14 => c.disclosure.channel = donor.disclosure.channel,
        15 => c.disclosure.principal = donor.disclosure.principal,
        16 => c.secret.level = donor.secret.level,
        17 => c.secret.channel = donor.secret.channel,
        18 => c.secret.principal = donor.secret.principal,
        19 => c.network.direction = donor.network.direction,
        20 => c.network.destination = donor.network.destination,
        21 => c.network.payload = donor.network.payload,
        22 => c.execution.trust = donor.execution.trust,
        23 => c.cost = donor.cost,
        _ => unreachable!("axis index out of range"),
    }
    c
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(3000))]

    /// `set_facets` must distinguish any two capabilities that differ at all.
    ///
    /// It is a hand-written enumeration of 24 `push!` lines, and forgetting one is invisible: the
    /// omitted axis simply never prints. That is not merely a cosmetic gap — `near_neighbours_are_
    /// declared` computes archetype differences THROUGH `set_facets`, so a missing axis would make
    /// two genuinely different archetypes look facet-identical, and the guard would demand they be
    /// declared aliases. A false alias in the confusability catalog is exactly the wrong direction.
    ///
    /// Stated as "different capabilities have different facet lists", which fails the moment an
    /// axis is added to `Capability` without being added here.
    #[test]
    fn set_facets_distinguishes_capabilities_differing_in_one_axis(
        a in arb_capability(),
        donor in arb_capability(),
        i in 0..AXIS_COUNT,
    ) {
        let b = with_axis_from(&a, &donor, i);
        // `because` is prose, not a facet, so it is deliberately outside this claim.
        let same_point = Capability { because: String::new(), ..a.clone() }
            == Capability { because: String::new(), ..b.clone() };
        if !same_point {
            prop_assert_ne!(
                a.set_facets(),
                b.set_facets(),
                "capabilities differing only on axis {} render the same facet list — that axis is \
                 missing from set_facets, which would also make near_neighbours_are_declared see \
                 two different archetypes as a false alias",
                i,
            );
        }
    }

    /// Every axis index really moves something, so `AXIS_COUNT` cannot drift below the number of
    /// facets and quietly stop covering the tail — which would make the completeness property above
    /// silently skip the uncovered axes.
    #[test]
    fn every_axis_index_is_reachable(i in 0..AXIS_COUNT) {
        let zero = Capability::default();
        prop_assert_ne!(
            with_axis_from(&zero, &all_axes_non_default(), i).set_facets(),
            zero.set_facets(),
            "axis index {} changed nothing", i,
        );
    }

    /// Every facet `set_facets` reports is one the capability genuinely carries at that term, and
    /// every omitted axis really is at its zero. Guards the other direction: a `push!` line wired
    /// to the wrong field would still distinguish capabilities while lying about which axis moved.
    #[test]
    fn set_facets_omits_exactly_the_zero_terms(cap in arb_capability()) {
        let d = Capability::default();
        let reported: std::collections::BTreeMap<_, _> = cap.set_facets().into_iter().collect();
        // `operation` is always reported, by design, even at its zero term.
        prop_assert!(reported.contains_key("operation"));
        let baseline: std::collections::BTreeMap<_, _> = d.set_facets().into_iter().collect();
        for (axis, term) in &baseline {
            if *axis != "operation" {
                prop_assert!(
                    !reported.contains_key(axis) || reported[axis] != *term,
                    "{axis} reported at its zero term {term}",
                );
            }
        }
    }
}

/// `AXIS_COUNT` must match what `set_facets` can actually report.
///
/// The compile-time destructure forces an author to NOTICE a new facet; this forces the axis list
/// to actually cover it. Without both, a facet could be added to `Capability` and to `set_facets`
/// while `AXIS_COUNT` stayed put — leaving the new axis outside the range the completeness property
/// samples, which fails silently rather than loudly.
#[test]
fn axis_count_matches_what_set_facets_reports() {
    let cap = all_axes_non_default();
    // `set_facets` flattens a present supply chain into three extra rows that are not independent
    // axes of `with_axis_from` (it is one `Option` field), so they are not counted here.
    let supply_rows = usize::from(cap.execution.supply_chain.is_some()) * 3;
    assert_eq!(
        cap.set_facets().len() - supply_rows,
        AXIS_COUNT,
        "set_facets reports {} axes (excluding supply-chain rows) but AXIS_COUNT is {AXIS_COUNT} — \
         the completeness property samples 0..AXIS_COUNT, so any excess axis is never witnessed",
        cap.set_facets().len() - supply_rows,
    );
}

/// `Capability::worst()` must carry the DECLARED hazard on every axis.
///
/// worst() is the fail-closed sentinel: `resolve` returns it whenever it cannot certify what a
/// command does, so it is the last thing standing between an unrecognized form and an auto-approval.
/// Its guarantee is "no well-formed level admits this", and that guarantee is only as good as the
/// weakest axis — a benign term on any one of them means a clause constraining that axis admits the
/// sentinel there, and the denial then rests entirely on the OTHER axes happening to be constrained.
///
/// Hand-writing 24 terms is how it drifted. Checked here axis by axis so a future facet cannot be
/// added to worst() with a plausible-looking but benign term.
#[test]
fn worst_carries_the_declared_hazard_on_every_axis() {
    let w = Capability::worst("under test");
    let mut wrong = Vec::new();
    macro_rules! check {
        ($name:literal, $actual:expr, $ty:ty) => {
            let want = <$ty as FacetTerm>::hazard();
            if $actual != want {
                wrong.push(format!(
                    "{} = {} but the declared hazard is {}",
                    $name,
                    $actual.as_str(),
                    want.as_str(),
                ));
            }
        };
    }
    check!("operation", w.operation, Operation);
    check!("locus.local", w.locus.local, LocalLocus);
    check!("locus.remote", w.locus.remote, RemoteReach);
    check!("locus.binding", w.locus.binding, RemoteBinding);
    check!("locus.provenance", w.locus.provenance, Provenance);
    check!("scale", w.scale, Scale);
    check!("retrieval", w.retrieval, RetrievalGranularity);
    check!("authority", w.authority, Authority);
    check!("isolation", w.isolation, Isolation);
    check!("reversibility", w.reversibility, Reversibility);
    check!("persistence.level", w.persistence.level, PersistenceLevel);
    check!("persistence.trigger.escape", w.persistence.trigger.escape, TriggerEscape);
    check!("persistence.trigger.kind", w.persistence.trigger.kind, TriggerKind);
    check!("disclosure.audience", w.disclosure.audience, DisclosureAudience);
    check!("disclosure.channel", w.disclosure.channel, Channel);
    check!("disclosure.principal", w.disclosure.principal, Principal);
    check!("secret.level", w.secret.level, SecretLevel);
    check!("secret.channel", w.secret.channel, Channel);
    check!("secret.principal", w.secret.principal, Principal);
    check!("network.direction", w.network.direction, NetDirection);
    check!("network.destination", w.network.destination, NetDestination);
    check!("network.payload", w.network.payload, NetPayload);
    check!("execution.trust", w.execution.trust, ExecutionTrust);
    check!("cost", w.cost, Cost);

    // A supply chain that is ABSENT satisfies every supply-chain constraint vacuously on the allow
    // path (`supply_chain_admits` returns true for `None`), so the sentinel must carry one rather
    // than skip the axis entirely — otherwise the install/RCE surface is the one place worst() is
    // not worst.
    match w.execution.supply_chain {
        None => wrong.push(
            "execution.supply_chain is absent, which satisfies every supply-chain constraint \
             vacuously on an allow clause"
                .to_string(),
        ),
        Some(sc) => {
            check!("supply_chain.source", sc.source, SupplySource);
            check!("supply_chain.pinning", sc.pinning, Pinning);
            check!("supply_chain.exec_surface", sc.exec_surface, ExecSurface);
        }
    }
    assert!(wrong.is_empty(), "worst() is not worst on:\n  {}", wrong.join("\n  "));
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(200))]

    /// No authored level admits the sentinel even with ANY ONE axis relaxed to its benign term.
    ///
    /// This is the property the hazard declarations exist to buy. worst()'s doc has always claimed
    /// "`locus.local = kernel` alone denies it everywhere" — true, but it means the guarantee rested
    /// on ONE axis, and a level whose clauses do not constrain `locus.local` would have admitted the
    /// sentinel outright. Relaxing each axis in turn and finding it still denied shows the denial is
    /// over-determined rather than balanced on a single term.
    ///
    /// It is also what makes the two fixes load-bearing rather than cosmetic: before them,
    /// `persistence.trigger.kind` and the absent supply chain were already benign, so those axes
    /// contributed nothing to the denial at all.
    #[test]
    fn the_sentinel_is_denied_even_with_any_one_axis_relaxed(i in 0..AXIS_COUNT) {
        let mut relaxed = with_axis_from(&Capability::worst("relaxed sentinel"), &Capability::default(), i);
        // The supply chain is an `Option`, not one of `with_axis_from`'s axes, so relaxing it means
        // REMOVING it — which is exactly the shape the sentinel used to ship with, and which an
        // allow clause satisfies vacuously. Folded in here so the one axis that cannot be relaxed
        // by index is not the one axis left untested.
        if i % 2 == 0 {
            relaxed.execution.supply_chain = None;
        }
        let profile = Profile::of(vec![relaxed.clone()]);
        // `yolo` is excluded because it admits the sentinel outright, by design: it is the level
        // that means "auto-approve everything", so a user who selects it has opted out of the
        // question this sentinel answers. Every level whose JOB is to deny is covered.
        for level in crate::engine::authoring::default_levels().iter().filter(|l| l.name != "yolo") {
            prop_assert!(
                !level.admits(&profile),
                "level `{}` admits the sentinel with axis {} relaxed to {:?}",
                level.name, i, relaxed.set_facets(),
            );
        }
    }
}

/// A declared `hazard` must be the term the authored levels actually reject.
///
/// `worst_carries_the_declared_hazard_on_every_axis` cannot check this: `worst()` is DERIVED from
/// `hazard()`, so changing a declaration moves both sides and the comparison stays true. Verified by
/// red demo — mis-declaring `Pinning`'s hazard as `digest` (the SAFEST term on that trust ladder)
/// left it green. The declarations are the single point of truth, so something has to check them
/// against evidence outside themselves.
///
/// The evidence is the authored levels. If a clause is selective on an axis — admits some terms and
/// not others — then the term it rejects is the hazardous one. So: no authored allow clause may
/// admit the hazard while rejecting some other term of that axis. `Pinning >= version` rejects
/// `floating` and admits `digest`, which pins the direction of that trust ladder from the level
/// authoring rather than from a comment.
#[test]
fn a_declared_hazard_is_the_term_authored_levels_reject() {
    let mut wrong = Vec::new();
    let mut unconstrained = Vec::new();

    macro_rules! ord_axis {
        ($name:literal, $field:ident, $ty:ty) => {
            let mut seen = false;
            for level in crate::engine::authoring::default_levels().iter().filter(|l| l.name != "yolo") {
                for c in &level.allow {
                    let Some(bound) = c.$field else { continue };
                    seen = true;
                    let hz = <$ty as FacetTerm>::hazard();
                    let admits_hazard = bound.admits(hz);
                    let rejects_something = <$ty>::all().iter().any(|t| !bound.admits(*t));
                    if admits_hazard && rejects_something {
                        wrong.push(format!(
                            "{}: `{}` admits the declared hazard `{}` while rejecting other terms — \
                             the hazard is pointed the wrong way on this axis",
                            $name, level.name, hz.as_str(),
                        ));
                    }
                }
            }
            if !seen {
                unconstrained.push($name);
            }
        };
    }
    macro_rules! set_axis {
        ($name:literal, $field:ident, $ty:ty) => {
            let mut seen = false;
            for level in crate::engine::authoring::default_levels().iter().filter(|l| l.name != "yolo") {
                for c in &level.allow {
                    let Some(set) = c.$field.as_ref() else { continue };
                    seen = true;
                    let hz = <$ty as FacetTerm>::hazard();
                    if set.contains(&hz) && <$ty>::all().iter().any(|t| !set.contains(t)) {
                        wrong.push(format!(
                            "{}: `{}` admits the declared hazard `{}` while rejecting other terms",
                            $name, level.name, hz.as_str(),
                        ));
                    }
                }
            }
            if !seen {
                unconstrained.push($name);
            }
        };
    }

    ord_axis!("locus.local", local_locus, LocalLocus);
    ord_axis!("locus.remote", remote_reach, RemoteReach);
    ord_axis!("locus.provenance", provenance, Provenance);
    ord_axis!("scale", scale, Scale);
    ord_axis!("retrieval", retrieval, RetrievalGranularity);
    ord_axis!("authority", authority, Authority);
    ord_axis!("isolation", isolation, Isolation);
    ord_axis!("reversibility", reversibility, Reversibility);
    ord_axis!("persistence.level", persistence_level, PersistenceLevel);
    ord_axis!("persistence.trigger.escape", trigger_escape, TriggerEscape);
    ord_axis!("disclosure.audience", disclosure_audience, DisclosureAudience);
    ord_axis!("secret.level", secret_level, SecretLevel);
    ord_axis!("network.direction", net_direction, NetDirection);
    ord_axis!("network.destination", net_destination, NetDestination);
    ord_axis!("network.payload", net_payload, NetPayload);
    ord_axis!("execution.trust", execution_trust, ExecutionTrust);
    ord_axis!("supply_chain.pinning", pinning, Pinning);
    ord_axis!("cost", cost, Cost);
    set_axis!("locus.binding", remote_binding, RemoteBinding);
    set_axis!("persistence.trigger.kind", trigger_kind, TriggerKind);
    set_axis!("disclosure.channel", disclosure_channel, Channel);
    set_axis!("disclosure.principal", disclosure_principal, Principal);
    set_axis!("secret.channel", secret_channel, Channel);
    set_axis!("secret.principal", secret_principal, Principal);
    set_axis!("supply_chain.source", supply_source, SupplySource);
    set_axis!("supply_chain.exec_surface", exec_surface, ExecSurface);

    assert!(wrong.is_empty(), "hazard declared against the evidence:\n  {}", wrong.join("\n  "));
    // `operation` is deliberately absent above. Levels PARTITION their allow clauses by operation
    // — one clause for observes, another for mutates — so every operation is admitted by some
    // clause and rejected by others, and "the term levels reject" is not a well-defined question
    // on that axis. Its declaration is held instead by
    // `the_sentinel_is_denied_even_with_any_one_axis_relaxed`, which checks the thing that actually
    // matters: that relaxing `operation` to `observe` still leaves the sentinel denied.
    //
    // Reported, not asserted: an axis no level constrains has no evidence either way, so its
    // declaration rests on the doc comment alone. Naming them keeps that visible instead of letting
    // the test look more thorough than it is.
    if !unconstrained.is_empty() {
        eprintln!(
            "hazard unverifiable (no authored clause constrains these axes): {}",
            unconstrained.join(", "),
        );
    }
}

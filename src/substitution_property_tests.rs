use crate::command_verdict_in;
use crate::handler_property_tests::{
    ABSTRACTION_CONTEXTS, ABSTRACTION_ROOTS, ABSTRACTION_SUBS, abstraction_suffixes, abstraction_witnesses, workspace,
};

/// A substitution must never be MORE PERMISSIVE than a concrete path it could produce.
///
/// The general form of every substitution fail-open found so far. It needs no hand-written
/// expectation: the literal spelling — already correct and heavily tested — IS the expectation, so
/// a laundering bug shows up without anyone having imagined that particular route to it.
///
/// EXHAUSTIVE rather than sampled, deliberately. As a `proptest` over the same space this passed
/// against a reintroduced pathgate bug: the space is ~10k combinations, proptest draws 256, and the
/// one pairing that mattered (`asciidoctor -o` with an `/etc` root) simply never came up. A finite
/// cross-product this cheap should be enumerated — it also makes the red demos deterministic.
///
/// Red→green, all four historical bugs, each caught with its corpus entries removed: the shape
/// test on roots, the pathgate `is_unpinnable` key, the pre-expansion tag read, and the
/// unclassified residue.
///
/// Each context is checked on its own thread, and within one an abstraction is classified once per
/// root and suffix rather than once per refused witness, since its verdict does not depend on the
/// witness. Run serially and per witness, this one test was 143s of a 149s suite and made every
/// mutation run outgrow its job.
#[test]
fn substitution_is_never_more_permissive_than_a_path_it_could_produce() {
    let suffixes = abstraction_suffixes();
    let per_context: Vec<(usize, Vec<String>)> = std::thread::scope(|scope| {
        let handles: Vec<_> = ABSTRACTION_CONTEXTS.iter().map(|ctx| scope.spawn(|| check_context(ctx, &suffixes))).collect();
        handles.into_iter().map(|h| h.join().expect("a context's check panicked")).collect()
    });
    let constrained: usize = per_context.iter().map(|(n, _)| n).sum();
    let violations: Vec<String> = per_context.into_iter().flat_map(|(_, v)| v).collect();

    assert!(constrained > 0, "no witness was refused — the property is vacuous");
    assert!(
        violations.is_empty(),
        "a substitution was more permissive than a path it could produce ({} cases):\n{}",
        violations.len(),
        violations.join("\n"),
    );
}

fn check_context(ctx: &str, suffixes: &[String]) -> (usize, Vec<String>) {
    let mut violations = Vec::new();
    let mut constrained = 0usize;
    for root in ABSTRACTION_ROOTS {
        for suffix in suffixes {
            // Only a witness the classifier REFUSES constrains the abstraction; an allowed one
            // says nothing, since a different witness may still force the denial.
            let refused: Vec<String> = abstraction_witnesses(root)
                .into_iter()
                .map(|witness| ctx.replace("{}", &format!("{witness}{suffix}")))
                .filter(|concrete| !command_verdict_in(concrete, workspace()).is_allowed())
                .collect();
            if refused.is_empty() {
                continue;
            }
            constrained += refused.len();
            let allowed: Vec<String> = ABSTRACTION_SUBS
                .iter()
                .map(|sub| ctx.replace("{}", &format!("{}{suffix}", sub.replace("{root}", root))))
                .filter(|abstracted| command_verdict_in(abstracted, workspace()).is_allowed())
                .collect();
            for concrete in &refused {
                for abstracted in &allowed {
                    violations.push(format!("  `{concrete}` denies but `{abstracted}` allows"));
                }
            }
        }
    }
    (constrained, violations)
}

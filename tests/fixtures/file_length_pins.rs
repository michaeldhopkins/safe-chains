// The pin list for `tests/file_length.rs`, generated from the tree as it stood at `cf54a536`, the
// parent of the commit that introduced the gate. Held in its own file so the gate's logic and the
// backlog it is working through stay separate to read — and so that shrinking a file touches this
// list and nothing else.
//
// Every entry is a file that was already over its limit. None may grow; each must shrink, and when
// it does, its number comes down here in the same change. An entry disappears when its file gets
// under the limit. Nothing is ever ADDED here: a file that outgrows its limit after this point is
// split, and new code goes in a new module rather than into a file on this list.
const PINS: [(&str, usize); 18] = [
    ("src/cst/check.rs", 895),
    ("src/cst/parse.rs", 1063),
    ("src/cst/proptests.rs", 759),
    ("src/engine/facet.rs", 685),
    ("src/engine/resolve.rs", 1500),
    ("src/engine/resolve/locus.rs", 584),
    ("src/engine/resolve/regions.rs", 928),
    ("src/engine/testgen.rs", 657),
    ("src/handler_property_tests.rs", 2386),
    ("src/lib.rs", 443),
    ("src/pathgate.rs", 522),
    ("src/registry/build.rs", 1363),
    ("src/registry/dispatch.rs", 544),
    ("src/registry/mod.rs", 659),
    ("src/registry/tests.rs", 7324),
    ("src/registry/types.rs", 1050),
    ("src/tests.rs", 2107),
    ("tests/integration_hooks.rs", 756),
];

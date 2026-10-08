// The pin list for `tests/file_length.rs`, generated from the tree as it stood at `cf54a536`, the
// parent of the commit that introduced the gate. Held in its own file so the gate's logic and the
// backlog it is working through stay separate to read — and so that shrinking a file touches this
// list and nothing else.
//
// Every entry is a file that was already over its limit. None may grow; each must shrink, and when
// it does, its number comes down here in the same change. An entry disappears when its file gets
// under the limit. Nothing is ever ADDED here: a file that outgrows its limit after this point is
// split, and new code goes in a new module rather than into a file on this list.
const PINS: [(&str, usize); 19] = [
    ("src/cst/check.rs", 1072),
    ("src/cst/parse.rs", 1182),
    ("src/cst/proptests.rs", 759),
    ("src/engine/facet.rs", 685),
    ("src/engine/resolve.rs", 1622),
    ("src/engine/resolve/locus.rs", 600),
    ("src/engine/resolve/regions.rs", 929),
    ("src/engine/testgen.rs", 657),
    ("src/handler_property_tests.rs", 2439),
    ("src/handlers/perl.rs", 454),
    ("src/lib.rs", 612),
    ("src/pathgate.rs", 932),
    ("src/registry/build.rs", 1532),
    ("src/registry/dispatch.rs", 572),
    ("src/registry/mod.rs", 733),
    ("src/registry/tests.rs", 7324),
    ("src/registry/types.rs", 1162),
    ("src/tests.rs", 2119),
    ("tests/integration_hooks.rs", 756),
];

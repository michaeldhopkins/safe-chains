// The pin list for `tests/file_length.rs`, generated from the tree as it stood at `cf54a536`, the
// parent of the commit that introduced the gate. Held in its own file so the gate's logic and the
// backlog it is working through stay separate to read — and so that shrinking a file touches this
// list and nothing else.
//
// Every entry is a file that was already over its limit. None may grow; each must shrink, and when
// it does, its number comes down here in the same change. An entry disappears when its file gets
// under the limit. Nothing is ever ADDED here: a file that outgrows its limit after this point is
// split, and new code goes in a new module rather than into a file on this list.
const PINS: [(&str, usize); 21] = [
    ("src/cst/check.rs", 1103),
    ("src/cst/parse.rs", 1304),
    ("src/cst/proptests.rs", 801),
    ("src/engine/facet.rs", 697),
    ("src/engine/resolve.rs", 1661),
    ("src/engine/resolve/locus.rs", 617),
    ("src/engine/resolve/regions.rs", 965),
    ("src/engine/testgen.rs", 658),
    ("src/handler_property_tests.rs", 2616),
    ("src/handlers/perl.rs", 476),
    ("src/lib.rs", 645),
    ("src/pathctx.rs", 430),
    ("src/pathgate.rs", 969),
    ("src/registry/build.rs", 1638),
    ("src/registry/dispatch.rs", 620),
    ("src/registry/docs.rs", 420),
    ("src/registry/mod.rs", 759),
    ("src/registry/tests.rs", 7774),
    ("src/registry/types.rs", 1162),
    ("src/tests.rs", 2139),
    ("tests/integration_hooks.rs", 851),
];

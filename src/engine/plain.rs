//! Plain words for what `--explain` prints in facet terms.
//!
//! `--explain` shows the behaviour a command resolved to as facet terms (`locus.local = machine`),
//! which mean nothing to someone who has not read the taxonomy. This is the dictionary of short,
//! everyday phrases that replace them: one for every operation in every local place (the line that
//! says what the command does), and one for every term of every other axis.
//!
//! The tests derive the vocabulary from the code (`FacetTerm::all()` per axis, and the axis names
//! `Capability::set_facets` prints), so a new axis or term fails until it has a phrase.

use super::facet::*;

/// What a command does in a place: `(operation, locus.local, phrase)`.
pub const ACTIONS: &[(&str, &str, &str)] = &[
    ("observe", "process", "looks only at its own input"),
    ("observe", "temp", "reads a file in a temporary folder"),
    ("observe", "sandbox-scope", "reads a file in this session's scratch folder"),
    ("observe", "worktree", "reads files in your project"),
    ("observe", "adjacent", "reads files in a project next to this one"),
    ("observe", "worktree-trusted", "reads your project's .git, hooks or CI settings"),
    ("observe", "user", "reads a file in your home folder"),
    ("observe", "machine", "reads files in a place safe-chains has no rule for"),
    ("observe", "system-integrity", "reads the files that control login and startup"),
    ("observe", "device", "reads a disk or device directly"),
    ("observe", "kernel", "reads the operating system's core"),
    ("create", "process", "makes something that lasts only while it runs"),
    ("create", "temp", "creates a file in a temporary folder"),
    ("create", "sandbox-scope", "creates a file in this session's scratch folder"),
    ("create", "worktree", "creates files in your project"),
    ("create", "adjacent", "creates files in a project next to this one"),
    ("create", "worktree-trusted", "adds to your project's .git, hooks or CI settings"),
    ("create", "user", "creates a file in your home folder"),
    ("create", "machine", "creates files in a place safe-chains has no rule for"),
    ("create", "system-integrity", "adds to the files that control login and startup"),
    ("create", "device", "creates a device"),
    ("create", "kernel", "adds to the operating system's core"),
    ("mutate", "process", "changes only its own running state"),
    ("mutate", "temp", "changes a file in a temporary folder"),
    ("mutate", "sandbox-scope", "changes a file in this session's scratch folder"),
    ("mutate", "worktree", "changes files in your project"),
    ("mutate", "adjacent", "changes files in a project next to this one"),
    ("mutate", "worktree-trusted", "changes your project's .git, hooks or CI settings"),
    ("mutate", "user", "changes a file in your home folder"),
    ("mutate", "machine", "changes files in a place safe-chains has no rule for"),
    ("mutate", "system-integrity", "changes the files that control login and startup"),
    ("mutate", "device", "writes to a disk or device directly"),
    ("mutate", "kernel", "changes the operating system's core"),
    ("destroy", "process", "removes only what it made while running"),
    ("destroy", "temp", "deletes files in a temporary folder"),
    ("destroy", "sandbox-scope", "deletes files in this session's scratch folder"),
    ("destroy", "worktree", "deletes files in your project"),
    ("destroy", "adjacent", "deletes files in a project next to this one"),
    ("destroy", "worktree-trusted", "deletes from your project's .git, hooks or CI settings"),
    ("destroy", "user", "deletes files in your home folder"),
    ("destroy", "machine", "deletes files in a place safe-chains has no rule for"),
    ("destroy", "system-integrity", "deletes files that control login and startup"),
    ("destroy", "device", "erases a disk or device"),
    ("destroy", "kernel", "removes part of the operating system's core"),
    ("execute", "process", "runs code that stays inside the command"),
    ("execute", "temp", "runs code from a temporary folder"),
    ("execute", "sandbox-scope", "runs code from this session's scratch folder"),
    ("execute", "worktree", "runs your project's code"),
    ("execute", "adjacent", "runs code from a project next to this one"),
    ("execute", "worktree-trusted", "runs your project's hooks"),
    ("execute", "user", "runs code from your home folder"),
    ("execute", "machine", "runs code from a place safe-chains has no rule for"),
    ("execute", "system-integrity", "runs system programs in a way that can change them"),
    ("execute", "device", "runs code straight from a device"),
    ("execute", "kernel", "loads code into the operating system's core"),
    ("communicate", "process", "talks to another system without touching your files"),
    ("communicate", "temp", "sends or fetches data using a temporary folder"),
    ("communicate", "sandbox-scope", "sends or fetches data using this session's scratch folder"),
    ("communicate", "worktree", "sends or fetches data for your project"),
    ("communicate", "adjacent", "sends or fetches data for a project next to this one"),
    ("communicate", "worktree-trusted", "sends or fetches data using your project's .git settings"),
    ("communicate", "user", "sends or fetches data using files in your home folder"),
    ("communicate", "machine", "sends or fetches data using files in a place safe-chains has no rule for"),
    ("communicate", "system-integrity", "sends or fetches data using login or startup files"),
    ("communicate", "device", "talks to a device directly"),
    ("communicate", "kernel", "talks to the operating system's core"),
    ("configure", "process", "changes settings only for itself"),
    ("configure", "temp", "changes settings kept in a temporary folder"),
    ("configure", "sandbox-scope", "changes settings kept in this session's scratch folder"),
    ("configure", "worktree", "changes your project's settings"),
    ("configure", "adjacent", "changes the settings of a project next to this one"),
    ("configure", "worktree-trusted", "changes your project's .git, hooks or CI settings"),
    ("configure", "user", "changes your personal settings"),
    ("configure", "machine", "changes this computer's settings"),
    ("configure", "system-integrity", "changes how this computer logs in or starts up"),
    ("configure", "device", "changes a device's settings"),
    ("configure", "kernel", "changes the operating system's core settings"),
    ("authorize", "process", "changes permissions only for itself"),
    ("authorize", "temp", "changes permissions on a temporary folder"),
    ("authorize", "sandbox-scope", "changes permissions on this session's scratch folder"),
    ("authorize", "worktree", "changes permissions in your project"),
    ("authorize", "adjacent", "changes permissions in a project next to this one"),
    ("authorize", "worktree-trusted", "changes what your project's .git or CI trusts"),
    ("authorize", "user", "changes your keys, passwords or trusted accounts"),
    ("authorize", "machine", "changes what this computer trusts"),
    ("authorize", "system-integrity", "changes who may log in or use admin rights"),
    ("authorize", "device", "changes who may use a device"),
    ("authorize", "kernel", "changes what the operating system's core trusts"),
    ("control", "process", "signals only its own processes"),
    ("control", "temp", "starts or stops a program from a temporary folder"),
    ("control", "sandbox-scope", "starts or stops a program from this session's scratch folder"),
    ("control", "worktree", "starts or stops your project's programs"),
    ("control", "adjacent", "starts or stops programs from a project next to this one"),
    ("control", "worktree-trusted", "starts or stops your project's hooks or services"),
    ("control", "user", "starts or stops programs running as you"),
    ("control", "machine", "starts or stops services on this computer"),
    ("control", "system-integrity", "starts, stops or restarts the system itself"),
    ("control", "device", "starts, stops or resets a device"),
    ("control", "kernel", "loads or unloads parts of the operating system's core"),
];

/// Every other axis: `(axis, term, phrase)`. `operation` and `locus.local` are here too, for a
/// line that names one without the other (a refusal saying which places a level allows).
pub const TERMS: &[(&str, &str, &str)] = &[
    ("operation", "observe", "reads or looks"),
    ("operation", "create", "creates"),
    ("operation", "mutate", "changes"),
    ("operation", "destroy", "deletes"),
    ("operation", "execute", "runs code"),
    ("operation", "communicate", "talks to another system"),
    ("operation", "configure", "changes settings that affect later commands"),
    ("operation", "authorize", "changes what is trusted"),
    ("operation", "control", "starts, stops or signals programs"),
    ("locus.local", "process", "nowhere outside the command"),
    ("locus.local", "temp", "a temporary folder"),
    ("locus.local", "sandbox-scope", "this session's scratch folder"),
    ("locus.local", "worktree", "your project"),
    ("locus.local", "adjacent", "a project next to this one"),
    ("locus.local", "worktree-trusted", "your project's .git, hooks or CI settings"),
    ("locus.local", "user", "your home folder"),
    ("locus.local", "machine", "a place safe-chains has no rule for"),
    ("locus.local", "system-integrity", "the files that control login and startup"),
    ("locus.local", "device", "a disk or device directly"),
    ("locus.local", "kernel", "the operating system's core"),
    ("locus.remote", "none", "no other computer"),
    ("locus.remote", "fixed", "one other computer or service, named in the command"),
    ("locus.remote", "arbitrary", "any computer or service the command chooses"),
    ("locus.binding", "n/a", "does not apply"),
    ("locus.binding", "pinned", "the account or host is named in the command"),
    ("locus.binding", "ambient", "uses whichever account or host is set up already"),
    ("locus.provenance", "n/a", "does not apply"),
    ("locus.provenance", "established", "a remote you set up before"),
    ("locus.provenance", "literal", "an address typed in the command"),
    ("locus.provenance", "opaque", "an address that comes from a variable, so it cannot be seen"),
    ("scale", "single", "one thing"),
    ("scale", "bounded", "a set of things it names"),
    ("scale", "unbounded", "everything it finds, without limit"),
    ("retrieval", "metadata", "only names and details"),
    ("retrieval", "record", "data records"),
    ("retrieval", "bulk-content", "whole files or stored data"),
    ("authority", "user", "as you"),
    ("authority", "elevated", "with admin rights (sudo)"),
    ("authority", "root", "as the system administrator"),
    ("authority", "other-user", "as another user"),
    ("isolation", "none", "with no sandbox"),
    ("isolation", "view", "in a restricted view of the files"),
    ("isolation", "namespace", "in a container"),
    ("isolation", "userns", "in a container with its own users"),
    ("isolation", "vm", "in a virtual machine"),
    ("isolation", "ocap", "with only the access it is handed"),
    ("reversibility", "none", "nothing to undo"),
    ("reversibility", "trivial", "one command undoes it"),
    (
        "reversibility",
        "recoverable",
        "can usually be undone from version control, the trash or a snapshot, or by running the opposite command",
    ),
    ("reversibility", "effortful", "hard to undo; it may take a backup, a rebuild or a reverse operation"),
    ("reversibility", "irreversible", "cannot be undone"),
    ("persistence.level", "transient", "leaves nothing behind"),
    ("persistence.level", "data", "leaves data behind"),
    ("persistence.level", "reconfiguring", "changes how later commands behave"),
    ("persistence.level", "installing", "installs programs, services or hooks"),
    ("persistence.trigger.escape", "immediate", "finishes when the command does"),
    ("persistence.trigger.escape", "detached", "keeps running after the session ends"),
    ("persistence.trigger.escape", "recurring", "runs again on a schedule or an event"),
    ("persistence.trigger.escape", "boot", "runs again every time the computer starts"),
    ("persistence.trigger.kind", "none", "does not repeat"),
    ("persistence.trigger.kind", "clock", "repeats on a schedule"),
    ("persistence.trigger.kind", "event", "repeats when something happens"),
    ("disclosure.audience", "none", "shows nothing"),
    ("disclosure.audience", "local-process", "shows its output to the agent"),
    ("disclosure.audience", "local-persistent", "saves output where other people on this computer can read it"),
    ("disclosure.audience", "trusted-remote", "sends output to a service you trust"),
    ("disclosure.audience", "shared-remote", "sends output to a shared service"),
    ("disclosure.audience", "public", "makes output public"),
    ("disclosure.channel", "none", "through no channel"),
    ("disclosure.channel", "filesystem", "through a file"),
    ("disclosure.channel", "stdout-to-model", "to the agent's screen"),
    ("disclosure.channel", "network", "over the network"),
    ("disclosure.channel", "clipboard", "through the clipboard"),
    ("disclosure.channel", "ipc", "to another running program"),
    ("disclosure.channel", "credential-store", "through your keychain or password store"),
    ("disclosure.channel", "cross-process", "by reading another program's memory"),
    ("disclosure.channel", "unknown", "by a route safe-chains cannot tell"),
    ("disclosure.principal", "own", "your own data"),
    ("disclosure.principal", "cross", "someone else's data"),
    ("secret.level", "none", "touches no passwords or keys"),
    ("secret.level", "uses-ambient", "uses sign-ins you already set up, without reading them"),
    ("secret.level", "reads", "reads a password, key or token"),
    ("secret.level", "writes", "stores a password, key or token"),
    ("secret.level", "transmits", "sends a password, key or token somewhere"),
    ("secret.channel", "none", "through no channel"),
    ("secret.channel", "filesystem", "from a file"),
    ("secret.channel", "stdout-to-model", "onto the agent's screen"),
    ("secret.channel", "network", "over the network"),
    ("secret.channel", "clipboard", "through the clipboard"),
    ("secret.channel", "ipc", "from another running program"),
    ("secret.channel", "credential-store", "from your keychain or password store"),
    ("secret.channel", "cross-process", "from another program's memory"),
    ("secret.channel", "unknown", "by a route safe-chains cannot tell"),
    ("secret.principal", "own", "your own"),
    ("secret.principal", "cross", "someone else's"),
    ("network.direction", "none", "no network"),
    ("network.direction", "loopback", "only to this computer"),
    ("network.direction", "outbound", "connects out to the internet"),
    ("network.direction", "inbound-listen", "listens for connections from outside"),
    ("network.destination", "n/a", "no network"),
    ("network.destination", "fixed", "to a destination named in the command"),
    ("network.destination", "arbitrary", "to any destination"),
    ("network.payload", "none", "sends nothing"),
    ("network.payload", "fetches", "downloads data"),
    ("network.payload", "sends-host-data", "sends data from this computer"),
    ("execution.trust", "none", "runs no code"),
    ("execution.trust", "self", "runs only its own code"),
    ("execution.trust", "caller-inline", "runs code written in the command"),
    ("execution.trust", "caller-file", "runs a script you point it at"),
    ("execution.trust", "ambient-config", "runs code from files it finds, such as a Makefile or hooks"),
    ("execution.trust", "network-sourced", "runs code downloaded from the internet"),
    ("cost", "none", "costs nothing"),
    ("cost", "local-resource", "uses a lot of this computer's resources"),
    ("cost", "metered", "can cost money"),
    ("cost", "quota", "uses up a quota"),
    ("supply_chain.source", "unverified-url", "from an unchecked web address"),
    ("supply_chain.source", "public-registry", "from a public package registry"),
    ("supply_chain.source", "signed-repo", "from a signed repository"),
    ("supply_chain.source", "private-registry", "from a private registry"),
    ("supply_chain.source", "vendored", "from a copy kept in your project"),
    ("supply_chain.pinning", "floating", "whatever the latest version is"),
    ("supply_chain.pinning", "version", "a named version"),
    ("supply_chain.pinning", "hash-verified", "a version checked against a hash"),
    ("supply_chain.pinning", "digest", "one exact build, checked by its digest"),
    ("supply_chain.exec_surface", "none", "runs nothing it downloads"),
    ("supply_chain.exec_surface", "install-hook", "runs downloaded code while installing"),
    ("supply_chain.exec_surface", "build-script", "runs downloaded code while building"),
    ("supply_chain.exec_surface", "call-time", "runs downloaded code when your program runs"),
    ("supply_chain.exec_surface", "run-artifact", "runs the downloaded program itself"),
];

/// An empty phrase is a row whose wording awaits the owner's approval: `--explain` keeps printing
/// today's term for it. Approving the wording is replacing the empty string, and removing the row
/// from `HELD` in the tests.
pub fn is_held(phrase: &str) -> bool {
    phrase.is_empty()
}

/// The phrase for what a command does: an operation in a local place.
pub fn action(operation: &str, local: &str) -> Option<&'static str> {
    ACTIONS.iter().find(|(o, l, _)| *o == operation && *l == local).map(|(_, _, p)| *p)
}

/// The phrase for one term of one axis.
pub fn term(axis: &str, value: &str) -> Option<&'static str> {
    TERMS.iter().find(|(a, t, _)| *a == axis && *t == value).map(|(_, _, p)| *p)
}

/// The phrase for a term whose wording is approved; `None` while it is held or unknown.
pub fn spoken(axis: &str, value: &str) -> Option<&'static str> {
    term(axis, value).filter(|p| !is_held(p))
}

/// What to print for a term: its phrase, or the term itself while its wording is held or unknown.
pub fn words<'a>(axis: &str, value: &'a str) -> &'a str {
    match term(axis, value) {
        Some(p) if !is_held(p) => p,
        _ => value,
    }
}

/// The rows `--explain` prints for one capability's facets, in order. The operation and the local
/// place become one row of what the command does; while that wording is held the two rows stay as
/// they were. Every other axis keeps its name and prints its phrase.
pub fn rows(facets: &[(&'static str, &'static str)]) -> Vec<(String, String)> {
    let find = |axis: &str| facets.iter().find(|(a, _)| *a == axis).map(|(_, t)| *t);
    let operation = find("operation");
    let place = find("locus.local").unwrap_or("process");
    let does = operation.and_then(|o| action(o, place)).filter(|p| !is_held(p));
    let mut out = Vec::new();
    for (axis, value) in facets {
        match (*axis, does) {
            ("operation", Some(phrase)) => out.push(("does".to_string(), phrase.to_string())),
            ("locus.local", Some(_)) => {}
            ("operation", None) => out.push((String::from("operation"), (*value).to_string())),
            _ => out.push(((*axis).to_string(), words(axis, value).to_string())),
        }
    }
    out
}

/// The body of the `resolved profile` section: each capability's rows, then the verdict of the band.
pub fn render(ex: &super::bridge::ProfileExplanation) -> String {
    let mut out = String::new();
    for (because, facets) in &ex.capabilities {
        out.push_str(&format!("    · {because}\n"));
        for (name, words) in rows(facets) {
            out.push_str(&format!("        {name:<28} {words}\n"));
        }
    }
    match &ex.blocked_by {
        Some((level, miss)) => {
            out.push_str(&format!("\n  refused by `{level}` (the most permissive auto-approving level):\n    {}\n", mismatch(miss)));
        }
        None => out.push_str("\n  admitted by the auto-approve band.\n"),
    }
    out
}

/// The refusal line for a facet that stopped a level, in plain words.
pub fn mismatch(m: &super::level::FacetMismatch) -> String {
    match term(m.facet, m.actual) {
        Some(p) if !is_held(p) => match &m.plain_bound {
            Some(bound) => format!("{p} ({bound})"),
            None => m.to_string(),
        },
        _ => m.to_string(),
    }
}

/// Every axis `--explain` can print, with every term it can take, from the facet types.
pub fn vocabulary() -> Vec<(&'static str, Vec<&'static str>)> {
    fn all<T: FacetTerm>() -> Vec<&'static str> {
        T::all().iter().map(|t| t.as_str()).collect()
    }
    vec![
        ("operation", all::<Operation>()),
        ("locus.local", all::<LocalLocus>()),
        ("locus.remote", all::<RemoteReach>()),
        ("locus.binding", all::<RemoteBinding>()),
        ("locus.provenance", all::<Provenance>()),
        ("scale", all::<Scale>()),
        ("retrieval", all::<RetrievalGranularity>()),
        ("authority", all::<Authority>()),
        ("isolation", all::<Isolation>()),
        ("reversibility", all::<Reversibility>()),
        ("persistence.level", all::<PersistenceLevel>()),
        ("persistence.trigger.escape", all::<TriggerEscape>()),
        ("persistence.trigger.kind", all::<TriggerKind>()),
        ("disclosure.audience", all::<DisclosureAudience>()),
        ("disclosure.channel", all::<Channel>()),
        ("disclosure.principal", all::<Principal>()),
        ("secret.level", all::<SecretLevel>()),
        ("secret.channel", all::<Channel>()),
        ("secret.principal", all::<Principal>()),
        ("network.direction", all::<NetDirection>()),
        ("network.destination", all::<NetDestination>()),
        ("network.payload", all::<NetPayload>()),
        ("execution.trust", all::<ExecutionTrust>()),
        ("cost", all::<Cost>()),
        ("supply_chain.source", all::<SupplySource>()),
        ("supply_chain.pinning", all::<Pinning>()),
        ("supply_chain.exec_surface", all::<ExecSurface>()),
    ]
}

#[cfg(test)]
mod tests;

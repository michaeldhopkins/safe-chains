//! Static capability archetypes (`docs/design/behavioral-taxonomy-archetypes.md`). The recurring
//! facet profiles the Phase-1 subcommand surface classifies against. Each archetype is a fixed
//! [`Capability`] declared in `archetypes.toml`; a subcommand references one by name (`profile =
//! "remote-mutate"`) and the resolver emits that capability directly (a static profile — the
//! sub's facets don't depend on its arguments, unlike the operand-role commands of Phase 0).
//!
//! This is the archetype as a *reusable audited bundle*, never a unit of analysis: `profile = …`
//! expands to the explicit capability here, the researcher still verifies the sub genuinely is
//! that archetype and cites it (the per-item provenance schema). Facet fields take an EXACT term,
//! not a bound — these are points in facet-space, not the level predicates of `authoring`.

use std::collections::BTreeMap;
use std::sync::LazyLock;

use serde::Deserialize;

use super::facet::{Capability, FacetTerm, Operation};

/// The capability an archetype expands to, or `None` if the name is unknown (fail-closed: an
/// unknown `profile = …` must not silently resolve to nothing).
pub fn archetype(name: &str) -> Option<&'static Capability> {
    ARCHETYPES.get(name)
}

/// Every archetype name, for the `profile = …` closed-set check and the docs.
pub fn names() -> impl Iterator<Item = &'static str> {
    ARCHETYPES.keys().map(String::as_str)
}

static ARCHETYPES: LazyLock<BTreeMap<String, Capability>> = LazyLock::new(|| {
    build_archetypes(include_str!("../../archetypes.toml")).expect("embedded archetypes.toml must compile")
});

fn build_archetypes(src: &str) -> Result<BTreeMap<String, Capability>, String> {
    let set: TomlArchetypeSet = toml::from_str(src).map_err(|e| e.to_string())?;
    set.archetype
        .into_iter()
        .map(|(name, tc)| build_capability(&name, tc).map(|c| (name, c)))
        .collect()
}

fn build_capability(name: &str, tc: TomlCapability) -> Result<Capability, String> {
    let operation = Operation::from_term(&tc.operation)
        .ok_or_else(|| format!("archetype `{name}`: unknown operation `{}`", tc.operation))?;
    let mut c = Capability::new(operation);

    if let Some(l) = &tc.locus {
        set_term(name, "locus.local", l.local.as_deref(), &mut c.locus.local)?;
        set_term(name, "locus.remote", l.remote.as_deref(), &mut c.locus.remote)?;
        set_term(name, "locus.binding", l.binding.as_deref(), &mut c.locus.binding)?;
        set_term(name, "locus.provenance", l.provenance.as_deref(), &mut c.locus.provenance)?;
    }
    set_term(name, "scale", tc.scale.as_deref(), &mut c.scale)?;
    set_term(name, "retrieval", tc.retrieval.as_deref(), &mut c.retrieval)?;
    set_term(name, "authority", tc.authority.as_deref(), &mut c.authority)?;
    set_term(name, "reversibility", tc.reversibility.as_deref(), &mut c.reversibility)?;
    if let Some(p) = &tc.persistence {
        set_term(name, "persistence.level", p.level.as_deref(), &mut c.persistence.level)?;
    }
    if let Some(d) = &tc.disclosure {
        set_term(name, "disclosure.audience", d.audience.as_deref(), &mut c.disclosure.audience)?;
    }
    if let Some(s) = &tc.secret {
        set_term(name, "secret.level", s.level.as_deref(), &mut c.secret.level)?;
    }
    if let Some(net) = &tc.network {
        set_term(name, "network.direction", net.direction.as_deref(), &mut c.network.direction)?;
        set_term(name, "network.destination", net.destination.as_deref(), &mut c.network.destination)?;
        set_term(name, "network.payload", net.payload.as_deref(), &mut c.network.payload)?;
    }
    set_term(name, "execution", tc.execution.as_deref(), &mut c.execution.trust)?;
    set_term(name, "cost", tc.cost.as_deref(), &mut c.cost)?;

    if tc.because.trim().is_empty() {
        return Err(format!("archetype `{name}`: `because` is required"));
    }
    c.because = tc.because;
    Ok(c)
}

/// Parse an optional term into `slot`, leaving the zero-term default when absent. An unrecognized
/// term is a compile error naming the archetype and facet (fail-closed, never a silent skip).
fn set_term<T: FacetTerm>(name: &str, field: &str, s: Option<&str>, slot: &mut T) -> Result<(), String> {
    if let Some(v) = s {
        *slot = T::from_term(v).ok_or_else(|| format!("archetype `{name}`: unknown {field} term `{v}`"))?;
    }
    Ok(())
}

#[derive(Deserialize)]
struct TomlArchetypeSet {
    #[serde(default)]
    archetype: BTreeMap<String, TomlCapability>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TomlCapability {
    operation: String,
    because: String,
    #[serde(default)]
    locus: Option<TomlLocus>,
    #[serde(default)]
    scale: Option<String>,
    #[serde(default)]
    retrieval: Option<String>,
    #[serde(default)]
    authority: Option<String>,
    #[serde(default)]
    reversibility: Option<String>,
    #[serde(default)]
    persistence: Option<TomlPersistence>,
    #[serde(default)]
    disclosure: Option<TomlDisclosure>,
    #[serde(default)]
    secret: Option<TomlSecret>,
    #[serde(default)]
    network: Option<TomlNetwork>,
    #[serde(default)]
    execution: Option<String>,
    #[serde(default)]
    cost: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TomlLocus {
    local: Option<String>,
    remote: Option<String>,
    binding: Option<String>,
    provenance: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TomlPersistence {
    level: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TomlDisclosure {
    audience: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TomlSecret {
    level: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TomlNetwork {
    direction: Option<String>,
    destination: Option<String>,
    payload: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::authoring::default_levels;
    use crate::engine::bridge::project;
    use crate::engine::facet::Profile;
    use crate::engine::level::Level;
    use crate::verdict::Verdict;

    fn level(name: &str) -> &'static Level {
        default_levels().iter().find(|l| l.name == name).expect("level exists")
    }

    #[test]
    fn archetypes_toml_compiles_and_every_capability_is_justified() {
        // LazyLock forces the parse; a bad term / missing `because` would have panicked.
        let mut count = 0;
        for n in names() {
            let c = archetype(n).expect("listed archetype resolves");
            assert!(!c.because.is_empty(), "archetype `{n}` has no because");
            count += 1;
        }
        assert!(count >= 10, "expected the full catalog, got {count}");
        assert!(archetype("does-not-exist").is_none(), "unknown profile fails closed");
    }

    /// The catalog's "Lands at" column, verified against the real algebra by loading the ACTUAL
    /// archetype definitions (not hand-built copies): each is admitted by its claimed level and
    /// refused by the level just below it. Ties archetypes.toml ↔ the catalog doc ↔ the levels.
    #[test]
    fn archetypes_land_where_the_catalog_says() {
        // (archetype, admitted_by, refused_by)
        let cases: &[(&str, &str, &str)] = &[
            // A pure remote fetch is a READ — reader admits it; only paranoid (no network) refuses.
            ("remote-read", "reader", "paranoid"),
            // A BULK remote export (db dump to stdout) is still a read — reader admits it. `scale`
            // records the volume but does not gate a read; the -f output file is a SEPARATE cap.
            ("data-export", "reader", "paranoid"),
            ("remote-mutate", "network-admin", "developer"),
            ("remote-create", "network-admin", "developer"),
            ("remote-destroy-recoverable", "network-admin", "developer"),
            ("remote-destroy-irreversible", "yolo", "network-admin"),
            ("remote-authorize", "network-admin", "developer"),
            ("remote-control", "network-admin", "developer"),
            ("vcs-sync", "network-admin", "developer"),
            ("blockchain-txn", "yolo", "network-admin"),
            ("local-privileged", "local-admin", "developer"),
            // Transient service control (systemctl restart) — the mildest root-machine op, still local-admin.
            ("privileged-control", "local-admin", "developer"),
            // A pinned, scripts-off install runs no foreign code → developer (via the install clause).
            ("local-install-pinned", "developer", "editor"),
            // The scripts-on / unpinned install RUNS foreign code (network-sourced) → yolo only.
            ("supply-chain-build", "yolo", "developer"),
            // Arbitrary remote code execution (kubectl exec, ssh cmd) — execute op, no level below yolo.
            ("remote-exec", "yolo", "network-admin"),
            // Credential material read/mint → yolo (secret > uses-ambient everywhere below yolo).
            ("credential-read", "yolo", "network-admin"),
            ("credential-mint", "yolo", "network-admin"),
            // Arbitrary stored-object retrieval (s3 get-object): classified by `retrieval =
            // bulk-content` (§5 #1), it lands at NETWORK-ADMIN — the proportionate bulk-egress tier —
            // refused by developer. NOT yolo (it is not a credential read) and NOT reader (opaque bulk
            // content is above the everyday read band).
            ("bulk-object-read", "network-admin", "developer"),
        ];
        for (name, admitted_by, refused_by) in cases {
            let p = Profile::of(vec![archetype(name).expect("archetype exists").clone()]);
            assert!(level(admitted_by).admits(&p), "{name} should be admitted by {admitted_by}");
            assert!(!level(refused_by).admits(&p), "{name} should be refused by {refused_by}");
        }
    }

    /// The whole point of Phase 1 for the WRITE side: every remote archetype that CHANGES remote
    /// state (mutate/create/destroy/authorize/control), plus vcs-sync and blockchain-txn, is above
    /// the auto-approve band — denied by CLASSIFICATION, not hand-marking. `remote-read` is the
    /// deliberate exception: a pure fetch is a reader-level read and auto-approves (SafeRead).
    #[test]
    fn every_remote_write_archetype_is_not_auto_approved() {
        let write_remotes = names()
            .filter(|n| (n.starts_with("remote-") && *n != "remote-read") || *n == "vcs-sync" || *n == "blockchain-txn");
        for name in write_remotes {
            let p = Profile::of(vec![archetype(name).expect("archetype").clone()]);
            assert_eq!(project(&p), Verdict::Denied, "{name} must not auto-approve in the 3-value projection");
        }
        // and the read DOES auto-approve — the read/write asymmetry, verified
        assert_eq!(
            project(&Profile::of(vec![archetype("remote-read").unwrap().clone()])),
            Verdict::Allowed(crate::verdict::SafetyLevel::SafeRead),
            "a pure remote fetch is reader-level",
        );
    }

    /// The exposure reframe (behavioral-taxonomy-exposure.md §3, §7): `disclosure.audience = public`
    /// is a RECORD, not a gate. Publishing content you authored to a public destination (git push to
    /// a public repo, `npm publish`) is a network-admin operation — NOT held back to yolo by its
    /// publicness. What still gates to yolo is CONTENT: transmitting a secret off-box. Proves the
    /// gate moved from "how public the destination is" to "is a secret leaving". Red on the old
    /// `disclosure = { audience = "<= trusted-remote" }` ceiling (public publish refused everywhere
    /// below yolo); green on `<= public`.
    #[test]
    fn public_disclosure_is_recorded_not_gated_secret_transmission_is() {
        use crate::engine::facet::{
            DisclosureAudience, NetDestination, NetDirection, NetPayload, Network, RemoteReach,
            Reversibility, SecretLevel,
        };

        let publish_to_public = || {
            let mut c = Capability::new(Operation::Communicate);
            c.locus.remote = RemoteReach::Arbitrary;
            c.reversibility = Reversibility::Effortful;
            c.disclosure.audience = DisclosureAudience::Public;
            c.network = Network {
                direction: NetDirection::Outbound,
                destination: NetDestination::Arbitrary,
                payload: NetPayload::SendsHostData,
            };
            c
        };

        // Non-secret public publish → a network-admin op, still above the local developer band.
        let mut publish = publish_to_public();
        publish.because = "publish authored content to a public destination".into();
        let publish = Profile::of(vec![publish]);
        assert!(level("network-admin").admits(&publish), "public non-secret publish is network-admin");
        assert!(!level("developer").admits(&publish), "outbound remote egress is above developer");

        // Same shape, but it TRANSMITS A SECRET — now the CONTENT gates it up to yolo.
        let mut exfil = publish_to_public();
        exfil.secret.level = SecretLevel::Transmits;
        exfil.because = "transmit a secret off-box".into();
        let exfil = Profile::of(vec![exfil]);
        assert!(!level("network-admin").admits(&exfil), "secret transmission is the gate, above network-admin");
        assert!(level("yolo").admits(&exfil), "yolo admits secret exfil (non-destroy clause)");
    }

    /// The machine locus SUB-RUNG split (the `restart nginx` vs `/etc/passwd` distinction). ORDINARY
    /// machine state — a service, an app config — is `machine` → local-admin. The identity/auth/boot/
    /// loader TRUST substrate is `system-integrity` → ABOVE local-admin, yolo-only. Same operation +
    /// authority; only the locus rung differs, and that difference is the whole gate: "run the machine
    /// as admin" vs "own the machine's trust root".
    #[test]
    fn system_integrity_is_above_local_admin_ordinary_machine_is_not() {
        use crate::engine::facet::{Authority, LocalLocus};
        let (local, yolo) = (level("local-admin"), level("yolo"));

        let root_write_at = |loc| {
            let mut c = Capability::new(Operation::Mutate);
            c.locus.local = loc;
            c.authority = Authority::Root;
            c.because = "root machine write".into();
            Profile::of(vec![c])
        };

        // ordinary machine config (edit /etc/nginx.conf as root) — local-admin admits.
        assert!(local.admits(&root_write_at(LocalLocus::Machine)), "ordinary machine write is local-admin");

        // the trust substrate (rewrite /etc/passwd as root) — local-admin REFUSES; only yolo.
        let integrity = root_write_at(LocalLocus::SystemIntegrity);
        assert!(!local.admits(&integrity), "the system-integrity substrate is above local-admin");
        assert!(yolo.admits(&integrity), "yolo owns the machine's trust root");
    }

    /// The developer supply-chain / install clause. A PINNED, SCRIPTS-OFF install (`npm ci
    /// --ignore-scripts`) fetches packages and writes node_modules but runs NO foreign code —
    /// `execution = self`, `persistence = installing` → a dev-loop staple, admitted at developer.
    /// The scripts-ON or UNPINNED install is `execution = network-sourced` (the supply-chain-build
    /// archetype) → no home below yolo. The resolver picks which shape a command emits; this pins the
    /// LEVEL boundary. Modeling the safe install as `execution = self` (not a guardrail-gated
    /// `network-sourced`) is what keeps the clause all-`<=` and facet-monotone.
    #[test]
    fn pinned_scripts_off_install_is_developer_the_supply_chain_surface_is_yolo() {
        use crate::engine::facet::{
            ExecutionTrust, LocalLocus, NetDirection, NetPayload, PersistenceLevel, Reversibility,
        };
        let (dev, yolo) = (level("developer"), level("yolo"));

        // `npm ci --ignore-scripts`: install files, execute nothing foreign.
        let safe_install = {
            let mut c = Capability::new(Operation::Create);
            c.locus.local = LocalLocus::Worktree;
            c.persistence.level = PersistenceLevel::Installing;
            c.reversibility = Reversibility::Effortful;
            c.network.direction = NetDirection::Outbound;
            c.network.payload = NetPayload::Fetches;
            c.execution.trust = ExecutionTrust::SelfCode;
            c.because = "pinned, scripts-off install".into();
            Profile::of(vec![c])
        };
        assert!(dev.admits(&safe_install), "a pinned, scripts-off install is developer");
        assert!(yolo.admits(&safe_install), "and of course yolo");

        // scripts-ON / unpinned: the supply-chain surface (network-sourced execution).
        let supply_chain = Profile::of(vec![archetype("supply-chain-build").unwrap().clone()]);
        assert!(!dev.admits(&supply_chain), "network-sourced install (scripts on / unpinned) is above developer");
        assert!(yolo.admits(&supply_chain), "the supply-chain surface lands at yolo");
    }

    /// Destination-trust (behavioral-taxonomy-exposure.md §4): the new `locus.provenance` facet.
    /// A send to a target designated `literal` (a URL typed inline) is a network-admin op — the
    /// human reviewing at that level SEES the URL; a send to an `opaque` target (from a variable,
    /// unreviewable) is held to yolo. Proves network-admin's `provenance <= literal` ceiling. Red
    /// if the ceiling is absent (opaque would leak into network-admin) or set to `established`
    /// (literal URLs would be wrongly refused); green at `<= literal`.
    #[test]
    fn a_literal_send_target_is_network_admin_an_opaque_one_is_yolo() {
        use crate::engine::facet::{NetDirection, NetPayload, Provenance, RemoteReach};

        let send_to = |prov| {
            let mut c = Capability::new(Operation::Communicate);
            c.locus.remote = RemoteReach::Fixed;
            c.locus.provenance = prov;
            c.network.direction = NetDirection::Outbound;
            c.network.payload = NetPayload::SendsHostData;
            c.because = "send host data to a designated target".into();
            c
        };

        let literal = Profile::of(vec![send_to(Provenance::Literal)]);
        assert!(level("network-admin").admits(&literal), "a visible literal URL is a network-admin send");
        assert!(!level("developer").admits(&literal), "sends-host-data is above the local developer band");

        let opaque = Profile::of(vec![send_to(Provenance::Opaque)]);
        assert!(!level("network-admin").admits(&opaque), "an opaque (variable) destination is held above network-admin");
        assert!(level("yolo").admits(&opaque), "yolo leaves provenance unconstrained");
    }
}

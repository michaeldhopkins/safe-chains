//! Building a sub declaration into its runtime spec, out of `build.rs`. A child of `build`, whose
//! `fail!` and `ensure!` it uses.

use crate::policy::UnknownTolerance;

use super::*;

/// Builds one SubSpec per alias (canonical name first, then each alias).
/// All entries share the same kind via Clone — the dispatcher doesn't care
/// which name the user invoked. `handler_policies` is consulted only by
/// subs that set `policy = "key"`.
pub(super) fn build_subs(
    parent: &str,
    toml: TomlSub,
    handler_policies: &std::collections::HashMap<String, OwnedPolicy>,
) -> Result<Vec<SubSpec>, String> {
    let aliases = toml.aliases.clone();
    let canonical = build_sub(parent, toml, handler_policies)?;
    let mut out = Vec::with_capacity(1 + aliases.len());
    for alias in aliases {
        out.push(SubSpec { name: alias, ..canonical.clone() });
    }
    out.push(canonical);
    Ok(out)
}

/// Enforce the research standard at build time (reading the TOML fields, so provenance is a
/// production-validated part of the tree, not dead metadata). A sub with a `profile` must name a
/// real archetype and cite a `fact` + `source`; each escalating `[[command.sub.flag]]` must name a
/// real archetype (or the `unclassified` fail-closed escape) and cite its own `fact` + `source`. A
/// mis-authored classification fails CLOSED — the registry panics at load rather than silently
/// under-recording why a subcommand sits above the auto-approve line.
fn assert_sub_provenance(parent: &str, toml: &TomlSub) -> Result<(), String> {
    let cited = |o: &Option<String>| o.as_deref().is_some_and(|s| !s.trim().is_empty());
    // A `judgment` is optional (the discretionary layer), but if given it must say something.
    let judged = |o: &Option<String>| o.as_deref().is_none_or(|s| !s.trim().is_empty());
    if let Some(p) = &toml.profile {
        ensure!(
            crate::engine::archetype::archetype(p).is_some(),
            "{parent} sub `{}`: profile `{p}` is not a known archetype (archetypes.toml)",
            toml.name,
        );
        ensure!(cited(&toml.fact), "{parent} sub `{}`: `profile` requires a `fact`", toml.name);
        ensure!(cited(&toml.source), "{parent} sub `{}`: `profile` requires a `source`", toml.name);
        ensure!(judged(&toml.judgment), "{parent} sub `{}`: `judgment`, if given, must not be blank", toml.name);
        // A profiled sub must say what flags it accepts. The engine classifies it from its archetype
        // without running the legacy flag walk, so an EMPTY list is not "nothing permitted" — before
        // this was enforced it meant "everything permitted", and `git rebase --exec 'rm -rf /'`
        // classified as an ordinary rebase. Declaring `tolerate_unknown_*` is the explicit way to say
        // a surface is genuinely unbounded; saying nothing at all is not an option.
        ensure!(
            !toml.standalone.is_empty()
                || !toml.valued.is_empty()
                || toml.tolerate_unknown_short == Some(true)
                || toml.tolerate_unknown_long == Some(true)
                || !toml.flag.is_empty(),
            "{parent} sub `{}`: a profiled sub must declare its flag surface — list `standalone`/\
             `valued`, or set `tolerate_unknown_long = true` if it is genuinely unbounded",
            toml.name,
        );
        // A profiled sub is a leaf: the engine classifies it by archetype, and its legacy kind is
        // forced to deny-all (below) — a nested Branching would sidestep that.
        ensure!(toml.sub.is_empty(), "{parent} sub `{}`: a profiled sub must be a leaf (no nested subs)", toml.name);
    }
    // `network_destination` classifies the destination onto the archetype's `locus.provenance`, so
    // it only has meaning on a profiled sub.
    ensure!(
        toml.network_destination != Some(true) || toml.profile.is_some(),
        "{parent} sub `{}`: `network_destination` requires a `profile`",
        toml.name,
    );
    ensure!(
        toml.destination_flag.is_none() || toml.network_destination == Some(true),
        "{parent} sub `{}`: `destination_flag` requires `network_destination`",
        toml.name,
    );
    // An output-file path is only meaningful on a profiled (`data-export`) sub — the engine gates
    // that file's write onto the sub's derived profile.
    ensure!(
        toml.output_path_flags.is_empty() || toml.profile.is_some(),
        "{parent} sub `{}`: `output_path_flags` requires a `profile`",
        toml.name,
    );
    for f in &toml.flag {
        ensure!(
            f.classifies == "unclassified" || crate::engine::archetype::archetype(&f.classifies).is_some(),
            "{parent} sub `{}` flag `{}`: classifies `{}` is not a known archetype",
            toml.name,
            f.name,
            f.classifies,
        );
        ensure!(cited(&f.fact), "{parent} sub `{}` flag `{}`: requires a `fact`", toml.name, f.name);
        ensure!(cited(&f.source), "{parent} sub `{}` flag `{}`: requires a `source`", toml.name, f.name);
        ensure!(judged(&f.judgment), "{parent} sub `{}` flag `{}`: `judgment`, if given, must not be blank", toml.name, f.name);
        ensure!(
            !(f.when_absent == Some(true) && f.value_prefix.is_some()),
            "{parent} sub `{}` flag `{}`: `when_absent` and `value_prefix` are mutually exclusive",
            toml.name,
            f.name,
        );
    }
    Ok(())
}

/// `loopback_localizes` says "pointed at this machine, clear the facets the destination decides".
/// Two things must hold or the claim is incoherent:
///
/// - it needs a `loopback_valued` flag, since nothing else can establish that the destination IS
///   this machine;
/// - a DESTROY archetype may not set it. The emulator claim is unverifiable — `ssh -L
///   8000:dynamodb.<region>.amazonaws.com:443` makes `localhost:8000` production — and a wrong
///   guess costs a stray write for a mutate and the data for a delete. Refusing it here rather than
///   relying on the resolver's runtime skip means no future sub opts out by accident.
fn assert_loopback_localizes_is_coherent(parent: &str, name: &str, toml: &TomlSub) -> Result<(), String> {
    // Both the admission gate (`presents_unlisted_flag`) and the delta (`sub_loopback_localizes`)
    // are reached only through the PROFILED sub walk, so on an unprofiled sub the declaration is
    // inert — the flag would simply be missing from the legacy lists and deny, with nothing saying
    // why. Fails closed, but silently, which is its own kind of bug.
    ensure!(
        toml.loopback_valued.is_empty() || toml.profile.is_some(),
        "{parent} sub `{name}`: `loopback_valued` needs `profile` — the gate is only consulted on \
         a profiled sub, so here it would do nothing at all",
    );
    if !toml.loopback_localizes.unwrap_or(false) {
        return Ok(());
    }
    ensure!(
        !toml.loopback_valued.is_empty(),
        "{parent} sub `{name}`: `loopback_localizes` requires a `loopback_valued` flag — without \
         one nothing can establish that the destination is this machine",
    );
    // Asked of the archetype's OPERATION facet, not of its name. A name test passes today only
    // because every destroy archetype happens to be called `*destroy*`; a later `remote-wipe` or
    // `bucket-purge` carrying `operation = "destroy"` would sail through it and localize. Deciding
    // by name rather than behavior is the exact mistake this classifier exists to avoid.
    let profile = toml.profile.as_deref().unwrap_or_default();
    let operation = crate::engine::archetype::archetype(profile).map(|c| c.operation);
    ensure!(
        operation != Some(crate::engine::facet::Operation::Destroy),
        "{parent} sub `{name}`: `profile = \"{profile}\"` carries `operation = destroy` and may \
         not set `loopback_localizes` — a loopback endpoint cannot be verified (an SSH tunnel makes \
         localhost mean production), and that is only unrecoverable for destroy",
    );
    Ok(())
}

pub(super) fn build_sub(
    parent: &str,
    mut toml: TomlSub,
    handler_policies: &std::collections::HashMap<String, OwnedPolicy>,
) -> Result<SubSpec, String> {
    check_no_legacy_positional_style(&toml.name, toml.positional_style)?;
    let name = toml.name.clone();
    let policy_ref = toml.policy.clone();
    let profile = toml.profile.clone();
    assert_sub_provenance(parent, &toml)?;
    assert_no_candidate_shadowed_by_glob(&format!("{parent} {}", toml.name), &toml.sub, &toml.first_arg)?;
    let flags = std::mem::take(&mut toml.flag)
        .into_iter()
        .map(|f| crate::registry::types::FlagProvenance {
            name: f.name,
            classifies: f.classifies,
            value_prefix: f.value_prefix,
            when_absent: f.when_absent.unwrap_or(false),
        })
        .collect();
    // A profiled sub is engine-classified and above the auto-approve line. Its LEGACY dispatch is
    // reached only when the engine ABSTAINS — e.g. a global flag (`git -c …`, `git -C …`) intervenes
    // before the subcommand, so `sub_archetypes`'s walk stops early. In that case the legacy kind
    // MUST deny outright (fail-closed), never fall through to a permissive default: force bare off,
    // no flags, no positionals.
    // Preserve the DECLARED lists before the legacy kind is zeroed: the engine classifies a profiled
    // sub from its archetype without ever running the legacy flag walk, so these are the only copy
    // that survives, and `sub_archetypes` validates against them.
    let (allowed_standalone, allowed_valued) = (toml.standalone.clone(), toml.valued.clone());
    let allowed_unknown = match (toml.tolerate_unknown_short.unwrap_or(false), toml.tolerate_unknown_long.unwrap_or(false)) {
        (false, false) => UnknownTolerance::Strict,
        (true, false) => UnknownTolerance::Short,
        (false, true) => UnknownTolerance::Long,
        (true, true) => UnknownTolerance::Both,
    };
    if profile.is_some() {
        toml.bare = Some(false);
        toml.standalone = Vec::new();
        toml.valued = Vec::new();
        toml.max_positional = Some(0);
    }
    let eval_safe = toml.eval_safe.unwrap_or(false);
    let eval_safe_flags = std::mem::take(&mut toml.eval_safe_flags);
    let eval_safe_flag_values = std::mem::take(&mut toml.eval_safe_flag_values);
    let eval_safe_required_flags = std::mem::take(&mut toml.eval_safe_required_flags);
    let network_destination = toml.network_destination.unwrap_or(false);
    let destination_flag = toml.destination_flag.clone();
    let output_path_flags = toml.output_path_flags.clone();
    let loopback_valued = toml.loopback_valued.clone();
    let loopback_effect = if toml.loopback_localizes.unwrap_or(false) { LoopbackEffect::Localizes } else { LoopbackEffect::AdmitOnly };
    assert_loopback_localizes_is_coherent(parent, &name, &toml)?;
    let valued_for_check = toml.valued.clone();
    assert_eval_safe_flags_require_tag(parent, &name, eval_safe, &eval_safe_flags)?;
    assert_eval_safe_flag_values_consistent(parent, &name, &eval_safe_flags, &eval_safe_flag_values)?;
    assert_eval_safe_valued_flags_declared(parent, &name, &eval_safe_flags, &valued_for_check, &eval_safe_flag_values)?;
    assert_eval_safe_required_flags_consistent(parent, &name, &eval_safe_flags, &eval_safe_required_flags)?;
    assert_sub_eval_safe_only_on_leaf(parent, &toml)?;
    let writes_cwd =
        super::super::cwd_writes::lower(parent, &name, toml.writes_cwd.as_deref(), &toml.output_dirs, toml.executor.as_deref())?;
    let output = lower_output(&format!("{parent} {name}"), toml.output.as_ref())?;
    let flag_values = super::super::glob_flags::flag_values(parent, &name, &toml)?;
    Ok(SubSpec {
        name,
        output,
        writes_cwd,
        name_match: if toml.per_database { NameMatch::WithDatabaseSuffix } else { NameMatch::Exact },
        kind: super::super::passthrough_gate::build_sub_kind(parent, super::super::glob_flags::inherit_sub_flags(toml), handler_policies)?,
        policy_ref,
        profile,
        flags,
        allowed_standalone,
        allowed_valued,
        allowed_unknown,
        eval_safe,
        eval_safe_flags,
        eval_safe_flag_values,
        eval_safe_required_flags,
        network_destination,
        destination_flag,
        loopback_valued,
        loopback_effect,
        output_path_flags,
        flag_values,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        /// Every alias spelling of a sub carries the canonical sub's declared flag lists and
        /// matching mode, and the canonical spelling comes last.
        #[test]
        fn aliases_inherit_the_canonical_declaration(
            aliases in proptest::collection::btree_set("[a-z]{1,6}", 0..4),
            standalone in proptest::collection::btree_set("--[a-z]{1,6}", 0..4),
            valued in proptest::collection::btree_set("--[a-z]{1,6}", 0..3),
        ) {
            let standalone: Vec<String> = standalone.into_iter().collect();
            let valued: Vec<String> = valued.into_iter().filter(|v| !standalone.contains(v)).collect();
            let quoted = |words: &[String]| words.iter().map(|w| format!("\"{w}\"")).collect::<Vec<_>>().join(", ");
            let aliases: Vec<String> = aliases.into_iter().filter(|a| a != "canon").collect();
            let source = format!(
                "name = \"canon\"\nlevel = \"SafeWrite\"\naliases = [{}]\nstandalone = [{}]\nvalued = [{}]\n",
                quoted(&aliases), quoted(&standalone), quoted(&valued),
            );
            let toml: TomlSub = toml::from_str(&source).unwrap();
            let built = build_subs("cmd", toml, &HashMap::new()).unwrap();
            prop_assert_eq!(built.len(), aliases.len() + 1);
            prop_assert_eq!(built.last().map(|s| s.name.as_str()), Some("canon"));
            for sub in &built {
                prop_assert_eq!(&sub.allowed_standalone, &standalone);
                prop_assert_eq!(&sub.allowed_valued, &valued);
                prop_assert_eq!(sub.name_match, NameMatch::Exact);
            }
        }
    }
}

//! The transfer arm of behavior resolution: `cp`, `mv` and `ln`, and the paths a link reaches.

use super::capability::{Placing, breadth_scale, mutates, observes, observes_path, overwrites, relocates, transfer_profile, worst};
use super::flags::walk_value;
use super::{behavior_flag_present, locus};
use crate::engine::facet::{Capability, Profile, Scale};
use crate::parse::Token;
use crate::registry::types::TransferSpec;

/// The transfer arm of `resolve_behavior` (cp/mv/ln): split the operands into sources and a
/// destination (`-t`/`--target-directory` value, else the last operand), gate each at its locus
/// — a relocate source at its WRITE face — and fold in any path-flag capabilities. Fails closed
/// on a missing spec, a missing dest, or a `-t` dest with no sources.
pub(super) fn resolve_transfer(
    spec: &crate::registry::types::BehaviorSpec,
    operands: Vec<&str>,
    flag_caps: Vec<Capability>,
    tokens: &[Token],
) -> Profile {
    use crate::registry::types::TransferSource;
    let Some(t) = &spec.transfer else {
        return worst("behavior: transfer role without transfer spec — worst-cased (§0)");
    };
    // Whether the destination is DEFINITIVELY a container rather than the entry being created.
    // `-t DIR` says so outright, and with two or more sources the last operand must be a directory
    // for the command to make sense at all. Only the two-operand form is ambiguous, and there the
    // conservative reading (the destination is the entry) is the safe one.
    let (sources, dest, dest_is_container) = if let Some(d) = walk_value(&spec.valued_short, tokens, b't', "--target-directory") {
        if operands.is_empty() {
            return worst("behavior: transfer -t with no source operand — worst-cased (§0)");
        }
        (operands, d, true)
    } else {
        match operands.split_last() {
            Some((last, rest)) if !rest.is_empty() => (rest.to_vec(), *last, rest.len() >= 2),
            _ => return worst("behavior: transfer needs a source and a destination — worst-cased (§0)"),
        }
    };
    let no_clobber = if t.clobber_flags.is_empty() {
        t.no_clobber_flags.iter().any(|f| behavior_flag_present(tokens, f))
    } else {
        // A clobber flag PRESENT means overwrite; its absence is the no-clobber default.
        !t.clobber_flags.iter().any(|f| behavior_flag_present(tokens, f))
    };
    let recursive = t.recursive_flags.iter().any(|f| behavior_flag_present(tokens, f));
    let words: Vec<&str> = tokens.iter().map(Token::as_str).collect();
    let (on_link, follow) = (super::link_mode::acts_on_link(&words), super::link_mode::follows_links(&words));
    let transfer_scale = breadth_scale(&sources, recursive);
    // A relocate REMOVES its source, so the source name stops referring to anything: that is a
    // REBIND, not merely a write, and it is what makes `mv ~/.config elsewhere` a relocation of the
    // trust root rather than an edit of it.
    let source_face = match t.source {
        TransferSource::Relocate => locus::Face::Rebind,
        TransferSource::Observe => locus::Face::Read,
    };
    // `ln` points the destination NAME at something else; `cp`/`mv` write bytes at or under it.
    // Both are `create`/`transfer`, so only the command's own declaration separates them.
    //
    // But the declaration is about the ENTRY the command creates, and `ln -t DIR a` or
    // `ln a b DIR` puts that entry INSIDE the directory instead of replacing it. Treating those as
    // rebinds denied `ln -t ~/.config a`, which is an ordinary link into a directory you granted —
    // the same container-versus-object mistake the write face made before this face existed.
    let dest_face = if t.rebinds_destination && !dest_is_container { locus::Face::Rebind } else { locus::Face::Write };
    let set = set_flags(&spec.valued_short, &spec.valued_long, tokens);
    let no_deref = set.iter().any(|(f, _)| matches!(f.as_str(), "-n" | "-h" | "--no-dereference"));
    let replaced = t.rebinds_destination && !dest_is_container && !dest.ends_with('/') && replaces(dest, no_deref);
    let targets = if t.rebinds_destination { link_targets(t, &sources, dest, (dest_is_container, replaced), &set) } else { Vec::new() };
    let read_at: Vec<&str> = if t.rebinds_destination { targets.iter().map(String::as_str).collect() } else { sources.clone() };
    let mut prof = transfer_profile(
        &read_at,
        dest,
        transfer_scale,
        Placing { face: source_face, on_link, dest_is_entry: replaced },
        dest_face,
        |loc, sc| match t.source {
            TransferSource::Observe => observes(loc, sc, "transfer reads the source at its locus"),
            TransferSource::Relocate => relocates(loc, sc),
        },
        |loc, sc| overwrites(loc, sc, no_clobber),
    );
    prof.capabilities.extend(flag_caps);
    prof.capabilities.extend(targets.iter().map(String::as_str).map(reaches));
    if recursive && !t.rebinds_destination {
        let landed = |source: &&str| super::frozen_reach::landed(source, dest, follow);
        let landed = sources
            .iter()
            .flat_map(|source| if on_link { crate::pathctx::on_the_link(|| landed(source)) } else { landed(source) });
        prof.capabilities
            .extend(landed.map(|at| overwrites(locus::write_locus(&at), Scale::Single, no_clobber)));
        let through = sources.iter().filter(|_| follow).flat_map(|source| super::frozen_reach::read_through(source));
        prof.capabilities
            .extend(through.map(|at| observes_path(&at, Scale::Unbounded, "a copy that follows links reads where they lead")));
    }
    prof
}

/// A link makes its target reachable under the link's own name, so whatever later writes through
/// the link writes the target. The target is therefore judged as a rebind where the link is made.
fn reaches(target: &str) -> Capability {
    mutates(locus::rebind_locus(target), Scale::Single, "behavior: a link makes its target writable through the link's name")
}

/// The paths each source names once it is a link target. A hard link, or a symbolic link made with
/// a relative-flag, resolves the source as written. A symbolic link's relative target resolves
/// against the directory the link is made in: the destination itself when it is a container, and
/// otherwise its parent, or the destination when that turns out to be a directory, so both. A destination
/// `ln` replaces rather than enters is made in its parent alone.
fn link_targets(
    t: &TransferSpec,
    sources: &[&str],
    dest: &str,
    (dest_is_container, replaced): (bool, bool),
    set: &[(String, bool)],
) -> Vec<String> {
    let present = |flags: &[String], leading_only: bool| set.iter().any(|(f, leading)| (*leading || !leading_only) && flags.contains(f));
    let as_written = !present(&t.symbolic_flags, true) || present(&t.relative_flags, false);
    let joined = present(&t.symbolic_flags, false) && !present(&t.relative_flags, true);
    let container = dest_is_container || dest.ends_with('/');
    let dest = dest.trim_end_matches('/');
    let dest = if dest.is_empty() { "/" } else { dest };
    let bases: Vec<&str> = if container {
        vec![dest]
    } else if replaced {
        vec![parent(dest)]
    } else {
        vec![parent(dest), dest]
    };
    let mut targets = Vec::new();
    for source in sources {
        let pinned = source.starts_with(['/', '~', '$']);
        if as_written || pinned {
            targets.push((*source).to_string());
        }
        if joined && !pinned {
            targets.extend(bases.iter().map(|base| join(base, source)));
        }
    }
    targets
}

/// Whether `ln` makes its link in the destination's place rather than inside it: the destination
/// exists and is not a folder, or is a link that `-n` (`-h`) says to replace rather than enter.
fn replaces(dest: &str, no_deref: bool) -> bool {
    let Some(path) = crate::pathctx::on_the_link(|| super::frozen_walk::on_disk(dest)) else { return false };
    let Ok(own) = std::fs::symlink_metadata(&path) else { return false };
    (no_deref && own.file_type().is_symlink()) || std::fs::metadata(&path).is_ok_and(|m| !m.is_dir())
}

/// Every flag the command sets, each short flag of a cluster on its own. A valued flag ends its
/// cluster and takes the rest, or the next word, as its value, so `-Ss` sets `-S` alone. Words
/// after `--` are operands. Each flag records whether it came before the first operand: BSD ln, and
/// GNU ln under POSIXLY_CORRECT, read every word after it as an operand, so a later flag may not be one.
fn set_flags(valued_short: &[u8], valued_long: &[String], tokens: &[Token]) -> Vec<(String, bool)> {
    let mut set = Vec::new();
    let mut leading = true;
    let mut words = tokens.iter().skip(1).map(Token::as_str);
    while let Some(word) = words.next() {
        if word == "--" {
            break;
        }
        if let Some(long) = word.strip_prefix("--") {
            let (name, glued) = long.split_once('=').map_or((long, false), |(name, _)| (name, true));
            let name = format!("--{name}");
            if !glued && valued_long.contains(&name) {
                words.next();
            }
            set.push((name, leading));
        } else if let Some(cluster) = word.strip_prefix('-').filter(|c| !c.is_empty()) {
            for (at, flag) in cluster.bytes().enumerate() {
                set.push((format!("-{}", flag as char), leading));
                if valued_short.contains(&flag) {
                    if at + 1 == cluster.len() {
                        words.next();
                    }
                    break;
                }
            }
        } else {
            leading = false;
        }
    }
    set
}

fn parent(path: &str) -> &str {
    match path.rsplit_once('/') {
        Some(("", _)) => "/",
        Some((head, _)) => head,
        None => ".",
    }
}

fn join(base: &str, rel: &str) -> String {
    let unknown = |p: &str| p.contains('$') || p.contains(crate::cst::eval::TAGGED_PREFIX);
    if unknown(base) || unknown(rel) {
        return format!("{base}/{rel}");
    }
    let absolute = base.starts_with('/');
    let mut parts: Vec<&str> = Vec::new();
    for part in base.split('/').chain(rel.split('/')) {
        match part {
            "" | "." => {}
            ".." if parts.last().is_some_and(|p| *p != "..") => {
                parts.pop();
            }
            ".." if absolute => {}
            other => parts.push(other),
        }
    }
    match (absolute, parts.is_empty()) {
        (true, _) => format!("/{}", parts.join("/")),
        (false, true) => ".".to_string(),
        (false, false) => parts.join("/"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn components() -> impl Strategy<Value = Vec<&'static str>> {
        prop::collection::vec(prop::sample::select(vec!["a", "b", ".claude", "..", ".", ""]), 0..7)
    }

    proptest! {
        #[test]
        fn joining_keeps_every_climb_out_of_the_base_visible(base in components(), rel in components()) {
            let base = base.join("/");
            prop_assume!(!base.starts_with('/'));
            let rel = rel.join("/");
            let mut depth: i64 = 0;
            let mut escaped = false;
            for part in base.split('/').chain(rel.split('/')) {
                match part {
                    "" | "." => {}
                    ".." => depth -= 1,
                    _ => depth += 1,
                }
                escaped |= depth < 0;
            }
            let joined = join(&base, &rel);
            prop_assert_eq!(joined == ".." || joined.starts_with("../"), escaped, "join({:?}, {:?}) = {:?}", base, rel, joined);
            prop_assert!(!joined.split('/').skip(1).any(|p| p.is_empty() || p == "."), "{:?} is not normalized", joined);
        }

        #[test]
        fn an_absolute_base_stays_absolute(base in components(), rel in components()) {
            let joined = join(&format!("/{}", base.join("/")), &rel.join("/"));
            prop_assert!(joined.starts_with('/'));
            prop_assert!(!joined.split('/').any(|p| p == ".."));
        }

        #[test]
        fn an_unknown_component_is_never_normalized_away(base in components(), rel in components()) {
            let base = base.join("/");
            let rel = format!("$X/{}", rel.join("/"));
            prop_assert_eq!(join(&base, &rel), format!("{base}/{rel}"));
        }
    }

    #[test]
    fn a_link_resolves_from_its_own_directory() {
        let spec = crate::registry::types::TransferSpec {
            source: crate::registry::types::TransferSource::Observe,
            rebinds_destination: true,
            no_clobber_flags: Vec::new(),
            clobber_flags: Vec::new(),
            recursive_flags: Vec::new(),
            symbolic_flags: vec!["-s".to_string()],
            relative_flags: vec!["-r".to_string()],
        };
        let tokens = |words: &[&str]| words.iter().map(|w| Token::from_test(w)).collect::<Vec<_>>();
        let set = |words: &[&str]| set_flags(b"St", &["--suffix".to_string()], &tokens(words));
        let symbolic = set(&["ln", "-s"]);
        assert_eq!(link_targets(&spec, &["../t"], "sub/l", (false, false), &symbolic), ["t", "sub/t"]);
        assert_eq!(link_targets(&spec, &["../t"], "sub/l", (false, true), &symbolic), ["t"]);
        assert_eq!(link_targets(&spec, &["t"], "sub/", (false, false), &symbolic), ["sub/t"]);
        assert_eq!(link_targets(&spec, &["t"], "dir", (true, false), &symbolic), ["dir/t"]);
        assert_eq!(link_targets(&spec, &["/etc/t", "~/t"], "dir", (true, false), &symbolic), ["/etc/t", "~/t"]);
        assert_eq!(link_targets(&spec, &["../t"], "sub/l", (false, false), &set(&["ln"])), ["../t"]);
        assert_eq!(link_targets(&spec, &["../t"], "sub/l", (false, false), &set(&["ln", "-sr"])), ["../t"]);
        for hard in [&["ln", "-Ss", "x"][..], &["ln", "-S", "-s"], &["ln", "-tsub"], &["ln", "--suffix", "-s"], &["ln", "--", "-s"]] {
            assert_eq!(link_targets(&spec, &["../t"], "sub/l", (false, false), &set(hard)), ["../t"], "{hard:?} makes a hard link");
        }
        for as_written in [&["ln", "-s", "-Sr"][..], &["ln", "-s", "-S", "-r"]] {
            assert_eq!(
                link_targets(&spec, &["../t"], "sub/l", (false, false), &set(as_written)),
                ["t", "sub/t"],
                "{as_written:?} is not relative"
            );
        }
        let names = |words: &[&str]| set(words).into_iter().map(|(f, _)| f).collect::<Vec<_>>();
        assert_eq!(names(&["ln", "-fsSx", "--suffix=y", "-vt", "d"]), ["-f", "-s", "-S", "--suffix", "-v", "-t"]);
        assert_eq!(set(&["ln", "-f", "a", "-s"]), [("-f".to_string(), true), ("-s".to_string(), false)]);
        assert_eq!(link_targets(&spec, &["../t"], "sub/l", (false, false), &set(&["ln", "-s", "-r"])), ["../t"]);
        assert_eq!(link_targets(&spec, &["../t"], "sub/l", (false, false), &set(&["ln", "x", "-s"])), ["../t", "t", "sub/t"]);
        assert_eq!(link_targets(&spec, &["../t"], "sub/l", (false, false), &set(&["ln", "-s", "x", "-r"])), ["../t", "t", "sub/t"]);
    }
}

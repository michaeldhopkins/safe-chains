use super::check::ShellEffects;
use super::{ListOp, Stmt};

pub(super) type Cwd = Option<String>;

const MAX_CANDIDATE_CWDS: usize = 4;
const EXTRA_START_WORK: u32 = 8;

fn union(parts: &[&[Cwd]]) -> Vec<Cwd> {
    let mut out: Vec<Cwd> = Vec::new();
    for cwd in parts.iter().flat_map(|p| p.iter()) {
        if !out.contains(cwd) {
            out.push(cwd.clone());
        }
    }
    if out.len() > MAX_CANDIDATE_CWDS {
        out = vec![Some(crate::pathctx::UNRESOLVED_CWD.to_string())];
    }
    out
}

pub(super) struct CwdFlow {
    pub(super) ok: Vec<Cwd>,
    pub(super) fail: Vec<Cwd>,
}

impl CwdFlow {
    pub(super) fn begin(cwd: Cwd) -> CwdFlow {
        CwdFlow { ok: vec![cwd.clone()], fail: vec![cwd] }
    }

    pub(super) fn start(&self, previous: Option<ListOp>, ran_in: &[Cwd]) -> Vec<Cwd> {
        let starts = self.starts(previous, ran_in);
        if starts.len() > 1 && !super::check::charge_classify_work(EXTRA_START_WORK) {
            return vec![Some(crate::pathctx::UNRESOLVED_CWD.to_string())];
        }
        starts
    }

    fn starts(&self, previous: Option<ListOp>, ran_in: &[Cwd]) -> Vec<Cwd> {
        match previous {
            Some(ListOp::And) => self.ok.clone(),
            Some(ListOp::Or) => self.fail.clone(),
            Some(ListOp::Amp) => union(&[&self.ok, &self.fail, ran_in]),
            Some(ListOp::Semi) | None => union(&[&self.ok, &self.fail]),
        }
    }

    pub(super) fn then(self, previous: Option<ListOp>, own: CwdFlow) -> CwdFlow {
        match previous {
            Some(ListOp::And) => CwdFlow { fail: union(&[&self.fail, &own.fail]), ok: own.ok },
            Some(ListOp::Or) => CwdFlow { ok: union(&[&self.ok, &own.ok]), fail: own.fail },
            _ => own,
        }
    }
}

pub(super) fn statement_flow(stmt: &Stmt, effects: &ShellEffects, starts: &[Cwd]) -> CwdFlow {
    let flow = unnegated_flow(stmt, effects, starts);
    if stmt.pipeline.bang { CwdFlow { ok: flow.fail, fail: flow.ok } } else { flow }
}

fn unnegated_flow(stmt: &Stmt, effects: &ShellEffects, starts: &[Cwd]) -> CwdFlow {
    let landed: Vec<Option<Cwd>> = starts
        .iter()
        .map(|cwd| {
            let _cwd = crate::pathctx::enter_cwd(cwd.clone());
            super::cd::cd_target(&stmt.pipeline).map(|(target, physical)| {
                crate::pathctx::join_cwd_mode(cwd.as_deref(), &target, physical)
                    .or_else(|| Some(crate::pathctx::UNRESOLVED_CWD.to_string()))
            })
        })
        .collect();
    if landed.iter().any(Option::is_some) {
        let landed: Vec<Cwd> = landed.into_iter().flatten().collect();
        return CwdFlow { ok: union(&[&landed]), fail: starts.to_vec() };
    }
    if effects.cwd {
        let unresolved = [Some(crate::pathctx::UNRESOLVED_CWD.to_string())];
        let anywhere = union(&[&unresolved, starts]);
        return CwdFlow { ok: anywhere.clone(), fail: anywhere };
    }
    CwdFlow { ok: starts.to_vec(), fail: starts.to_vec() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn cwd() -> impl Strategy<Value = Cwd> {
        prop_oneof![Just(None), "[a-c]{1,2}".prop_map(|name| Some(format!("/{name}")))]
    }

    proptest! {
        #[test]
        fn a_union_keeps_every_folder_or_collapses_to_unknown(a in prop::collection::vec(cwd(), 0..4), b in prop::collection::vec(cwd(), 0..4)) {
            let merged = union(&[&a, &b]);
            let unknown = vec![Some(crate::pathctx::UNRESOLVED_CWD.to_string())];
            if merged != unknown {
                prop_assert!(merged.len() <= MAX_CANDIDATE_CWDS);
                for cwd in a.iter().chain(b.iter()) {
                    prop_assert!(merged.contains(cwd));
                }
            }
        }

        #[test]
        fn exactly_the_cap_of_distinct_folders_is_kept_and_one_more_collapses(names in prop::collection::hash_set("[a-z]{1,3}", 0..8)) {
            let folders: Vec<Cwd> = names.iter().map(|name| Some(format!("/{name}"))).collect();
            let merged = union(&[&folders]);
            if folders.len() <= MAX_CANDIDATE_CWDS {
                prop_assert_eq!(merged.len(), folders.len());
            } else {
                prop_assert_eq!(merged, vec![Some(crate::pathctx::UNRESOLVED_CWD.to_string())]);
            }
        }

        #[test]
        fn an_or_keeps_the_successes_of_both_sides_and_the_failures_of_the_last(a in prop::collection::vec(cwd(), 1..2), b in prop::collection::vec(cwd(), 1..2), c in prop::collection::vec(cwd(), 1..2), d in prop::collection::vec(cwd(), 1..2)) {
            let after = CwdFlow { ok: a.clone(), fail: b }.then(Some(ListOp::Or), CwdFlow { ok: c.clone(), fail: d.clone() });
            prop_assert_eq!(after.fail, d);
            for cwd in a.iter().chain(c.iter()) {
                prop_assert!(after.ok.contains(cwd));
            }
        }

        #[test]
        fn a_flow_after_an_or_starts_only_from_failures(a in prop::collection::vec(cwd(), 1..3), b in prop::collection::vec(cwd(), 1..2)) {
            let flow = CwdFlow { ok: a, fail: b.clone() };
            prop_assert_eq!(flow.start(Some(ListOp::Or), &[]), b);
        }
    }
}

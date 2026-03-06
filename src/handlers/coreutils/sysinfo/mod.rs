mod arch;
mod cal;
mod dust;
mod groups;
mod htop;
mod id;
mod ioreg;
mod iotop;
mod last;
mod lastlog;
mod locale;
mod lsof;
mod nproc;
mod pgrep;
mod procs;
mod ps;
mod sleep;
mod sw_vers;
mod system_profiler;
mod top;
mod tty;
mod uname;
mod uptime;
mod vm_stat;
mod w;
mod who;

use crate::command::FlatDef;
use crate::parse::{Segment, Token};

pub(super) fn dispatch(cmd: &str, tokens: &[Token], is_safe: &dyn Fn(&Segment) -> bool) -> Option<bool> {
    for flat in all_flat_defs() {
        if let r @ Some(_) = flat.dispatch(cmd, tokens) {
            return r;
        }
    }
    arch::dispatch(cmd, tokens, is_safe)
}

pub(super) fn command_docs() -> Vec<crate::docs::CommandDoc> {
    let mut docs: Vec<_> = all_flat_defs().iter().map(|d| d.to_doc()).collect();
    docs.extend(arch::command_docs());
    docs
}

pub(super) fn all_flat_defs() -> Vec<&'static FlatDef> {
    let mut v = Vec::new();
    v.extend(cal::FLAT_DEFS);
    v.extend(dust::FLAT_DEFS);
    v.extend(groups::FLAT_DEFS);
    v.extend(htop::FLAT_DEFS);
    v.extend(id::FLAT_DEFS);
    v.extend(ioreg::FLAT_DEFS);
    v.extend(iotop::FLAT_DEFS);
    v.extend(last::FLAT_DEFS);
    v.extend(lastlog::FLAT_DEFS);
    v.extend(locale::FLAT_DEFS);
    v.extend(lsof::FLAT_DEFS);
    v.extend(nproc::FLAT_DEFS);
    v.extend(pgrep::FLAT_DEFS);
    v.extend(procs::FLAT_DEFS);
    v.extend(ps::FLAT_DEFS);
    v.extend(sleep::FLAT_DEFS);
    v.extend(sw_vers::FLAT_DEFS);
    v.extend(system_profiler::FLAT_DEFS);
    v.extend(top::FLAT_DEFS);
    v.extend(tty::FLAT_DEFS);
    v.extend(uname::FLAT_DEFS);
    v.extend(uptime::FLAT_DEFS);
    v.extend(vm_stat::FLAT_DEFS);
    v.extend(w::FLAT_DEFS);
    v.extend(who::FLAT_DEFS);
    v
}

#[cfg(test)]
pub(super) fn registry() -> Vec<&'static crate::handlers::CommandEntry> {
    let mut v = Vec::new();
    v.extend(arch::REGISTRY);
    v
}

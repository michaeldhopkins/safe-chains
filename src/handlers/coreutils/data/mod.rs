mod base64;
mod bc;
mod dasel;
mod echo;
mod expr;
mod factor;
mod fx;
mod getconf;
mod gojq;
mod htmlq;
mod jaq;
mod jless;
mod jq;
mod mlr;
mod printf;
mod seq;
mod shuf;
mod sort;
mod test_cmd;
mod tomlq;
mod uuidgen;
mod xmllint;
mod xq;
mod xxd;
mod yq;

use crate::command::FlatDef;
use crate::verdict::Verdict;
use crate::parse::Token;

pub(super) fn dispatch(cmd: &str, tokens: &[Token]) -> Option<Verdict> {
    for flat in all_flat_defs() {
        if let r @ Some(_) = flat.dispatch(cmd, tokens) {
            return r;
        }
    }
    dasel::dispatch(cmd, tokens)
        .or_else(|| mlr::dispatch(cmd, tokens))
}

pub(super) fn command_docs() -> Vec<crate::docs::CommandDoc> {
    let mut docs: Vec<_> = all_flat_defs().iter().map(|d| d.to_doc()).collect();
    docs.extend(dasel::command_docs());
    docs.extend(mlr::command_docs());
    docs
}

#[cfg(test)]
pub(super) fn registry() -> Vec<&'static crate::handlers::CommandEntry> {
    let mut v = Vec::new();
    v.extend(dasel::REGISTRY);
    v.extend(mlr::REGISTRY);
    v
}

pub(super) fn all_flat_defs() -> Vec<&'static FlatDef> {
    let mut v = Vec::new();
    v.extend(base64::FLAT_DEFS);
    v.extend(bc::FLAT_DEFS);
    v.extend(echo::FLAT_DEFS);
    v.extend(expr::FLAT_DEFS);
    v.extend(factor::FLAT_DEFS);
    v.extend(fx::FLAT_DEFS);
    v.extend(getconf::FLAT_DEFS);
    v.extend(gojq::FLAT_DEFS);
    v.extend(htmlq::FLAT_DEFS);
    v.extend(jaq::FLAT_DEFS);
    v.extend(jless::FLAT_DEFS);
    v.extend(jq::FLAT_DEFS);
    v.extend(printf::FLAT_DEFS);
    v.extend(seq::FLAT_DEFS);
    v.extend(shuf::FLAT_DEFS);
    v.extend(sort::FLAT_DEFS);
    v.extend(test_cmd::FLAT_DEFS);
    v.extend(tomlq::FLAT_DEFS);
    v.extend(uuidgen::FLAT_DEFS);
    v.extend(xmllint::FLAT_DEFS);
    v.extend(xq::FLAT_DEFS);
    v.extend(xxd::FLAT_DEFS);
    v.extend(yq::FLAT_DEFS);
    v
}

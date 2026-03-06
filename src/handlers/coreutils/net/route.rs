use crate::parse::{Segment, Token, WordSet};

static ROUTE_SAFE_FLAGS: WordSet = WordSet::new(&["-4", "-6", "-n", "-v"]);
static ROUTE_SAFE_SUBCMDS: WordSet = WordSet::new(&["get", "monitor", "print", "show"]);

fn is_safe_route(tokens: &[Token]) -> bool {
    let mut i = 1;
    while i < tokens.len() {
        let t = &tokens[i];
        if ROUTE_SAFE_FLAGS.contains(t) {
            i += 1;
            continue;
        }
        if ROUTE_SAFE_SUBCMDS.contains(t) {
            return true;
        }
        return false;
    }
    true
}

pub(in crate::handlers::coreutils) fn dispatch(cmd: &str, tokens: &[Token], _is_safe: &dyn Fn(&Segment) -> bool) -> Option<bool> {
    match cmd {
        "route" => Some(is_safe_route(tokens)),
        _ => None,
    }
}

pub(in crate::handlers::coreutils) fn command_docs() -> Vec<crate::docs::CommandDoc> {
    vec![
        crate::docs::CommandDoc::handler("route",
            "- Allowed subcommands: get, monitor, print, show\n- Allowed flags: -4, -6, -n, -v\n- Bare invocation allowed"),
    ]
}

#[cfg(test)]
pub(in crate::handlers::coreutils) const REGISTRY: &[crate::handlers::CommandEntry] = &[
    crate::handlers::CommandEntry::Positional { cmd: "route" },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        route_bare: "route",
        route_n: "route -n",
        route_print: "route print",
        route_get: "route get 8.8.8.8",
        route_show: "route show",
        route_monitor: "route monitor",
        route_dash_v: "route -v",
        route_n_get: "route -n get 8.8.8.8",
        route_4_get: "route -4 get 8.8.8.8",
    }

    denied! {
        route_add_denied: "route add default 192.168.1.1",
        route_del_denied: "route del default",
        route_delete_denied: "route delete default",
        route_change_denied: "route change default 192.168.1.1",
        route_flush_denied: "route flush",
        route_replace_denied: "route replace default via 192.168.1.1",
        route_n_add_denied: "route -n add default 192.168.1.1",
    }
}

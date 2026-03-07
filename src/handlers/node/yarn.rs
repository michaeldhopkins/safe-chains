use crate::parse::{Segment, Token, WordSet};
use crate::policy::{self, FlagPolicy, FlagStyle};

static YARN_LIST_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--json", "--long", "--production"]),
    standalone_short: b"",
    valued: WordSet::flags(&["--depth", "--pattern"]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static YARN_BARE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--json"]),
    standalone_short: b"",
    valued: WordSet::flags(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub fn is_safe_yarn(tokens: &[Token]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    if tokens.len() == 2 && matches!(tokens[1].as_str(), "--help" | "-h" | "--version" | "-V") {
        return true;
    }
    match tokens[1].as_str() {
        "list" | "ls" => policy::check(&tokens[1..], &YARN_LIST_POLICY),
        "info" | "why" => policy::check(&tokens[1..], &YARN_BARE_POLICY),
        "test" => true,
        _ => tokens[1].starts_with("test:"),
    }
}

pub(crate) fn dispatch(cmd: &str, tokens: &[Token], _is_safe: &dyn Fn(&Segment) -> bool) -> Option<bool> {
    match cmd {
        "yarn" => Some(is_safe_yarn(tokens)),
        _ => None,
    }
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    vec![
        crate::docs::CommandDoc::handler("yarn",
            "https://yarnpkg.com/cli",
            "Subcommands: info, list, ls, test, test:*, why."),
    ]
}

#[cfg(test)]
pub(crate) const REGISTRY: &[crate::handlers::CommandEntry] = &[
    crate::handlers::CommandEntry::Subcommand { cmd: "yarn", subs: &[
        crate::handlers::SubEntry::Policy { name: "list" },
        crate::handlers::SubEntry::Policy { name: "ls" },
        crate::handlers::SubEntry::Policy { name: "info" },
        crate::handlers::SubEntry::Policy { name: "why" },
        crate::handlers::SubEntry::Positional { name: "test" },
    ]},
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        yarn_list: "yarn list --depth=0",
        yarn_list_json: "yarn list --json",
        yarn_ls: "yarn ls bootstrap",
        yarn_info: "yarn info react",
        yarn_info_json: "yarn info react --json",
        yarn_why: "yarn why lodash",
        yarn_version: "yarn --version",
        yarn_test: "yarn test",
        yarn_test_watch: "yarn test:watch",
        yarn_test_with_args: "yarn test --testPathPattern=Foo",
    }
}

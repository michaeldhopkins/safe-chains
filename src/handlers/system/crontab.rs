use crate::parse::Token;

fn is_safe_crontab(tokens: &[Token]) -> bool {
    let args: Vec<&str> = tokens[1..].iter().map(|t| t.as_str()).collect();
    matches!(args.as_slice(), ["-l"] | ["-l", "-u", _] | ["-u", _, "-l"])
}

pub(in crate::handlers::system) fn dispatch(cmd: &str, tokens: &[Token]) -> Option<bool> {
    match cmd {
        "crontab" => Some(is_safe_crontab(tokens)),
        _ => None,
    }
}

pub(in crate::handlers::system) fn command_docs() -> Vec<crate::docs::CommandDoc> {
    vec![
        crate::docs::CommandDoc::handler("crontab",
            "https://ss64.com/mac/crontab.html",
            "Allowed: -l (list), -l -u <user>."),
    ]
}

#[cfg(test)]
pub(in crate::handlers::system) const REGISTRY: &[crate::handlers::CommandEntry] = &[
    crate::handlers::CommandEntry::Positional { cmd: "crontab" },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        crontab_list: "crontab -l",
        crontab_list_user: "crontab -l -u root",
        crontab_list_user_reversed: "crontab -u root -l",
    }

    denied! {
        crontab_edit: "crontab -e",
        crontab_remove: "crontab -r",
        crontab_install_file: "crontab mycron.txt",
        bare_crontab: "crontab",
        crontab_remove_user: "crontab -r -u root",
    }
}

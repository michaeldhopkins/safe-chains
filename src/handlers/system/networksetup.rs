use crate::parse::Token;
use crate::verdict::{SafetyLevel, Verdict};

fn is_safe_networksetup(tokens: &[Token]) -> Verdict {
    if tokens.len() < 2 {
        return Verdict::Denied;
    }
    let sub = tokens[1].as_str();
    if !(sub.starts_with("-list")
        || sub.starts_with("-get")
        || sub.starts_with("-show")
        || sub.starts_with("-print")
        || sub == "-version"
        || sub == "-help")
    {
        return Verdict::Denied;
    }
    Verdict::Allowed(SafetyLevel::Inert)

}

pub(in crate::handlers::system) fn dispatch(cmd: &str, tokens: &[Token]) -> Option<Verdict> {
    match cmd {
        "networksetup" => Some(is_safe_networksetup(tokens)),
        _ => None,
    }
}

pub(in crate::handlers::system) fn command_docs() -> Vec<crate::docs::CommandDoc> {
    vec![
        crate::docs::CommandDoc::handler("networksetup",
            "https://ss64.com/mac/networksetup.html",
            "Allowed: subcommands starting with -list, -get, -show, -print, \
             plus -version and -help."),
    ]
}

#[cfg(test)]
pub(in crate::handlers::system) const REGISTRY: &[crate::handlers::CommandEntry] = &[
    crate::handlers::CommandEntry::Positional { cmd: "networksetup" },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        networksetup_listallhardwareports: "networksetup -listallhardwareports",
        networksetup_listallnetworkservices: "networksetup -listallnetworkservices",
        networksetup_getinfo: "networksetup -getinfo Wi-Fi",
        networksetup_getdnsservers: "networksetup -getdnsservers Wi-Fi",
        networksetup_version: "networksetup -version",
        networksetup_help: "networksetup -help",
    }

    denied! {
        networksetup_setdnsservers_denied: "networksetup -setdnsservers Wi-Fi 8.8.8.8",
        networksetup_setairportpower_denied: "networksetup -setairportpower en0 on",
        networksetup_no_args_denied: "networksetup",
    }
}

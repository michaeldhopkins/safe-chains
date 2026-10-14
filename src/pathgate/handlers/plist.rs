//! Property-list tools whose operand roles follow a mode the tool's own grammar selects.

use crate::parse::Token;
use crate::pathgate::{Role, gate};

/// `plutil MODE [args] file…` — `-p`, `-lint` and `-type` read their files and print what they
/// hold (`-p` prints the whole plist, `-lint` a parser error quoting it). `-convert` and
/// `-extract` REWRITE the file in place unless `-o` names where the result goes, so without `-o`
/// their operands are write targets, and with it they are reads and the `-o` value is the write.
/// Only an `-o` in option position counts: one in the mode's own arguments (`-extract -o json F`,
/// where `-o` is the key path) or after `--` is a file name, and the operands are still rewritten.
///
/// Every bare token is gated, the mode's format name and key path included: `-type KEYPATH` and
/// `-extract KEYPATH FMT` take them positionally, and a key path that happens to spell a sensitive
/// path only adds a refusal, while skipping by position would leave `plutil -type ~/.ssh/id_rsa`
/// (one operand, read as the file by a reader that counts) ungated.
pub(in crate::pathgate) fn plutil_mode(tokens: &[Token]) -> bool {
    let mode_args = match tokens.get(1).map(Token::as_str) {
        Some("-convert") => 1,
        Some("-extract") => 2,
        _ => 0,
    };
    let rewrites = mode_args > 0;
    let has_output = tokens.iter().skip(2 + mode_args).take_while(|t| *t != "--").any(|t| t == "-o");
    let role = if rewrites && !has_output { Role::Write } else { Role::Read };
    let mut i = 2;
    while i < tokens.len() {
        let t = tokens[i].as_str();
        if t == "-o" {
            if let Some(v) = tokens.get(i + 1)
                && v != "-"
                && gate(Role::Write, v.as_str())
            {
                return true;
            }
            i += 2;
            continue;
        }
        if t == "--" {
            return tokens[i + 1..].iter().any(|p| gate(role, p.as_str()));
        }
        if !(t.starts_with('-') && t != "-") && gate(role, t) {
            return true;
        }
        i += 1;
    }
    false
}

/// `defaults read|read-type|export [-g | -app NAME | DOMAIN] …` — the domain is a preferences
/// name or a path to a plist, and `defaults` reads that path with `.plist` appended when it is
/// missing, so both spellings are judged. `-g`/`-globalDomain` and `-app NAME` choose the domain
/// themselves, and then no positional is one. `export` writes the domain to its last operand
/// (`-` is stdout); the other positionals of `read` and `read-type` are keys.
pub(in crate::pathgate) fn defaults_domain(tokens: &[Token]) -> bool {
    let export = tokens.first().is_some_and(|t| t == "export");
    let mut named_domain = true;
    let mut positionals: Vec<&str> = Vec::new();
    let mut i = 1;
    while i < tokens.len() {
        let t = tokens[i].as_str();
        match t {
            "-app" => {
                named_domain = false;
                i += 2;
                continue;
            }
            "-g" | "-globalDomain" => named_domain = false,
            "--" => {
                positionals.extend(tokens[i + 1..].iter().map(Token::as_str));
                break;
            }
            _ if t.starts_with('-') && t != "-" => {}
            _ => positionals.push(t),
        }
        i += 1;
    }
    let domain_read = named_domain
        && positionals
            .first()
            .is_some_and(|d| gate(Role::Read, d) || (!d.ends_with(".plist") && gate(Role::Read, &format!("{d}.plist"))));
    let output_written = export
        && positionals
            .last()
            .is_some_and(|out| *out != "-" && (!named_domain || positionals.len() > 1) && gate(Role::Write, out));
    domain_read || output_written
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use crate::pathctx::PathCtx;

    const ROOT: &str = "/work/projects/app";

    fn denied(cmd: &str) -> bool {
        let ctx = PathCtx { cwd: Some(ROOT.into()), root: Some(ROOT.into()), session_id: None };
        !crate::command_verdict_in(cmd, ctx).is_allowed()
    }

    #[test]
    fn plutil_reads_are_gated_as_reads() {
        for cmd in [
            "plutil -p ~/.ssh/id_rsa", "plutil -p ~/.aws/credentials", "plutil -lint ~/.ssh/id_rsa", "plutil -type key ~/.ssh/id_rsa",
            "plutil -extract a raw -o - ~/.ssh/id_rsa", "plutil -convert xml1 -o - ~/.ssh/id_rsa",
        ] {
            assert!(denied(cmd), "{cmd}");
        }
        for cmd in ["plutil -p Info.plist", "plutil -lint a.plist b.plist", "plutil -convert json -o - ~/Library/Preferences/x.plist"] {
            assert!(!denied(cmd), "{cmd}");
        }
    }

    #[test]
    fn plutil_rewrites_are_gated_as_writes() {
        for cmd in [
            "plutil -convert xml1 /etc/hosts",
            "plutil -convert xml1 ~/.ssh/config",
            "plutil -convert xml1 -o /etc/x.plist a.plist",
            "plutil -extract a raw /etc/hosts",
            "plutil -convert binary1 ~/Library/Preferences/x.plist",
            "plutil -convert json -- -o ~/Library/LaunchAgents/x.plist",
            "plutil -convert json -s -- -o /etc/x.plist",
            "plutil -extract a json -- -o ~/Library/Preferences/x.plist",
            "plutil -extract -o json /etc/x.plist",
            "plutil -extract -o json ~/Library/Preferences/x.plist",
        ] {
            assert!(denied(cmd), "{cmd}");
        }
        for cmd in ["plutil -convert xml1 a.plist", "plutil -convert json -o out.json a.plist", "plutil -extract a json a.plist"] {
            assert!(!denied(cmd), "{cmd}");
        }
    }

    #[test]
    fn defaults_gates_the_domain_path_and_the_export_target() {
        for cmd in [
            "defaults read ~/.ssh/id_rsa", "defaults read ~/.ssh/known", "defaults read-type ~/.ssh/id_rsa key",
            "defaults export ~/.ssh/id_rsa -", "defaults export com.apple.finder /etc/x.plist", "defaults export -g /etc/x.plist",
            "defaults export -app Safari /etc/x.plist",
        ] {
            assert!(denied(cmd), "{cmd}");
        }
        for cmd in [
            "defaults read com.apple.finder",
            "defaults read com.apple.finder AppleShowAllFiles",
            "defaults read -g AppleLocale",
            "defaults read -app Safari",
            "defaults read ~/Library/Preferences/com.apple.finder",
            "defaults export com.apple.finder -",
            "defaults export com.apple.finder out.plist",
        ] {
            assert!(!denied(cmd), "{cmd}");
        }
    }

    proptest! {
        /// A plist tool never shows a file `cat` would refuse to: every reading spelling of each
        /// tool is at least as strict as `cat` on the same path.
        #[test]
        fn plist_readers_are_never_laxer_than_cat(
            path in prop::sample::select(vec![
                "~/.ssh/id_rsa", "~/.aws/credentials", "/etc/shadow", "~/.netrc", "~/.gnupg",
                "a.plist", "~/Library/Preferences/x.plist", "/etc/hosts", "../x", "$X", "~/.config/gh/hosts.yml",
            ]),
            reader in prop::sample::select(vec![
                "plutil -p {}", "plutil -lint {}", "plutil -type k {}", "plutil -extract k raw -o - {}",
                "plutil -convert json -o - {}", "defaults read {}", "defaults read-type {} k", "defaults export {} -",
                "mdls {}", "mdls -n kMDItemFSName {}", "exiftool {}", "exiftool -a -G {}",
            ]),
        ) {
            let cmd = reader.replace("{}", path);
            let cat = format!("cat {path}");
            prop_assert!(denied(&cmd) || !denied(&cat), "{cmd} approved while {cat} denies");
        }

        /// An `-o` that plutil reads as a file name, not as the output option, never turns an
        /// in-place rewrite into an approvable read.
        #[test]
        fn a_non_option_dash_o_never_relaxes_a_rewrite(
            path in prop::sample::select(vec!["/etc/x.plist", "~/Library/LaunchAgents/x.plist", "~/.ssh/config", "a.plist"]),
            form in prop::sample::select(vec![
                "plutil -convert json {} -- -o x", "plutil -convert json -- -o {}", "plutil -extract -o json {}",
                "plutil -extract k json -- -o {}", "plutil -extract -o -o {}",
            ]),
        ) {
            let cmd = form.replace("{}", path);
            let plain = format!("plutil -convert json {path}");
            prop_assert!(denied(&cmd) || !denied(&plain), "{cmd} approved while {plain} denies");
        }
    }
}

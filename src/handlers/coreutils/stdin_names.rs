use super::archive_extract::Given;
use crate::engine::resolve::{read_content_verdict, write_target_verdict};
use crate::parse::Token;

/// Some archivers take the names of the files they read from stdin rather than from their
/// arguments: `cpio -o` and `cpio -p`, `pax -w` with no file operand and `pax -r -w` with only
/// the destination, and `zip -@`. Each name is read, so `echo ~/.ssh/id_rsa | cpio -o` prints a
/// key, and a pass-through copy also writes it at the destination joined with the name: bsdcpio
/// drops a leading `/` there and keeps `..`, so `echo ../x | cpio -p out` writes `x` beside `out`.
///
/// The names are judged as `xargs` judges the items it hands its command: by the path that stands
/// for what the previous stage of the pipeline prints. With no stage before it, or one whose output
/// no path stands for (a redirect from a list file, a here-string), nothing can be checked and the
/// line is refused.
pub(crate) fn reads_names_it_cannot_check(tokens: &[Token]) -> bool {
    let destination = match names(tokens) {
        Names::Unread => return false,
        Names::Read => None,
        Names::ReadAndWrittenBelow(destination) => Some(destination),
    };
    let Some(item) = crate::pathctx::stdin_item_repr() else {
        return true;
    };
    if !read_content_verdict(&item).is_allowed() {
        return true;
    }
    destination.is_some_and(|destination| {
        let landing = format!("{}/{}", destination.trim_end_matches('/'), item.trim_start_matches('/'));
        !write_target_verdict(&landing).is_allowed()
    })
}

enum Names<'a> {
    Unread,
    Read,
    ReadAndWrittenBelow(&'a str),
}

fn names(tokens: &[Token]) -> Names<'_> {
    let Some(first) = tokens.first() else {
        return Names::Unread;
    };
    let command = first.command_name();
    let arguments = &tokens[1..];
    match command {
        "cpio" => {
            let given = Given::read(arguments, command);
            let has = |letter: char, long: &str| given.anywhere.contains(letter) || given.long_anywhere.contains(&long);
            if has('p', "--pass-through") {
                given.operands.first().map_or(Names::Read, |destination| Names::ReadAndWrittenBelow(destination))
            } else if has('o', "--create") {
                Names::Read
            } else {
                Names::Unread
            }
        }
        "pax" => {
            let given = Given::read(arguments, command);
            match (given.anywhere.contains('r'), given.anywhere.contains('w'), given.operands.as_slice()) {
                (false, true, []) => Names::Read,
                (true, true, [destination]) => Names::ReadAndWrittenBelow(destination),
                _ => Names::Unread,
            }
        }
        "zip" => {
            let from_stdin = arguments
                .iter()
                .map(Token::as_str)
                .take_while(|word| *word != "--")
                .any(|word| word == "--names-stdin" || (word.starts_with('-') && !word.starts_with("--") && word.contains('@')));
            if from_stdin { Names::Read } else { Names::Unread }
        }
        _ => Names::Unread,
    }
}

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    use proptest::prelude::*;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        cpio_archives_found_names: "find ./src | cpio -o -O ./a.cpio",
        cpio_archives_an_echoed_name: "echo ./notes.txt | cpio -o",
        cpio_copies_found_names: "find ./src -name '*.rs' | cpio -pdm ./dst",
        cpio_copies_listed_names: "git ls-files | cpio -pdm ./dst",
        cpio_copies_an_absolute_readable_name: "echo /etc/hosts | cpio -pd ./dst",
        pax_archives_found_names: "find ./src | pax -w -f ./a.pax",
        pax_copies_found_names: "find ./src | pax -rw ./dst",
        zip_archives_found_names: "find ./src | zip -@ ./a.zip",
        zip_archives_named_files: "zip ./a.zip ./notes.txt",
    }

    denied! {
        cpio_archives_a_key: "echo ~/.ssh/id_rsa | cpio -o",
        cpio_archives_a_key_into_a_file: "echo ~/.ssh/id_rsa | cpio -oO ./a.cpio",
        cpio_archives_a_key_long: "echo ~/.ssh/id_rsa | cpio --create",
        cpio_copies_a_key: "echo ~/.ssh/id_rsa | cpio -pd ./out",
        cpio_copies_a_key_long: "echo ~/.ssh/id_rsa | cpio --pass-through -d ./out",
        cpio_copies_out_of_the_destination: "echo ../../.zshrc | cpio -pd ./out",
        cpio_copies_onto_a_frozen_name: "echo .envrc | cpio -p ./dst",
        cpio_copies_into_git_metadata: "echo .git/hooks/pre-commit | cpio -pd ./dst",
        cpio_archives_names_from_nowhere: "cpio -o",
        cpio_copies_names_from_nowhere: "cpio -p ./dst",
        cpio_archives_names_from_a_list_file: "cpio -o < ./list.txt",
        cpio_archives_names_a_filter_read: "cat ./list.txt | cpio -o",
        cpio_archives_names_rewritten_by_sed: "find ./src | sed s@.*@/etc/master.passwd@ | cpio -o",
        cpio_archives_names_from_a_system_tree: "find /etc/ssh | cpio -o",
        pax_archives_a_key: "echo ~/.ssh/id_rsa | pax -w",
        pax_archives_a_key_into_a_file: "echo ~/.ssh/id_rsa | pax -w -f ./a.pax",
        pax_copies_a_key: "echo ~/.ssh/id_rsa | pax -rw ./out",
        zip_archives_a_key: "echo ~/.ssh/id_rsa | zip -@ ./a.zip",
        zip_archives_a_key_long: "echo ~/.ssh/id_rsa | zip --names-stdin ./a.zip",
        zip_archives_a_key_in_a_cluster: "echo ~/.ssh/id_rsa | zip -q@ ./a.zip",
    }

    proptest! {
        /// A name an archiver reads from stdin is read: whatever line reads it, it is approved only
        /// where reading that name is, and a pass-through copy only where writing it below the
        /// destination is too.
        #[test]
        fn a_name_from_stdin_is_approved_only_where_reading_it_is(
            name in prop::sample::select(vec![
                "./notes.txt", "src/lib.rs", "/etc/hosts", "~/.ssh/id_rsa", "~/.aws/credentials", "/etc/master.passwd",
                "../outside.txt", "../../.zshrc", ".envrc", ".git/config", "/etc/ssh/ssh_host_ed25519_key",
            ]),
            line in prop::sample::select(vec![
                "cpio -o", "cpio -ov", "cpio -o -H newc -O ./a.cpio", "cpio --create", "pax -w", "pax -w -f ./a.pax",
                "zip -@ ./a.zip", "zip -q@ ./a.zip", "cpio -p ./dst", "cpio -pdm ./dst", "cpio --pass-through ./dst", "pax -rw ./dst",
            ]),
        ) {
            let piped = format!("echo {name} | {line}");
            if is_safe_command(&piped) {
                prop_assert!(is_safe_command(&format!("cat {name}")), "{} approved a name cat may not read", piped);
                if line.ends_with("./dst") {
                    let landing = format!("./dst/{}", name.trim_start_matches('/'));
                    prop_assert!(is_safe_command(&format!("touch {landing}")), "{} approved a copy to {}", piped, landing);
                }
            }
            prop_assert!(!is_safe_command(line), "{} approved names it never saw", line);
        }
    }
}

use crate::parse::Token;

/// Extraction writes the paths and modes the archive names, set-id bits among them, which
/// nothing here can see without opening the archive. `tar x` and `unzip` are refused for that
/// reason, so pax, cpio and ditto are refused the same way when they extract. Listing stays
/// approved, and so do their copy modes: those keep the modes of files already on disk. xar strips
/// set-id bits unless told to keep them, so only that option is refused there.
///
/// A letter that makes a line safe (pax's `-w`, cpio's `-t` and `--list`) counts only in front of
/// the first operand. bsdcpio stops reading options there, and so does pax when POSIXLY_CORRECT is
/// set, so `cpio -i '*' -t` extracts every member and reads `-t` as one more pattern. A letter
/// that extracts counts wherever it appears.
///
/// bsdcpio reads `-W name` as the long option `--name`, abbreviations included, so `cpio -W ext`
/// is `cpio --extract`. Its value is a long option no list here can check, so cpio's `-W` is
/// refused whatever else the line says. A pass-through copy with `--insecure` drops the defenses
/// against `..`, absolute paths and symlinks, so it is refused as extraction is. pax's `-s`
/// rewrites each member's name by a regular expression, so where a copy writes is no longer the
/// path it was given, and it is refused too.
///
/// ptar extracts with `-x`, and ptargrep extracts every member that matches unless `-l` or
/// `--list-only` comes before the first operand (Getopt::Long stops there under POSIXLY_CORRECT).
pub(crate) fn extracts(tokens: &[Token]) -> bool {
    let Some(first) = tokens.first() else {
        return false;
    };
    let given = || Given::read(&tokens[1..], first.command_name());
    match first.command_name() {
        "pax" => {
            let given = given();
            given.anywhere.contains('s') || (given.anywhere.contains('r') && !given.leading.contains('w'))
        }
        "cpio" => {
            let given = given();
            let extract = given.anywhere.contains('i') || given.long_anywhere.contains(&"--extract");
            let passes = given.anywhere.contains('p') || given.long_anywhere.contains(&"--pass-through");
            given.anywhere.contains('W')
                || (passes && given.long_anywhere.contains(&"--insecure"))
                || (extract && !(given.leading.contains('t') || given.long_leading.contains(&"--list")))
        }
        "ditto" => given().anywhere.contains('x'),
        "ptar" => given().anywhere.contains('x'),
        "ptargrep" => {
            let given = given();
            !(given.leading.contains('l') || given.long_leading.contains(&"--list-only"))
        }
        "xar" => tokens[1..].iter().any(|t| t.as_str() == "--keep-setuid"),
        _ => false,
    }
}

/// The option letters and long options given, everywhere and in front of the first operand,
/// skipping the value each value-taking letter consumes. cpio's list is the union of bsdcpio's and GNU
/// cpio's (`-D` dir, `-M` message), so `cpio -Dt -i` extracts into `t` rather than listing.
struct Given<'a> {
    anywhere: String,
    leading: String,
    long_anywhere: Vec<&'a str>,
    long_leading: Vec<&'a str>,
}

impl<'a> Given<'a> {
    fn read(arguments: &'a [Token], command: &str) -> Given<'a> {
        let takes_a_value: &[char] = match command {
            "pax" => &['B', 'E', 'G', 'T', 'U', 'b', 'f', 'o', 'p', 's', 'x'],
            "ptar" => &['T', 'f'],
            "cpio" => &['C', 'D', 'E', 'F', 'H', 'I', 'M', 'O', 'R', 'W', 'f'],
            _ => &[],
        };
        let mut given = Given { anywhere: String::new(), leading: String::new(), long_anywhere: Vec::new(), long_leading: Vec::new() };
        let mut operand_seen = false;
        let mut words = arguments.iter().map(Token::as_str);
        while let Some(word) = words.next() {
            if word == "--" {
                break;
            }
            if word.starts_with("--") {
                given.long_anywhere.push(word);
                if !operand_seen {
                    given.long_leading.push(word);
                }
                if matches!(word, "--file" | "--format" | "--owner" | "--passphrase") && command == "cpio" {
                    words.next();
                }
                continue;
            }
            let Some(cluster) = word.strip_prefix('-').filter(|c| !c.is_empty()) else {
                operand_seen = true;
                continue;
            };
            for (at, c) in cluster.char_indices() {
                given.anywhere.push(c);
                if !operand_seen {
                    given.leading.push(c);
                }
                if takes_a_value.contains(&c) {
                    if at + c.len_utf8() == cluster.len() {
                        words.next();
                    }
                    break;
                }
            }
        }
        given
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
        pax_list: "pax -f ./archive.tar",
        pax_list_verbose: "pax -v -f ./archive.tar",
        pax_write: "pax -w -f ./out.tar ./src",
        pax_copy: "pax -rw ./src ./dst",
        pax_copy_keeping_modes: "pax -r -w -p e ./src ./dst",
        pax_list_archive_named_r: "pax -f r",
        cpio_list: "cpio -it",
        cpio_list_spaced: "cpio -i -t",
        cpio_list_long: "cpio --extract --list",
        cpio_create: "cpio -o",
        cpio_pass_through: "cpio -p ./dst",
        cpio_list_file_named_i: "cpio -t -F i",
        ditto_copy: "ditto ./src ./dst",
        ditto_archive: "ditto -c -k ./src ./out.zip",
        xar_extract: "xar -x -f ./archive.xar",
        ptar_list: "ptar -t -f ./a.tar",
        ptar_create: "ptar -c -f ./out.tar ./src",
        ptar_list_archive_named_x: "ptar -t -f x",
        ptargrep_list: "ptargrep -l pattern ./a.tar",
        ptargrep_list_long: "ptargrep --list-only -i pattern ./a.tar",
    }

    denied! {
        pax_copy_renaming_members: "pax -rw -s ,^,../../, ./src ./dst",
        pax_copy_renaming_to_a_file: "pax -rw -s ,.*,/x/.zshrc, ./x ./dst",
        pax_list_renaming: "pax -s ,r,w, -f ./archive.tar",
        pax_write_renaming: "pax -w -s ,a,b, -f ./out.tar ./src",
        pax_extract: "pax -r -f ./archive.tar",
        pax_extract_keeping_modes: "pax -r -p e -f ./archive.tar",
        pax_extract_cluster: "pax -rf ./archive.tar",
        pax_extract_glued_mode: "pax -r -pp -f ./archive.tar",
        cpio_extract: "cpio -i",
        cpio_extract_keeping_times: "cpio -im",
        cpio_extract_directories: "cpio -idm",
        cpio_extract_long: "cpio --extract",
        cpio_extract_from_file: "cpio -i -F ./archive.cpio",
        ditto_extract: "ditto -x ./archive.cpio ./out",
        ditto_extract_zip: "ditto -xk ./archive.zip ./out",
        ditto_extract_spaced: "ditto -x -k ./archive.zip ./out",
        xar_extract_keeping_set_id_bits: "xar -x --keep-setuid -f ./archive.xar",
        pax_extract_write_letter_after_a_pattern: "pax -r -f ./archive.tar '*' -w",
        pax_extract_write_letter_after_an_operand: "pax -r ./a -w ./b",
        ptar_extract: "ptar -x -f ./a.tar",
        ptar_extract_cluster: "ptar -xvf ./a.tar",
        ptar_extract_bare: "ptar -x",
        ptar_extract_after_the_archive: "ptar -f ./a.tar -x",
        ptargrep_extracts_matches: "ptargrep pattern ./a.tar",
        ptargrep_list_letter_after_an_operand: "ptargrep pattern ./a.tar -l",
        ptargrep_list_after_separator: "ptargrep -- pattern ./a.tar --list-only",
        cpio_pass_through_without_its_defenses: "cpio -p --insecure ./dst",
        cpio_pass_through_without_its_defenses_after: "cpio -pd ./dst --insecure",
        cpio_pass_through_long_without_its_defenses: "cpio --pass-through --insecure ./dst",
        cpio_extract_list_letter_after_a_pattern: "cpio -i '*' -t",
        cpio_extract_list_long_after_a_pattern: "cpio -i x --list",
        cpio_extract_spelled_as_a_long_name: "cpio -W extract",
        cpio_extract_spelled_as_a_long_name_glued: "cpio -Wextract",
        cpio_extract_spelled_as_an_abbreviated_long_name: "cpio -v -W ext",
        cpio_list_letter_with_a_long_name: "cpio -t -W extract",
        cpio_extract_into_a_directory_named_t: "cpio -Dt -i",
        cpio_extract_with_a_message_t: "cpio -iMt",
        cpio_extract_with_a_spaced_message: "cpio -i -M t",
    }

    proptest! {
        /// Whatever else is given, a pax, cpio or ditto line that extracts is refused, and one
        /// that only lists is not refused for extracting. A letter after the first operand is a
        /// pattern to a parser that stops there (bsdcpio, or pax under POSIXLY_CORRECT), so a
        /// listing or copy letter only counts in front of the operands.
        #[test]
        fn an_extraction_is_refused_and_a_listing_is_not(
            command in prop::sample::select(vec!["pax -r", "pax -rv", "cpio -i", "cpio -iv", "cpio --extract", "ditto -x", "ditto -xk"]),
            extra in prop::sample::subsequence(vec!["-v", "-k", "-p e", "-m", "-d"], 0..3),
        ) {
            let extracts = |line: &str| {
                let tokens: Vec<crate::parse::Token> = line.split_whitespace().map(crate::parse::Token::from_test).collect();
                super::extracts(&tokens)
            };
            let line = format!("{command} {} ./archive ./out", extra.join(" "));
            prop_assert!(extracts(&line), "{}", line);
            prop_assert!(!is_safe_command(&line), "{}", line);
            if !command.starts_with("ditto") {
                let safe_letter = if command.starts_with("pax") { "-w" } else { "-t" };
                let before = format!("{command} {safe_letter} {} ./archive ./out", extra.join(" "));
                prop_assert!(!extracts(&before), "{}", before);
                let after = format!("{line} {safe_letter}");
                prop_assert!(extracts(&after), "{}", after);
                prop_assert!(!is_safe_command(&after), "{}", after);
            }
        }
    }
}

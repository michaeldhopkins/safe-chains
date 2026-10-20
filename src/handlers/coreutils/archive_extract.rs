use crate::parse::Token;

/// Extraction writes the paths and modes the archive names, set-id bits among them, which
/// nothing here can see without opening the archive. `tar x` and `unzip` are refused for that
/// reason, so pax, cpio and ditto are refused the same way when they extract. Listing stays
/// approved, and so do their copy modes: those keep the modes of files already on disk. xar strips
/// set-id bits unless told to keep them, so only that option is refused there.
pub(crate) fn extracts(tokens: &[Token]) -> bool {
    let Some(first) = tokens.first() else {
        return false;
    };
    let letters = || short_letters(&tokens[1..], first.command_name());
    match first.command_name() {
        "pax" => {
            let letters = letters();
            letters.contains('r') && !letters.contains('w')
        }
        "cpio" => {
            let letters = letters();
            let long = |name: &str| tokens[1..].iter().any(|t| t.as_str() == name);
            (letters.contains('i') || long("--extract")) && !(letters.contains('t') || long("--list"))
        }
        "ditto" => letters().contains('x'),
        "xar" => tokens[1..].iter().any(|t| t.as_str() == "--keep-setuid"),
        _ => false,
    }
}

/// The option letters given, skipping the value each value-taking letter consumes.
fn short_letters(arguments: &[Token], command: &str) -> String {
    let takes_a_value: &[char] = match command {
        "pax" => &['B', 'E', 'G', 'T', 'U', 'b', 'f', 'o', 'p', 's', 'x'],
        "cpio" => &['C', 'E', 'F', 'H', 'I', 'O', 'R', 'f'],
        _ => &[],
    };
    let mut letters = String::new();
    let mut words = arguments.iter().map(Token::as_str);
    while let Some(word) = words.next() {
        if word == "--" {
            break;
        }
        let Some(cluster) = word.strip_prefix('-').filter(|c| !c.is_empty() && !c.starts_with('-')) else {
            if matches!(word, "--file" | "--format" | "--owner" | "--passphrase") && command == "cpio" {
                words.next();
            }
            continue;
        };
        for (at, c) in cluster.char_indices() {
            if takes_a_value.contains(&c) {
                if at + c.len_utf8() == cluster.len() {
                    words.next();
                }
                break;
            }
            letters.push(c);
        }
    }
    letters
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
        pax_list_substitution_with_r: "pax -s ,r,w, -f ./archive.tar",
        cpio_list: "cpio -it",
        cpio_list_spaced: "cpio -i -t",
        cpio_list_long: "cpio --extract --list",
        cpio_create: "cpio -o",
        cpio_pass_through: "cpio -p ./dst",
        cpio_list_file_named_i: "cpio -t -F i",
        ditto_copy: "ditto ./src ./dst",
        ditto_archive: "ditto -c -k ./src ./out.zip",
        xar_extract: "xar -x -f ./archive.xar",
    }

    denied! {
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
    }

    proptest! {
        /// Whatever else is given, a pax, cpio or ditto line that extracts is refused, and one
        /// that only lists is not refused for extracting.
        #[test]
        fn an_extraction_is_refused_and_a_listing_is_not(
            command in prop::sample::select(vec!["pax -r", "pax -rv", "cpio -i", "cpio -iv", "cpio --extract", "ditto -x", "ditto -xk"]),
            extra in prop::sample::subsequence(vec!["-v", "-k", "-p e", "-m", "-d"], 0..3),
        ) {
            let line = format!("{command} {} ./archive ./out", extra.join(" "));
            let tokens: Vec<crate::parse::Token> = line.split_whitespace().map(crate::parse::Token::from_test).collect();
            prop_assert!(super::extracts(&tokens), "{}", line);
            prop_assert!(!is_safe_command(&line), "{}", line);
            if !command.starts_with("ditto") {
                let listing = format!("{line} -t");
                let tokens: Vec<crate::parse::Token> = listing.split_whitespace().map(crate::parse::Token::from_test).collect();
                prop_assert_eq!(super::extracts(&tokens), command.starts_with("pax"), "{}", listing);
            }
        }
    }
}

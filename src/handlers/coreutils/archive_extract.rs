use crate::parse::Token;

/// Extraction writes the paths and modes the archive names, set-id bits among them, which
/// nothing here can see without opening the archive. `tar x` and `unzip` are refused for that
/// reason, so pax, cpio, ditto, xar and xip are refused the same way when they extract. Listing stays
/// approved, and so do their copy modes: those keep the modes of files already on disk. xar strips
/// set-id bits unless told to keep them, but the archive still names the paths it writes.
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
/// GNU cpio opens an archive named `host:path` or `user@host:path` (`-F`, `-I`, `-O`, `--file`)
/// on another machine through rsh or ssh, unless `--force-local` is given. The name is judged under
/// both cpios' option letters, as `extracts` judges them.
pub(crate) fn opens_a_remote_archive(tokens: &[Token]) -> bool {
    let arguments = tokens.get(1..).unwrap_or_default();
    if arguments.iter().any(|token| token.as_str() == "--force-local") {
        return false;
    }
    let remote = |name: &str| name.split('/').next().is_some_and(|head| head.contains(':'));
    let bsd: &[char] = &['C', 'D', 'E', 'F', 'H', 'I', 'M', 'O', 'R', 'W', 'f'];
    [bsd, &bsd[..bsd.len() - 1]].iter().any(|takes_a_value| {
        let mut words = arguments.iter().map(Token::as_str);
        while let Some(word) = words.next() {
            if word == "--" {
                return false;
            }
            if let Some(name) = word.strip_prefix("--file=") {
                if remote(name) {
                    return true;
                }
                continue;
            }
            if word == "--file" {
                if words.next().is_some_and(remote) {
                    return true;
                }
                continue;
            }
            let Some(cluster) = word.strip_prefix('-').filter(|c| !c.is_empty() && !c.starts_with('-')) else {
                continue;
            };
            for (at, c) in cluster.char_indices() {
                if takes_a_value.contains(&c) {
                    let glued = &cluster[at + c.len_utf8()..];
                    let value = if glued.is_empty() { words.next() } else { Some(glued) };
                    if matches!(c, 'F' | 'I' | 'O') && value.is_some_and(remote) {
                        return true;
                    }
                    break;
                }
            }
        }
        false
    })
}

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
        "cpio" => Given::readings(&tokens[1..], "cpio").iter().any(cpio_extracts),
        "ditto" => given().anywhere.contains('x'),
        "ptar" => given().anywhere.contains('x'),
        "ptargrep" => {
            let given = given();
            !(given.leading.contains('l') || given.long_leading.contains(&"--list-only"))
        }
        "xar" => given().anywhere.contains('x') || tokens[1..].iter().any(|t| t.as_str() == "--keep-setuid"),
        "xip" => tokens[1..].iter().any(|t| t.as_str() == "--expand" || t.as_str().starts_with("--expand=")),
        _ => crate::registry::canonical_name(first.command_name()) == "7z" && seven_zip_extracts(&tokens[1..]),
    }
}

/// 7-Zip's command is its first word that is not a switch; `x` and `e` extract, in either case.
/// 7-Zip glues every switch value (`-odir`), but a word after a switch spelled alone that this
/// registry reads as valued (`-o dir`) is judged as the command too.
fn seven_zip_extracts(arguments: &[Token]) -> bool {
    let (_, valued) = crate::registry::command_flag_sets("7z").unwrap_or((&[], &[]));
    let mut after_valued = false;
    for word in arguments.iter().map(Token::as_str) {
        if word.starts_with('-') && word != "-" {
            after_valued = valued.iter().any(|flag| flag == word);
            continue;
        }
        if word.eq_ignore_ascii_case("x") || word.eq_ignore_ascii_case("e") {
            return true;
        }
        if !after_valued {
            return false;
        }
        after_valued = false;
    }
    false
}

fn cpio_extracts(given: &Given<'_>) -> bool {
    let extract = given.anywhere.contains('i') || given.long_anywhere.contains(&"--extract");
    let passes = given.anywhere.contains('p') || given.long_anywhere.contains(&"--pass-through");
    given.anywhere.contains('W')
        || (passes && given.long_anywhere.contains(&"--insecure"))
        || (extract && !(given.leading.contains('t') || given.long_leading.contains(&"--list")))
}

/// The option letters and long options given, everywhere and in front of the first operand,
/// skipping the value each value-taking letter consumes. cpio's list is the union of bsdcpio's and GNU
/// cpio's (`-D` dir, `-M` message), so `cpio -Dt -i` extracts into `t` rather than listing.
pub(super) struct Given<'a> {
    pub(super) anywhere: String,
    leading: String,
    pub(super) long_anywhere: Vec<&'a str>,
    long_leading: Vec<&'a str>,
    pub(super) operands: Vec<&'a str>,
}

impl<'a> Given<'a> {
    pub(super) fn read(arguments: &'a [Token], command: &str) -> Given<'a> {
        let takes_a_value: &[char] = match command {
            "pax" => &['B', 'E', 'G', 'T', 'U', 'b', 'f', 'o', 'p', 's', 'x'],
            "ptar" => &['T', 'f'],
            "cpio" => &['C', 'D', 'E', 'F', 'H', 'I', 'M', 'O', 'R', 'W', 'f'],
            "xar" => &['C', 'f', 'n', 's'],
            _ => &[],
        };
        Given::read_with(arguments, command, takes_a_value)
    }

    /// Every way the line can be read. bsdcpio's `-f` takes a pattern and GNU cpio's `-f`
    /// (`--nonmatching`) takes nothing, so `cpio -fidm` is a listing-free extraction to GNU cpio
    /// and a pattern `idm` to bsdcpio. A caller judges each reading and refuses when any is refused.
    pub(super) fn readings(arguments: &'a [Token], command: &str) -> Vec<Given<'a>> {
        let mut readings = vec![Given::read(arguments, command)];
        if command == "cpio" {
            readings.push(Given::read_with(arguments, command, &['C', 'D', 'E', 'F', 'H', 'I', 'M', 'O', 'R', 'W']));
        }
        readings
    }

    fn read_with(arguments: &'a [Token], command: &str, takes_a_value: &[char]) -> Given<'a> {
        let mut given = Given {
            anywhere: String::new(),
            leading: String::new(),
            long_anywhere: Vec::new(),
            long_leading: Vec::new(),
            operands: Vec::new(),
        };
        let mut operand_seen = false;
        let mut words = arguments.iter().map(Token::as_str);
        while let Some(word) = words.next() {
            if word == "--" {
                given.operands.extend(words.by_ref());
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
                given.operands.push(word);
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

    #[test]
    fn only_a_single_dash_cluster_names_a_remote_archive() {
        let line = |words: &[&str]| words.iter().map(|w| crate::parse::Token::from_test(w)).collect::<Vec<_>>();
        assert!(super::opens_a_remote_archive(&line(&["cpio", "-Ihost:a.cpio"])));
        assert!(!super::opens_a_remote_archive(&line(&["cpio", "--XIhost:a.cpio"])));
        assert!(!super::opens_a_remote_archive(&line(&["cpio", "-", "-t"])));
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
        cpio_create: "find ./src | cpio -o",
        cpio_pass_through: "find ./src | cpio -p ./dst",
        cpio_pass_through_long: "find ./src | cpio --pass-through ./dst",
        ptar_archive_named_like_a_flag: "ptar -t -f -x",
        ptar_create_archive_named_like_a_flag: "ptar -c -f -x ./src",
        cpio_list_file_named_i: "cpio -t -F i",
        ditto_copy: "ditto ./src ./dst",
        ditto_archive: "ditto -c -k ./src ./out.zip",
        xar_list: "xar -t -f ./archive.xar",
        xar_create: "xar -c -f ./out.xar ./src",
        xar_create_glued_archive: "xar -cf./out.xar ./src",
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
        cpio_extract_with_bsdcpio_pattern_letter_t: "cpio -ift",
        cpio_extract_with_glued_directory_named_t: "cpio -iDt",
        ditto_extract: "ditto -x ./archive.cpio ./out",
        ditto_extract_zip: "ditto -xk ./archive.zip ./out",
        ditto_extract_spaced: "ditto -x -k ./archive.zip ./out",
        xar_extract_keeping_set_id_bits: "xar -x --keep-setuid -f ./archive.xar",
        xar_extract: "xar -x -f ./archive.xar",
        xar_extract_cluster: "xar -xf ./archive.xar",
        xar_extract_into_a_key_folder: "xar -x -f ./a.xar -C ~/.ssh",
        xar_archives_a_key_folder: "xar -c -f ./a.xar ~/.ssh",
        xar_writes_the_archive_over_a_login_file: "xar -c -f ~/.zshrc ./src",
        xar_dumps_the_header_over_a_login_file: "xar -t -f ./a.xar --dump-toc=~/.zshrc",
        xar_adds_a_key_as_a_subdocument: "xar -c -f ./a.xar -s ~/.ssh/id_rsa ./src",
        pax_extract_write_letter_after_a_pattern: "pax -r -f ./archive.tar '*' -w",
        pax_extract_write_letter_after_an_operand: "pax -r ./a -w ./b",
        ptar_extract: "ptar -x -f ./a.tar",
        ptar_extract_cluster: "ptar -xvf ./a.tar",
        ptar_extract_bare: "ptar -x",
        ptar_extract_after_the_archive: "ptar -f ./a.tar -x",
        ptargrep_extracts_matches: "ptargrep pattern ./a.tar",
        ptargrep_list_letter_after_an_operand: "ptargrep pattern ./a.tar -l",
        ptargrep_list_after_separator: "ptargrep -- pattern ./a.tar --list-only",
        cpio_pass_through_fed_without_its_defenses: "find ./src | cpio -p --insecure ./dst",
        cpio_pass_through_long_fed_without_its_defenses: "find ./src | cpio --pass-through --insecure ./dst",
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
        cpio_extract_behind_a_gnu_nonmatching_letter: "cpio -fidm",
        cpio_extract_behind_a_spaced_gnu_nonmatching_letter: "cpio -f -i",
        xip_expand: "xip --expand ./a.xip",
        xip_expand_from_downloads: "xip --expand ~/Downloads/a.xip",
        xip_expand_glued: "xip --expand=./a.xip",
        seven_zip_extract: "7z x ./a.7z",
        seven_zip_extract_flat: "7z e ./a.7z -o./out",
        seven_zip_extract_upper_case: "7z X ./a.7z",
        seven_zip_extract_after_switches: "7zz -y x ./a.7z",
        seven_zip_extract_after_a_spaced_switch_value: "7za -o ./out x ./a.7z",
        cpio_lists_a_remote_archive: "cpio -t -F host:a.cpio",
        cpio_lists_a_remote_archive_glued: "cpio -itFhost:a.cpio",
        cpio_writes_a_remote_archive: "find ./src | cpio -o -O user@host:a.cpio",
        cpio_reads_a_remote_archive_long: "cpio -t --file=host:/tmp/a.cpio",
        cpio_reads_a_remote_archive_behind_a_gnu_nonmatching_letter: "cpio -t -f -F host:a.cpio",
    }

    safe! {
        pax_write_with_a_format_before_the_operands: "pax -w -x ustar -f ./out.tar ./src",
        pax_write_naming_a_second_source_after_the_first: "pax -w -f ./out.tar ./src ./docs",
        ptar_create_from_a_list: "ptar -c -T ./files.txt -f ./a.tar",
        ptar_create_to_stdout: "ptar -c -f - ./src",
        cpio_list_leaving_out_a_pattern: "cpio -it -f '*.o'",
        seven_zip_list: "7z l ./a.7z",
        seven_zip_add_a_file_named_x: "7z a ./out.7z x",
        seven_zip_test: "7z t ./a.7z",
        cpio_lists_an_archive_with_a_colon_below_a_folder: "cpio -t -F ./backups/a:b.cpio",
    }

    denied! {
        pax_write_with_a_late_format_naming_a_key_folder: "pax -w ./src -x ~/.ssh",
        pax_copy_with_late_options_naming_a_key_folder: "pax -rw ./src ./dst -o ~/.ssh",
        ptar_archives_a_credential_file_to_stdout: "ptar -c -f - ~/.aws/credentials",
        ptar_archives_the_files_a_key_lists: "ptar -c -T ~/.ssh/id_rsa -f ./a.tar",
        ptar_writes_the_archive_over_a_login_file: "ptar -c -f ~/.zshrc ./src",
        ptargrep_lists_a_key_archive: "ptargrep -l pattern ~/.ssh/keys.tar",
    }

    proptest! {
        /// BSD pax stops reading options at the first operand, so every word after it is one more
        /// hierarchy to archive or copy. A key folder there is refused whichever option letter is
        /// in front of it, and the same line with the key folder left out is not.
        #[test]
        fn a_word_after_the_first_operand_is_judged_as_an_operand(
            mode in prop::sample::select(vec!["-w", "-wL", "-rw", "-r -w", ""]),
            letter in prop::sample::select(vec!["B", "E", "G", "T", "U", "b", "o", "p", "x", "L", "P", "v"]),
        ) {
            let operands = if mode.contains('r') { "./src ./dst" } else { "./src" };
            let refused = format!("pax {mode} {operands} -{letter} ~/.ssh");
            prop_assert!(!is_safe_command(&refused), "{}", refused);
            let plain = format!("pax {mode} {operands}");
            prop_assert!(is_safe_command(&plain), "{}", plain);
        }

        /// A letter whose arity differs between bsdcpio and GNU cpio (`-f`) hides nothing: put in
        /// front of an extracting cluster, the line is still refused, and in front of a listing it
        /// is still not refused for extracting.
        #[test]
        fn a_letter_only_one_cpio_reads_a_value_for_hides_no_extraction(
            cluster in prop::sample::select(vec!["i", "id", "idm", "iv", "im"]),
            spaced in any::<bool>(),
        ) {
            let line = if spaced { format!("cpio -f -{cluster}") } else { format!("cpio -f{cluster}") };
            prop_assert!(!is_safe_command(&line), "{}", line);
            let tokens: Vec<crate::parse::Token> = format!("cpio -t{cluster} -f x").split_whitespace().map(crate::parse::Token::from_test).collect();
            prop_assert!(!super::extracts(&tokens));
        }

        /// 7-Zip's command letter decides: under any of its names and after any switches, `x` and
        /// `e` extract and are refused, and `l`, `t` and `a` do not extract.
        #[test]
        fn a_seven_zip_extraction_is_refused_whatever_switches_come_first(
            name in prop::sample::select(vec!["7z", "7za", "7zz"]),
            switches in prop::sample::subsequence(vec!["-y", "-bd", "-r", "-o ./out", "-p secret", "-snl"], 0..3),
            command in prop::sample::select(vec!["x", "e", "X", "E", "l", "t", "a"]),
        ) {
            let line = format!("{name} {} {command} ./a.7z", switches.join(" "));
            let tokens: Vec<crate::parse::Token> = line.split_whitespace().map(crate::parse::Token::from_test).collect();
            let extracting = command.eq_ignore_ascii_case("x") || command.eq_ignore_ascii_case("e");
            prop_assert_eq!(super::extracts(&tokens), extracting, "{}", line);
            if extracting {
                prop_assert!(!is_safe_command(&line), "{}", line);
            }
        }

        /// Whatever else is given, a pax, cpio, ditto or xip line that extracts is refused, and one
        /// that only lists is not refused for extracting. A letter after the first operand is a
        /// pattern to a parser that stops there (bsdcpio, or pax under POSIXLY_CORRECT), so a
        /// listing or copy letter only counts in front of the operands.
        #[test]
        fn an_extraction_is_refused_and_a_listing_is_not(
            command in prop::sample::select(vec!["pax -r", "pax -rv", "cpio -i", "cpio -iv", "cpio --extract", "ditto -x", "ditto -xk", "xip --expand"]),
            extra in prop::sample::subsequence(vec!["-v", "-k", "-p e", "-m", "-d"], 0..3),
        ) {
            let extracts = |line: &str| {
                let tokens: Vec<crate::parse::Token> = line.split_whitespace().map(crate::parse::Token::from_test).collect();
                super::extracts(&tokens)
            };
            let line = format!("{command} {} ./archive ./out", extra.join(" "));
            prop_assert!(extracts(&line), "{}", line);
            prop_assert!(!is_safe_command(&line), "{}", line);
            if !command.starts_with("ditto") && !command.starts_with("xip") {
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

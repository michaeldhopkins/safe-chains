//! The backup an in-place edit writes beside the file it edits (`sed -i SUFFIX`, `perl -i SUFFIX`).
//!
//! A backup is a write the command line names only through a suffix, so the editors' resolvers
//! judged the edited file and never the backup. GNU sed and perl replace each `*` in the suffix
//! with the file's name and accept a directory in it, so `sed -i'.git/hooks/*' s/x/x/ pre-commit`
//! writes `.git/hooks/pre-commit`, and a plain suffix appends: `sed -i keys s/x/x/ authorized_`
//! writes `authorized_keys`.

use crate::parse::Token;

/// Every path an in-place edit of `files` may write as a backup, under any suffix the tokens could
/// be giving: glued to `-i` (`-i.bak`, `-pi.bak`), `--in-place=SUFFIX`, or, for BSD sed's
/// `-i SUFFIX`, the token after a bare `-i`. Over-approximating only adds paths to check. A token
/// holding a `/` is not taken as a BSD suffix: there it is the script (`sed -i s/a/b/ f`), and BSD sed
/// appends the suffix to a FILE name, so a suffix with a `/` names nothing a write could land on.
pub(super) fn in_place_backups(tokens: &[Token], files: &[&str], bsd_style: bool) -> Vec<String> {
    let words: Vec<&str> = tokens.iter().skip(1).map(Token::as_str).collect();
    let mut suffixes = Vec::new();
    for (idx, word) in words.iter().enumerate() {
        if *word == "--" {
            break;
        }
        if let Some(s) = word.strip_prefix("--in-place=") {
            suffixes.push(s);
        } else if let Some(cluster) = word.strip_prefix('-').filter(|c| !c.starts_with('-'))
            && let Some(at) = cluster.find('i')
        {
            let glued = &cluster[at + 1..];
            if !glued.is_empty() {
                suffixes.push(glued);
            } else if bsd_style
                && let Some(next) = words.get(idx + 1)
                && !next.contains('/')
            {
                suffixes.push(next);
            }
        }
    }
    let mut out = Vec::new();
    for suffix in suffixes.into_iter().filter(|s| !s.is_empty()) {
        for file in files {
            out.push(backup_of(file, suffix));
        }
    }
    out
}

fn backup_of(file: &str, suffix: &str) -> String {
    if !suffix.contains('*') {
        return format!("{file}{suffix}");
    }
    let (dir, name) = file.rsplit_once('/').map_or(("", file), |(d, n)| (d, n));
    let named = suffix.replace('*', name);
    if named.contains('/') || dir.is_empty() { named } else { format!("{dir}/{named}") }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn toks(line: &str) -> Vec<Token> {
        line.split(' ').map(|w| Token::from_raw(w.to_string())).collect()
    }

    #[test]
    fn each_suffix_spelling_names_its_backup() {
        let cases: &[(&str, &[&str], bool, &[&str])] = &[
            ("sed -i.bak s/x/x/ notes", &["notes"], false, &["notes.bak"]),
            ("sed -i.git/hooks/* s/x/x/ pre-commit", &["pre-commit"], false, &[".git/hooks/pre-commit"]),
            ("sed --in-place=.profil* s/x/x/ e", &["e"], false, &[".profile"]),
            ("sed -i keys s/x/x/ authorized_", &["authorized_"], true, &["authorized_keys"]),
            ("perl -pi.orig -e 1 src/a.rs", &["src/a.rs"], false, &["src/a.rs.orig"]),
            ("perl -pibak_* -e 1 src/a.rs", &["src/a.rs"], false, &["src/bak_a.rs"]),
            ("sed -i s/x/x/ notes", &["notes"], false, &[]),
        ];
        for (line, files, bsd, want) in cases {
            assert_eq!(in_place_backups(&toks(line), files, *bsd), want.iter().map(|s| (*s).to_string()).collect::<Vec<_>>(), "{line}");
        }
    }

    proptest! {
        /// A suffix with no `*` appends; one with a `*` puts the file's name where the star was. A
        /// backup is never the edited file itself unless the suffix is empty, which writes none.
        #[test]
        fn a_backup_is_the_file_renamed_by_its_suffix(file in "[a-z]{1,6}(/[a-z]{1,6}){0,2}", suffix in "[a-z./*]{1,8}") {
            let got = backup_of(&file, &suffix);
            let name = file.rsplit('/').next().unwrap_or(&file);
            if suffix.contains('*') {
                prop_assert!(got.ends_with(&suffix.replace('*', name).rsplit('/').next().unwrap_or("").to_string()));
            } else {
                prop_assert_eq!(got, format!("{file}{suffix}"));
            }
        }
    }
}

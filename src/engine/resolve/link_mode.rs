//! Whether a command acts on a link named as its operand, or on what the link leads to.
//!
//! `cp -R link x` copies the link, `rsync -a link x` stores it, and `chmod -h` and `chflags -h`
//! change the link's own mode or flags: none of them opens the link's target. Their source operands
//! are placed without following the last component, as a rebind's are. A flag that makes them follow
//! links (`cp -L`, `rsync -L`) is looked for anywhere in a flag word, and a long one by any prefix,
//! so a misread flag can only make the command follow. A flag that makes them act on the link is
//! read word by word, stopping at a flag that takes a value.

struct Rule {
    by_default: bool,
    itself_short: &'static [u8],
    itself_long: &'static [&'static str],
    follow_short: &'static [u8],
    follow_long: &'static [&'static str],
    valued_short: &'static [u8],
    valued_long: &'static [&'static str],
}

const NONE: Rule = Rule {
    by_default: false,
    itself_short: b"",
    itself_long: &[],
    follow_short: b"",
    follow_long: &[],
    valued_short: b"",
    valued_long: &[],
};

fn rule(name: &str) -> Option<Rule> {
    Some(match name {
        "cp" => Rule {
            itself_short: b"Rra",
            itself_long: &["--recursive", "--archive"],
            follow_short: b"LH",
            follow_long: &["--dereference"],
            valued_short: b"St",
            valued_long: &["--suffix", "--target-directory"],
            ..NONE
        },
        "chmod" => {
            Rule { itself_short: b"h", itself_long: &["--no-dereference"], follow_short: b"LH", follow_long: &["--dereference"], ..NONE }
        }
        "chflags" => Rule { itself_short: b"h", follow_short: b"LH", ..NONE },
        "rsync" => {
            Rule { by_default: true, follow_short: b"Lk", follow_long: &["--copy-links", "--copy-unsafe-links", "--copy-dirlinks"], ..NONE }
        }
        _ => return None,
    })
}

/// Whether the command `words` (its name first) acts on a link named as a source operand.
pub(crate) fn acts_on_link(words: &[&str]) -> bool {
    let Some(rule) = words.first().and_then(|w| rule(w.rsplit('/').next().unwrap_or(w))) else {
        return false;
    };
    !follows(&rule, &words[1..]) && (rule.by_default || sets_itself(&rule, &words[1..]))
}

/// Whether the command `words` follows links while it walks a tree (`cp -RL`, `rsync -L`).
pub(crate) fn follows_links(words: &[&str]) -> bool {
    words
        .first()
        .and_then(|w| rule(w.rsplit('/').next().unwrap_or(w)))
        .is_some_and(|rule| follows(&rule, &words[1..]))
}

/// Whether `rsync` writes through links to folders already in its destination (`-K`).
pub(crate) fn keeps_dest_links(words: &[&str]) -> bool {
    let keep = Rule { follow_short: b"K", follow_long: &["--keep-dirlinks"], ..NONE };
    words.first().is_some_and(|w| w.rsplit('/').next() == Some("rsync")) && follows(&keep, &words[1..])
}

/// What a command's link flags say about its operands: whether it acts on a link named as a
/// source, follows links while it walks, and writes through links to folders in its destination.
#[derive(Clone, Copy)]
pub(crate) struct Links {
    pub on_link: bool,
    pub follow: bool,
    pub dest: DestLinks,
}

/// Whether a command given a link acts on the link itself or on what it leads to.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum LinkUse {
    #[default]
    Itself,
    Target,
}

/// What a copy does with a link to a folder already in its destination: replaces it, or writes
/// through it into the folder (`rsync -K`).
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum DestLinks {
    Replaced,
    WrittenThrough,
}

impl Links {
    pub(crate) fn of(tokens: &[crate::parse::Token]) -> Links {
        let words: Vec<&str> = tokens.iter().map(crate::parse::Token::as_str).collect();
        Links {
            on_link: acts_on_link(&words),
            follow: follows_links(&words),
            dest: if keeps_dest_links(&words) { DestLinks::WrittenThrough } else { DestLinks::Replaced },
        }
    }
}

/// Runs `judge` on the link itself when the command `tokens` acts on it, else as usual.
pub(crate) fn judged<T>(tokens: &[crate::parse::Token], judge: impl FnOnce() -> T) -> T {
    if acts_on_link(&tokens.iter().map(crate::parse::Token::as_str).collect::<Vec<_>>()) {
        crate::pathctx::on_the_link(judge)
    } else {
        judge()
    }
}

fn follows(rule: &Rule, words: &[&str]) -> bool {
    words.iter().take_while(|w| **w != "--").any(|w| match w.strip_prefix("--") {
        Some(long) => {
            let name = long.split('=').next().unwrap_or(long);
            rule.follow_long.iter().any(|f| f[2..].starts_with(name) && name.len() >= 3)
        }
        None => w.len() > 1 && w.starts_with('-') && w.bytes().any(|b| rule.follow_short.contains(&b)),
    })
}

fn sets_itself(rule: &Rule, words: &[&str]) -> bool {
    let mut rest = words.iter();
    while let Some(word) = rest.next() {
        if *word == "--" {
            return false;
        }
        if let Some(long) = word.strip_prefix("--") {
            let (name, glued) = long.split_once('=').map_or((long, false), |(name, _)| (name, true));
            if rule.itself_long.iter().any(|f| f[2..] == *name) {
                return true;
            }
            if !glued && rule.valued_long.iter().any(|f| f[2..] == *name) {
                rest.next();
            }
            continue;
        }
        let Some(cluster) = word.strip_prefix('-').filter(|c| !c.is_empty()) else { continue };
        for (at, b) in cluster.bytes().enumerate() {
            if rule.itself_short.contains(&b) {
                return true;
            }
            if rule.valued_short.contains(&b) {
                if at + 1 == cluster.len() {
                    rest.next();
                }
                break;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::acts_on_link;

    fn on(line: &str) -> bool {
        acts_on_link(&line.split_whitespace().collect::<Vec<_>>())
    }

    #[test]
    fn a_recursive_copy_acts_on_the_link() {
        for line in ["cp -R l x", "cp -a l x", "/bin/cp -pR l x", "cp --recursive l x", "chmod -h 644 l", "chflags -h uchg l"] {
            assert!(on(line), "{line}");
        }
        assert!(on("rsync -a l x"));
    }

    #[test]
    fn a_follow_flag_anywhere_wins() {
        for line in ["cp -RL l x", "cp -R -H l x", "cp -a --deref l x", "rsync -aL l x", "rsync -a --copy-unsafe l x", "rsync -rk l x"] {
            assert!(!on(line), "{line}");
        }
    }

    #[test]
    fn a_value_is_not_read_as_a_flag() {
        for line in ["cp -tR l", "cp -t -R l", "cp --suffix -R l x", "cp -S -a l x", "cp l x", "cp -- -R x"] {
            assert!(!on(line), "{line}");
        }
        assert!(on("cp --suffix=.b -R l x"));
        assert!(!on("mv l x"));
    }

    mod properties {
        use super::super::{acts_on_link, follows_links};
        use proptest::prelude::*;

        const FLAGS: [&str; 14] = ["-R", "-a", "-r", "-p", "-t", "-S", "--suffix", "--recursive", "-h", "-v", "x", "l", "--", "-n"];

        fn follow_flags(name: &str) -> &'static [&'static str] {
            match name {
                "rsync" => &["-L", "-k", "-aL", "--copy-links", "--copy-unsafe"],
                "chmod" => &["-L", "-H", "-RH", "--dereference", "--deref"],
                _ => &["-L", "-H", "-aL", "--dereference", "--deref"],
            }
        }

        proptest! {
            #[test]
            fn a_follow_flag_before_the_operands_end_always_follows(
                name in prop::sample::select(vec!["cp", "rsync", "chmod", "/bin/cp"]),
                before in prop::collection::vec(prop::sample::select(FLAGS.to_vec()), 0..5),
                pick in 0..5usize,
            ) {
                let mut words = vec![name];
                words.extend(before.iter().take_while(|w| **w != "--"));
                words.push(follow_flags(name.rsplit('/').next().unwrap_or(name))[pick]);
                words.push("l");
                prop_assert!(follows_links(&words), "{:?}", words);
                prop_assert!(!acts_on_link(&words), "{:?}", words);
            }
        }
    }
}

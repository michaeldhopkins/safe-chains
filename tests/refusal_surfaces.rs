//! Every surface that answers "why didn't this auto-approve" must give the same answer.
//!
//! There are three, and a person reaching for one of them is asking the same question each time:
//! the hook's injected context (what the agent reads), `--explain` (what `--help` tells a human to
//! run), and `--suggest` (what a human reaches for when they want to make it work). They had
//! drifted apart on the case that matters most — a refusal caused by a PATH:
//!
//!   * the hook named the path, the working directory it sits outside of, the `[[grant]]` remedy
//!     and the page documenting it;
//!   * `--explain` said "safe-chains approves commands it has researched and has no opinion about
//!     the rest" and then gave advice about splitting chains — no path, no remedy;
//!   * `--suggest` said the cause was "a flag, subcommand, or path" and linked custom-commands,
//!     the one page that says nothing about paths.
//!
//! So the two surfaces a human is pointed at both hid the answer the hook already had, and a reader
//! who used them concluded safe-chains had no way to declare a readable directory — while
//! `[[grant]]` was documented the whole time. The invariant is that all three name the path they
//! refused over and where the remedy lives.
use std::io::Write;
use std::process::{Command, Stdio};

const HOW_IT_WORKS: &str = "https://www.michaeldhopkins.com/docs/safe-chains/how-it-works.html";

/// Commands safe-chains RECOGNIZES that are refused over a path: the path each reaches, and
/// whether a `[[grant]]` is the remedy for it.
///
/// Recognized on purpose: an unknown command is a different refusal with a different remedy (a
/// command definition, which is what `--suggest` generates and custom-commands.html documents).
/// These are the case where the command is fine and the location is not.
///
/// `grantable = false` is not an exemption, it is the other half of the contract. `/etc/sudoers`
/// decides who may log in, so its write face is frozen and no grant reopens it — every surface has
/// to say THAT rather than offer a remedy that cannot work. A guard that demanded the grant
/// sentence everywhere would be pressure to print it there too.
/// The shapes are chosen to still REFUSE. A plain named read outside the workspace is ordinary and
/// approved — that is the whole point of the model — so the reads here are the two kinds that are
/// not: an unbounded sweep, which names no file for the shield to test, and a credential store.
const REACHES: &[(&str, &str, bool)] = &[
    ("cat ~/.ssh/id_rsa", "/.ssh/id_rsa", true),
    ("echo x > /other/repo/x.rs", "/other/repo/x.rs", true),
    ("grep -r TODO /other/repo", "/other/repo", true),
    ("tee /etc/sudoers", "/etc/sudoers", false),
];

/// A HOME of our own: all three surfaces read `~/.claude/settings.json` and
/// `~/.config/safe-chains.toml`, and a guard that inherited them would be measuring the config of
/// whoever ran the suite rather than safe-chains.
fn bare_home() -> tempfile::TempDir {
    tempfile::tempdir().expect("tempdir")
}

fn run(args: &[&str], home: &std::path::Path, stdin_payload: Option<&str>) -> String {
    let mut child = Command::new(env!("CARGO_BIN_EXE_safe-chains"))
        .args(args)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .env("HOME", home)
        .stdin(if stdin_payload.is_some() { Stdio::piped() } else { Stdio::null() })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn safe-chains");
    if let Some(payload) = stdin_payload {
        child
            .stdin
            .take()
            .expect("stdin was piped")
            .write_all(payload.as_bytes())
            .expect("write the hook payload");
    }
    let out = child.wait_with_output().expect("wait for safe-chains");
    // Both streams: the hook answers on stdout, `--suggest` on stderr, `--explain` on stdout.
    // Which stream carries the answer is not what this guard is about.
    format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr))
}

#[test]
fn every_why_surface_names_the_path_and_the_remedy() {
    let home = bare_home();
    for (command, reached, grantable) in REACHES {
        let payload = serde_json::json!({
            "tool_name": "Bash",
            "tool_input": {"command": command},
            "cwd": env!("CARGO_MANIFEST_DIR"),
        })
        .to_string();

        for (surface, text) in [
            ("hook", run(&[], home.path(), Some(&payload))),
            ("--explain", run(&["--explain", command], home.path(), None)),
            ("--suggest", run(&["--suggest", command], home.path(), None)),
        ] {
            assert!(
                text.contains(reached),
                "`{surface}` on `{command}` never names the path it refused over \
                 (`{reached}`):\n{text}"
            );
            assert_eq!(
                text.contains("~/.config/safe-chains.toml"),
                *grantable,
                "`{surface}` on `{command}`: grant remedy offered={}, expected={grantable}\n{text}",
                text.contains("~/.config/safe-chains.toml")
            );
            assert!(
                text.contains(HOW_IT_WORKS),
                "`{surface}` on `{command}` links somewhere other than the page documenting \
                 the path model and `[[grant]]`:\n{text}"
            );
        }
    }
}

/// The other direction, so the fix above is a routing change rather than a blanket redirect.
///
/// A recognized command held back by its GRAMMAR — a subcommand or flag safe-chains doesn't
/// approve, with no path in sight — still belongs on custom-commands.html, which is where
/// overriding a command's definition is documented. Without this, "always link how-it-works" would
/// pass the guard above and send every grammar refusal to a page about directories.
#[test]
fn a_grammar_refusal_still_points_at_custom_commands() {
    let home = bare_home();
    for command in ["git push origin main", "npm install left-pad"] {
        let text = run(&["--suggest", command], home.path(), None);
        assert!(
            text.contains("custom-commands.html"),
            "`--suggest` on `{command}` is a grammar refusal and must still point at \
             custom-commands:\n{text}"
        );
        assert!(
            !text.contains(HOW_IT_WORKS),
            "`--suggest` on `{command}` reaches no path, so it must not offer the grant \
             remedy:\n{text}"
        );
    }
}

#[test]
fn suggest_offers_the_snapshot_flag_only_where_the_flag_would_be_approved() {
    let home = bare_home();
    let command = "jj -R ~/.claude log";
    let default = run(&["--suggest", command], home.path(), None);
    assert!(default.contains("--ignore-working-copy"), "the default level approves the flagged read:\n{default}");
    let paranoid = run(&["--level", "paranoid", "--suggest", command], home.path(), None);
    assert!(!paranoid.contains("--ignore-working-copy"), "paranoid refuses the flagged read too:\n{paranoid}");
    assert!(paranoid.contains("No grant changes this"), "{paranoid}");
}

const KNOWN_PLACES: &[&str] =
    &[", a credential store", ", a raw device", "safe-chains reads its own permissions", "decides who may log in"];

const REACHES_A_KNOWN_PLACE: &[&str] = &[
    "cat ~/.ssh/id_rsa", "cat ~/.aws/credentials", "cat ~/.gnupg/secring.gpg", "cat /etc/shadow", "cat ~root/.bashrc",
    "cat ~/Library/Keychains/login.keychain-db", "grep -r x ~/.ssh", "cp ~/.aws/credentials ./c", "tar -cf x.tar ~/.ssh",
    "mv ~/.ssh/id_rsa /tmp/x", "ln -s x ~/.ssh/id_rsa",
];

const REFUSAL_OWED: &[&str] = &["rm ~/.aws/credentials", "touch ~/.ssh/x", "sed -i s/a/b/ ~/.ssh/config", "cp x ~/.ssh/authorized_keys"];

fn without_bounds(text: &str) -> String {
    text.lines()
        .map(|line| line.split(" (allowed up to:").next().unwrap_or(line))
        .collect::<Vec<_>>()
        .join("\n")
}

fn explained(command: &str, home: &std::path::Path) -> String {
    let text = run(&["--explain", command], home, None);
    assert!(text.contains("resolved profile:"), "`{command}` prints no profile to check:\n{text}");
    assert!(KNOWN_PLACES.iter().any(|m| text.contains(m)), "`{command}` no longer names a known place:\n{text}");
    text
}

#[test]
fn explain_never_says_there_is_no_rule_for_a_place_it_names() {
    let home = bare_home();
    for command in REACHES_A_KNOWN_PLACE {
        let text = explained(command, home.path());
        assert!(
            !without_bounds(&text).contains("no rule for"),
            "`{command}` is said to reach a place without a rule while its Why names the place:\n{text}"
        );
    }
}

#[test]
fn a_write_to_a_named_place_says_no_rule_only_in_its_refusal_until_that_line_has_words() {
    let home = bare_home();
    for command in REFUSAL_OWED {
        let text = explained(command, home.path());
        let said = without_bounds(&text);
        let lines: Vec<&str> = said.lines().filter(|l| l.contains("no rule for")).collect();
        assert_eq!(
            lines,
            vec!["    a place safe-chains has no rule for"],
            "`{command}`: move it to REACHES_A_KNOWN_PLACE once its refusal no longer says this:\n{text}"
        );
    }
}

const NO_NAMED_PLACE: &[(&str, &str)] = &[
    ("rm /opt/y", "outside the working directory"),
    ("cat ~/$X", "is built by an interpolation"),
    ("rm -rf /tmp/foo /opt/y", "a temporary directory that is not this session's scratchpad"),
    ("cat /tmp/x /opt/y", "resolved profile:"),
];

#[test]
fn a_place_the_why_line_does_not_name_is_still_said_to_have_no_rule() {
    let home = bare_home();
    for (command, why) in NO_NAMED_PLACE {
        let text = run(&["--explain", command], home.path(), None);
        assert!(text.contains(why), "`{command}` no longer says `{why}`:\n{text}");
        assert!(!KNOWN_PLACES.iter().any(|m| text.contains(m)), "`{command}` now names a known place:\n{text}");
        let does = text
            .lines()
            .filter(|l| l.trim_start().starts_with("does "))
            .any(|l| l.ends_with("a place safe-chains has no rule for"));
        assert!(does, "`{command}` lost the words for a place with no rule:\n{text}");
    }
}

#[test]
fn which_place_the_why_line_names_follows_the_order_of_the_paths() {
    let home = bare_home();
    let named_first = without_bounds(&explained("cat ~/.ssh/id_rsa /opt/y", home.path()));
    assert!(!named_first.contains("no rule for"), "see TODO.md, the place a Why line names:\n{named_first}");
    let named_second = run(&["--explain", "cat /opt/y ~/.ssh/id_rsa"], home.path(), None);
    assert!(named_second.contains("outside the working directory"), "{named_second}");
    let rows: Vec<&str> = named_second.lines().filter(|l| l.trim_start().starts_with("does ")).collect();
    assert_eq!(rows.len(), 2, "{named_second}");
    assert!(
        rows.iter().all(|l| l.ends_with("a place safe-chains has no rule for")),
        "see TODO.md, the place a Why line names:\n{named_second}"
    );
}

const GUIDANCE_BRANCHES: &[&str] = &["cat /etc/shadow && cat /etc/shadow", "cd /tmp && cat /etc/shadow", "ls && cat /etc/shadow"];

const AGENT_LESSONS: &[&str] = &["This is not a block", "already run", "Next time", "Bundling them", "Nothing to change"];

#[test]
fn explain_keeps_the_lessons_addressed_to_an_agent_out_of_what_a_person_reads() {
    let home = bare_home();
    for command in GUIDANCE_BRANCHES {
        let agent = safe_chains::cst::explain(command).render();
        assert!(agent.contains("This is not a block"), "`{command}` no longer reaches a guidance branch:\n{agent}");
        let person = run(&["--explain", command], home.path(), None);
        assert!(person.contains("did not auto-approve"), "`{command}` printed no verdict:\n{person}");
        for lesson in AGENT_LESSONS {
            assert!(!person.contains(lesson), "`--explain` on `{command}` tells a person `{lesson}`:\n{person}");
        }
    }
}

#[test]
fn explain_settles_the_snapshot_reason_the_way_suggest_does() {
    let home = bare_home();
    let command = "jj -R ~/.claude log";
    let default = run(&["--explain", command], home.path(), None);
    assert!(default.contains("Add --ignore-working-copy"), "{default}");
    let paranoid = run(&["--level", "paranoid", "--explain", command], home.path(), None);
    assert!(!paranoid.contains("Add --ignore-working-copy"), "{paranoid}");
    assert!(paranoid.contains("safe-chains reads its permissions"), "{paranoid}");
}

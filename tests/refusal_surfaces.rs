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
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
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

//! The `--suggest` command: everything the flag prints, and where it says to put it.
//!
//! The binary's half, not the library's. `safe_chains::suggest` decides WHAT a command needs — it
//! analyses the command and generates the entry; this decides what the person reads and which exit
//! code they get, which is why it may call `process::exit` and the library may not.
//!
//! Split out of `main.rs` when the file-length gate went in: `main.rs` was over its limit, and
//! `--suggest` is a self-contained surface that had no reason to be in the same file as the hook
//! dispatch and the gate-mode verdict.

use std::process;

use crate::HOW_IT_WORKS_URL;

/// `--suggest`: help a user support a command safe-chains doesn't recognize. OPT-IN — reached only
/// by the explicit flag, never mentioned in any deny/hook output. Writes/updates a project
/// `.safe-chains.toml` and prints the `[[trusted]]` pin the user hand-adds to ~/ to approve it.
pub fn run_suggest(command: &str) -> ! {
    use safe_chains::suggest::{self, Outcome};
    const DOCS: &str = "https://www.michaeldhopkins.com/docs/safe-chains/custom-commands.html";

    match suggest::analyze(command) {
        Outcome::AlreadyAllowed => {
            println!("safe-chains already auto-approves this command. Nothing to add.");
            process::exit(0);
        }
        Outcome::Unparseable => {
            eprintln!(
                "safe-chains couldn't parse this command, so a command definition can't help. \
                 Check the quoting."
            );
            process::exit(1);
        }
        Outcome::RecognizedButDenied { names } => {
            // A recognized command can be held back by its own grammar (a flag, a subcommand) or by
            // a PATH it reaches, and those have different remedies on different pages. Saying "a
            // flag, subcommand, or path" and then linking custom-commands.html sent every path case
            // to the one page that says nothing about paths: a reader looking for a way to declare
            // an extra readable directory found none there and concluded safe-chains had none,
            // while `[[grant]]` was documented on how-it-works.html the whole time.
            //
            // So when the reach check can name the path, say which one and where the remedy lives.
            // `--suggest` still generates nothing either way — the command is already known.
            match safe_chains::workspace_overreach(command) {
                Some((path, reason)) => eprintln!(
                    "Every command here is one safe-chains already recognizes ({}). It isn't \
                     auto-approving because of the PATH it reaches, not because the command is \
                     unknown, so --suggest won't generate an override: {}. {HOW_IT_WORKS_URL}",
                    names.join(", "),
                    reason.message(&path)
                ),
                None => eprintln!(
                    "Every command here is one safe-chains already recognizes ({}). It isn't \
                     auto-approving because of HOW it's used — a flag or subcommand — not because \
                     the command is unknown, so --suggest won't generate an override. See {DOCS}.",
                    names.join(", ")
                ),
            }
            process::exit(1);
        }
        Outcome::Generated { entries, also_recognized } => {
            emit_suggestion(&entries, &also_recognized);
        }
    }
}

/// Locate the project `.safe-chains.toml` (nearest one walking up from the cwd), or the path where
/// one would be created in the cwd if none exists yet.
fn repo_config_path() -> std::path::PathBuf {
    let Ok(start) = std::env::current_dir() else {
        return std::path::PathBuf::from(REPO_FILENAME);
    };
    repo_config_path_from(&start)
}

const REPO_FILENAME: &str = ".safe-chains.toml";

/// The project `.safe-chains.toml` for a run starting at `start`: the nearest one at or above
/// `start` but NEVER above the project root, else the path where one would be created at that root.
///
/// The confinement is the point. This used to walk to the filesystem root and write to the first
/// `.safe-chains.toml` it met, so a `~/.safe-chains.toml` captured every `--suggest` run anywhere
/// beneath `$HOME` — the same command writing a different file depending on where an ancestor config
/// happened to sit, with nothing on the command line to say which. `--suggest`'s output asks for a
/// trust decision, so the file it offers has to be one the reader can predict: the project whose
/// commands were just analysed. With no project root there is no boundary to search within, so it
/// does not search at all.
fn repo_config_path_from(start: &std::path::Path) -> std::path::PathBuf {
    let mut dir = start.to_path_buf();
    let root = loop {
        if dir.join(".git").exists() || dir.join(".jj").exists() {
            break Some(dir);
        }
        if !dir.pop() {
            break None;
        }
    };
    let Some(root) = root else {
        return start.join(REPO_FILENAME);
    };

    let mut dir = start.to_path_buf();
    loop {
        let candidate = dir.join(REPO_FILENAME);
        if candidate.is_file() {
            return candidate;
        }
        if dir == root || !dir.pop() {
            break;
        }
    }
    root.join(REPO_FILENAME)
}

/// Print the entry a command would need, where it goes, and the pin that activates it. Writes
/// NOTHING.
///
/// It used to write the file and report "Added this to …", which is not what a flag called
/// `--suggest` says it does. The name is the part people read, and a tool that edits a repo file
/// when its name promises advice is a misnomer with consequences: the obvious way to find out what
/// `--suggest` says was to run it, and running it changed the project.
///
/// The write bought little even when it worked. The generated file does nothing until the user
/// pastes a `[[trusted]]` pin into `~/.config/safe-chains.toml` by hand — so the flow was never
/// hands-off, and the one step it automated was the one the user could see and check. Printing the
/// block leaves the same two steps, both explicit.
///
/// The hash in the pin is of the target file with exactly this block added, so it is correct for a
/// verbatim paste; the message says to recompute it otherwise, which is the same instruction that
/// already applied to any later edit.
fn emit_suggestion(entries: &[safe_chains::suggest::GeneratedEntry], also_recognized: &[String]) -> ! {
    use safe_chains::suggest;

    let target = repo_config_path();
    let existing = std::fs::read_to_string(&target).unwrap_or_default();
    let merged = suggest::merged_content(&existing, entries);
    let hash = suggest::config_hash(merged.as_bytes());
    let block = suggest::render_toml(entries);

    let dir = target.parent().unwrap_or_else(|| std::path::Path::new("."));
    let canonical = std::fs::canonicalize(dir).unwrap_or_else(|_| dir.to_path_buf());
    let pin = suggest::pin_block(&canonical.to_string_lossy(), &hash);

    // This path comes from the CWD, so a directory name chosen by whoever wrote the project picks
    // the bytes. `--suggest` is exactly what someone runs inside an unfamiliar checkout, and its
    // output ASKS FOR A TRUST DECISION — "add this to ~/.config/safe-chains.toml". Interpolated raw,
    // a directory named with newlines printed a second, forged `[[trusted]] path = "/"` block above
    // the real one, in our voice. `pin_block` escapes its own TOML; the prose around it did not.
    let shown = safe_chains::sanitize_display(&target.display().to_string());

    // Appending to a file that is not valid TOML produces a file that is still not valid TOML —
    // and safe-chains cannot load one, so the block would never take effect. Reporting "Added
    // this to …" and handing over a pin for it sends the reader off to approve something that
    // cannot work, and the hash pins the broken content. Refuse and say what is wrong instead.
    if !existing.trim().is_empty()
        && let Err(e) = toml::from_str::<toml::Value>(&existing)
    {
        eprintln!(
            "{shown} isn't valid TOML ({e}), so adding to it would leave a file safe-chains can't \
             load. Fix or move that file, then re-run. The block to add is:\n\n{block}"
        );
        process::exit(1);
    }

    println!("Add this to {shown}:\n\n{block}");
    println!(
        "That file does nothing until you approve it. Add this to ~/.config/safe-chains.toml \
         (which safe-chains never edits):\n\n{pin}"
    );
    println!(
        "The level defaults to \"SafeWrite\". Edit it to \"SafeRead\" (runs code, no \
         artifacts) or \"Inert\" (read-only) if that fits the tool. The hash above is of {shown} \
         with exactly the block above added; if you change either, recompute it with \
         `shasum -a 256 {shown}` and update the pin."
    );
    if !also_recognized.is_empty() {
        println!(
            "\nHeads up: this command also uses commands safe-chains already recognizes \
             ({}). The entry above only covers the unrecognized one(s), so if the whole \
             command still isn't approved, one of those is why.",
            also_recognized.join(", ")
        );
    }
    process::exit(0);
}

use std::io::{self, IsTerminal, Read, Write};
use std::process;

use clap::{CommandFactory, Parser};

use safe_chains::cli::{Cli, Subcommand};
use safe_chains::targets::{self, HookFormat};
use safe_chains::verdict::{SafetyLevel, Verdict};

fn print_docs() {
    let docs = safe_chains::docs::all_command_docs();
    print!("{}", safe_chains::docs::render_markdown(&docs));
}

fn run_cli(
    command: &str,
    threshold: SafetyLevel,
    engine_level: Option<&'static safe_chains::engine::level::Level>,
) {
    let verdict = safe_chains::command_verdict_ceilinged(command, threshold, engine_level);
    process::exit(i32::from(!verdict.is_allowed()));
}

fn run_explain(command: &str) -> ! {
    let explanation = safe_chains::cst::explain(command);
    print!("{}", explanation.render());
    // The facet breakdown is CLI-only. `render()` also feeds the hook's injected context, where an
    // agent mid-chain needs the verdict and nothing else; a 27-axis dump there would be noise it
    // cannot act on. Someone who typed `--explain` is asking why, so they get why.
    print!("{}", safe_chains::facet_breakdown(command));
    process::exit(i32::from(!explanation.is_allowed()));
}

/// `--suggest`: help a user support a command safe-chains doesn't recognize. OPT-IN — reached only
/// by the explicit flag, never mentioned in any deny/hook output. Writes/updates a project
/// `.safe-chains.toml` and prints the `[[trusted]]` pin the user hand-adds to ~/ to approve it.
fn run_suggest(command: &str) -> ! {
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
            eprintln!(
                "Every command here is one safe-chains already recognizes ({}). It isn't \
                 auto-approving because of HOW it's used — a flag, subcommand, or path — not because \
                 the command is unknown, so --suggest won't generate an override. See {DOCS}.",
                names.join(", ")
            );
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
    const REPO_FILENAME: &str = ".safe-chains.toml";
    let Ok(start) = std::env::current_dir() else {
        return std::path::PathBuf::from(REPO_FILENAME);
    };
    let mut dir = start.clone();
    loop {
        let candidate = dir.join(REPO_FILENAME);
        if candidate.is_file() {
            return candidate;
        }
        if !dir.pop() {
            return start.join(REPO_FILENAME);
        }
    }
}

fn emit_suggestion(
    entries: &[safe_chains::suggest::GeneratedEntry],
    also_recognized: &[String],
) -> ! {
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

    match std::fs::write(&target, &merged) {
        Ok(()) => {
            println!("Added this to {shown}:\n\n{block}");
            println!(
                "That file does nothing until you approve it. Add this to ~/.config/safe-chains.toml \
                 (which safe-chains never edits):\n\n{pin}"
            );
            println!(
                "The level defaults to \"SafeWrite\". Edit it to \"SafeRead\" (runs code, no \
                 artifacts) or \"Inert\" (read-only) if that fits the tool. Any later edit to \
                 {shown} changes its hash: recompute with `shasum -a 256 {shown}` and update the pin."
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
        Err(e) => {
            eprintln!(
                "Couldn't write {shown} ({e}). Add this block to a `.safe-chains.toml` yourself:\n\n{block}\n\
                 then pin it in ~/.config/safe-chains.toml:\n\n{pin}"
            );
            process::exit(1);
        }
    }
}

fn run_setup(name: Option<String>, auto_detect: bool) -> ! {
    let Some(home) = std::env::var_os("HOME") else {
        eprintln!("Error: HOME environment variable not set");
        process::exit(1);
    };
    let home = std::path::PathBuf::from(home);

    if auto_detect {
        let detected = targets::detect_installed(&home);
        if detected.is_empty() {
            eprintln!(
                "No supported tools detected on this machine. Run with --list-tools to see candidates."
            );
            process::exit(1);
        }
        let mut any_failed = false;
        for target in detected {
            match target.install(&home) {
                Ok(outcome) => println!("{}", outcome.message(target.display_name())),
                Err(e) => {
                    eprintln!("{}: {e}", target.display_name());
                    any_failed = true;
                }
            }
        }
        process::exit(i32::from(any_failed));
    }

    let target_name = name.as_deref().unwrap_or("claude");
    let Some(target) = targets::find(target_name) else {
        eprintln!("Unknown tool: {target_name}. Run with --list-tools to see candidates.");
        process::exit(1);
    };
    match target.install(&home) {
        Ok(outcome) => {
            println!("{}", outcome.message(target.display_name()));
            process::exit(0);
        }
        Err(e) => {
            eprintln!("{}: {e}", target.display_name());
            process::exit(1);
        }
    }
}

fn run_list_tools() -> ! {
    for target in targets::registry() {
        println!("{}\t{}", target.name(), target.display_name());
    }
    process::exit(0);
}

fn run_hook_for(target_name: &str) -> ! {
    let Some(target) = targets::find(target_name) else {
        eprintln!("Unknown tool: {target_name}. Run with --list-tools to see candidates.");
        process::exit(1);
    };
    let Some(format) = target.hook_format() else {
        eprintln!(
            "{}: this target does not use a runtime hook (config-only integration).",
            target.display_name()
        );
        process::exit(1);
    };
    run_hook_format(format, target_name);
}

/// The "outside the working directory" clause, NAMING the cwd when the harness reported one — so a
/// directory MISMATCH (the agent was launched from the wrong repo, a common and easy-to-forget
/// mistake) is visible in the message. Without naming it, the user can't tell "I meant to be
/// elsewhere" from "this command genuinely overreaches".
fn run_hook_format(format: &dyn HookFormat, target_name: &str) -> ! {
    // Claude's own permission files are trust ONLY when Claude is the harness being served.
    // Every other target gets safe-chains' own classification and nothing borrowed.
    if target_name == "claude" {
        safe_chains::trust_claude_config();
    }
    let mut buf = String::new();
    if io::stdin().read_to_string(&mut buf).is_err() {
        process::exit(0);
    }

    let Ok(input) = format.parse_input(&buf) else {
        process::exit(0);
    };


    // HP-19: install the harness cwd/root so relative paths resolve against the real
    // directory for the whole evaluation (verdict and explainer). Most harnesses send `cwd`
    // but no distinct project `root`; default root to cwd so the workspace boundary (and the
    // "reaches above your workspace" nudge) engages with the one directory we do know.
    let _ctx = safe_chains::pathctx::enter(safe_chains::pathctx::PathCtx {
        cwd: input.cwd.clone(),
        root: input.root.clone().or_else(|| input.cwd.clone()),
        session_id: input.session_id.clone(),
    });
    // The auto-approve ceiling comes from the write-protected user config (`~/.config/safe-chains.toml`,
    // `level = "…"`). Absent → the default developer band. An UPPER level (network-admin) RAISES it —
    // git push / bulk-object-read become reachable; a LOWER level (reader/editor) TIGHTENS it — a read-
    // only or no-destroy plan, gating writes the default would allow. Both funnel through the same
    // `<= threshold` gate as the CLI's `--level`. The pathctx (cwd/root) is already installed above.
    let (threshold, engine_level) = safe_chains::configured_hook_ceiling();
    let verdict = safe_chains::command_verdict_ceilinged(&input.command, threshold, engine_level);
    // `respond` owns the grant rule, including that a BLANK command grants nothing: it classifies
    // as inert, but inert-about-nothing is not something to approve. Routed through the shared seam
    // so the rule cannot be bypassed by reaching for `render_response` directly.
    if let Some(response) = targets::respond(format, &input.command, verdict) {
        let _ = io::stdout().write_all(response.stdout.as_bytes());
        process::exit(response.exit_code);
    }
    if verdict.is_allowed() {
        process::exit(0); // safe, but not grantable (blank) — abstain silently
    }

    // Coverage fallback: the built-in/pattern classifier (also honoring the user's own
    // `~/.claude/settings.json` `permissions.allow` grants), computed UNDER the configured engine level
    // so a covered command respects that level's rule (an `editor` plan's forbidden worktree destroy
    // classifies denied here too). Its REAL level in `overall` is then held under the SAME `<=
    // threshold` ceiling — so a lower `level` (reader) can't have the write it just gated re-admitted.
    // At the default band both are no-ops (no engine level, coverage `<= SafeWrite`).
    let explanation = safe_chains::explain_with_coverage_at_level(&input.command, engine_level);

    if let Verdict::Allowed(level) = explanation.overall
        && level <= threshold
    {
        let response = format.render_response(Verdict::Allowed(level));
        let _ = io::stdout().write_all(response.stdout.as_bytes());
        process::exit(response.exit_code);
    }

    // GATED command. What the hook emits depends on the harness's capabilities
    // (docs/design/harness-capability-model.md):
    //  - Deny (e.g. Codex): no interactive approval, so VETO it (silence would just run it — its
    //    sandbox even permits broad reads). Escape valve is a config-level exception.
    //  - Ask  (e.g. Antigravity): escalate to an in-the-moment human prompt.
    //  - Defer (e.g. Claude): fall through to context/nudge/silent so the harness's own prompt decides.
    // When the command was gated because it reaches OUTSIDE the workspace, fold that specific reason
    // into the Deny/Ask message so the human/model sees *why* — Defer surfaces it via render_context
    // below, but Deny/Ask exit here, so without this they'd get only the generic reason.
    const DOCS_URL: &str = "https://www.michaeldhopkins.com/docs/safe-chains/how-it-works.html";
    let overreach = safe_chains::workspace_overreach(&input.command);
    let overreach_why = overreach.as_ref().map(|(path, reason)| reason.message(path));
    match format.gated_policy() {
        safe_chains::targets::GatedPolicy::Deny => {
            let reason = match &overreach_why {
                Some(why) => format!(
                    "safe-chains blocked this: {why}. This harness has no interactive approval. {DOCS_URL}"
                ),
                None => format!(
                    "safe-chains blocked this: it is not on the allowlist and this harness has no \
                     interactive approval. To allow it, add a custom command or a grant to \
                     ~/.config/safe-chains.toml. {DOCS_URL}"
                ),
            };
            let response = format.render_deny(&reason);
            let _ = io::stdout().write_all(response.stdout.as_bytes());
            process::exit(response.exit_code);
        }
        safe_chains::targets::GatedPolicy::Ask => {
            let reason = match &overreach_why {
                Some(why) => format!(
                    "safe-chains did not auto-approve this, so please confirm: {why}. {DOCS_URL}"
                ),
                None => "safe-chains did not auto-approve this command, so please confirm. Add it \
                     to ~/.config/safe-chains.toml so safe-chains stops flagging it."
                    .to_string(),
            };
            let response = format.render_ask(&reason);
            let _ = io::stdout().write_all(response.stdout.as_bytes());
            process::exit(response.exit_code);
        }
        safe_chains::targets::GatedPolicy::Defer => {}
    }

    if explanation.should_surface() {
        let response = format.render_context(&explanation.render());
        let _ = io::stdout().write_all(response.stdout.as_bytes());
        process::exit(response.exit_code);
    }

    // The retreat's nudge: if the command wasn't auto-approved because it reaches outside the
    // workspace, say so (and how to allow it) instead of a silent prompt. Degrades to a plain
    // prompt on harnesses without additionalContext.
    if let Some((path, reason)) = overreach {
        let nudge = format!(
            "safe-chains did not auto-approve this: {}. {DOCS_URL}",
            reason.message(&path)
        );
        let response = format.render_context(&nudge);
        let _ = io::stdout().write_all(response.stdout.as_bytes());
        process::exit(response.exit_code);
    }

    process::exit(0);
}

fn main() {
    let cli = Cli::try_parse();

    match cli {
        Ok(cli) => {
            if let Some(Subcommand::Hook { tool }) = cli.subcommand {
                run_hook_for(&tool);
            }
            if cli.list_tools {
                run_list_tools();
            }
            if cli.setup {
                run_setup(cli.tool, cli.auto_detect);
            }
            if cli.list_commands {
                print_docs();
            } else if cli.generate_book {
                let docs = safe_chains::docs::all_command_docs();
                safe_chains::docs::render_book(&docs, std::path::Path::new("docs"));
            } else if let Some(command) = cli.command {
                let _ctx = safe_chains::pathctx::enter(safe_chains::pathctx::PathCtx {
                    // Default root to cwd, exactly as the hook arm above does. `pathctx::resolve`
                    // joins a relative path only when BOTH are known, so `--cwd X` on its own was
                    // silently inert: `cd /etc && cat master.passwd` came back approved from the
                    // CLI and denied through the hook, for the same command. A debugging tool that
                    // disagrees with the thing it debugs is worse than no tool.
                    cwd: cli.cwd.clone(),
                    root: cli.root.or_else(|| cli.cwd.clone()),
                    session_id: cli.session_id,
                });
                if cli.explain {
                    run_explain(&command);
                }
                if cli.suggest {
                    run_suggest(&command);
                }
                let (threshold, engine_level) = match cli.level.as_deref() {
                    None => (SafetyLevel::SafeWrite, None), // default: developer
                    Some(name) => {
                        if let Some((_, Some(current))) = SafetyLevel::resolve_threshold(name) {
                            eprintln!(
                                "note: '--level {name}' is a legacy level name, mapping to \
                                 '{current}'. Current levels: paranoid, reader, editor, \
                                 developer, local-admin, network-admin, yolo."
                            );
                        }
                        match safe_chains::level_ceiling(name) {
                            Some(pair) => pair,
                            None => {
                                eprintln!(
                                    "Error: unknown --level '{name}'. Levels: paranoid, reader, editor, \
                                     developer, local-admin, network-admin, yolo (legacy: inert, \
                                     safe-read, safe-write)."
                                );
                                process::exit(2);
                            }
                        }
                    }
                };
                run_cli(&command, threshold, engine_level);
            } else if io::stdin().is_terminal() {
                Cli::command().print_help().ok();
                println!();
                process::exit(2);
            } else {
                let claude = targets::find("claude").expect("claude target registered");
                let format = claude
                    .hook_format()
                    .expect("claude target has a hook format");
                // The no-argument stdin path IS the Claude hook (see the CLI docs), so it keeps
                // Claude's permission files as a trust source.
                run_hook_format(format, "claude");
            }
        }
        // A malformed CLI invocation — an unknown/typo'd flag (`--levle`), a bad value — must FAIL
        // CLOSED. clap prints the error and exits 2 (help/version exit 0). It must NEVER fall
        // through to hook mode: in CLI-gate mode there is no stdin JSON, so the hook would read
        // empty input and exit 0 = "allowed" — a security FAIL-OPEN (`safe-chains "rm -rf /"
        // --levle inert` would exit 0). Every legit hook invocation — `safe-chains` bare, or
        // `safe-chains hook <target>` — PARSES cleanly (the `Ok` arm above), so it never reaches here.
        Err(e) => e.exit(),
    }
}

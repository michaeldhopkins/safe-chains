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

fn print_opencode_config() {
    let patterns = safe_chains::all_opencode_patterns();
    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    print!(
        "{}",
        safe_chains::targets::opencode::render_opencode_json_in(&cwd, &patterns)
    );
}

fn run_cli(
    command: &str,
    threshold: SafetyLevel,
    upper_level: Option<&'static safe_chains::engine::level::Level>,
) {
    // Upper-band levels (local-admin/network-admin/yolo) have no 3-value ceiling, so they classify
    // per-level via `admits`; the lower band keeps the existing projection. Both funnel into the
    // same `<= threshold` gate (upper levels share the `SafeWrite` ceiling).
    let verdict = match upper_level {
        Some(level) => safe_chains::command_verdict_at_level(command, level),
        None => safe_chains::command_verdict(command),
    };
    let ok = match verdict {
        Verdict::Allowed(level) => level <= threshold,
        Verdict::Denied => false,
    };
    process::exit(i32::from(!ok));
}

fn run_explain(command: &str) -> ! {
    let explanation = safe_chains::cst::explain(command);
    print!("{}", explanation.render());
    process::exit(i32::from(!explanation.is_allowed()));
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
    run_hook_format(format);
}

/// The "outside the working directory" clause, NAMING the cwd when the harness reported one — so a
/// directory MISMATCH (the agent was launched from the wrong repo, a common and easy-to-forget
/// mistake) is visible in the message. Without naming it, the user can't tell "I meant to be
/// elsewhere" from "this command genuinely overreaches".
fn outside_workspace_clause() -> String {
    match safe_chains::pathctx::cwd() {
        Some(cwd) => format!("outside the working directory `{cwd}`"),
        None => "outside the working directory".to_string(),
    }
}

fn run_hook_format(format: &dyn HookFormat) -> ! {
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
    });
    let verdict = safe_chains::command_verdict(&input.command);
    if verdict.is_allowed() {
        let response = format.render_response(verdict);
        let _ = io::stdout().write_all(response.stdout.as_bytes());
        process::exit(response.exit_code);
    }

    let patterns = safe_chains::allowlist::Matcher::load();
    let explanation = safe_chains::cst::explain_with_coverage(&input.command, &patterns);

    if explanation.is_allowed() {
        let response = format.render_response(Verdict::Allowed(SafetyLevel::Inert));
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
    let overreach_why = overreach.as_ref().map(|(path, is_credential)| {
        if *is_credential {
            format!("it reaches `{path}`, a credential store the agent should almost certainly not touch")
        } else {
            format!("it reaches `{path}`, {}", outside_workspace_clause())
        }
    });
    match format.gated_policy() {
        safe_chains::targets::GatedPolicy::Deny => {
            let reason = match &overreach_why {
                Some(why) => format!(
                    "safe-chains blocked this: {why}. This harness has no interactive approval; to \
                     allow it, add a custom command or a grant to ~/.config/safe-chains.toml. {DOCS_URL}"
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
                    "safe-chains did not auto-approve this — please confirm: {why}. {DOCS_URL}"
                ),
                None => "safe-chains did not auto-approve this command — please confirm. (Add it to \
                     ~/.config/safe-chains.toml so safe-chains stops flagging it.)"
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
    if let Some((path, is_credential)) = overreach {
        let nudge = if is_credential {
            format!(
                "safe-chains did not auto-approve this: it reaches `{path}`, a credential store \
                 the agent should almost certainly not touch. If this was not intended, stop it. \
                 {DOCS_URL}"
            )
        } else {
            format!(
                "safe-chains did not auto-approve this: it reaches `{path}`, {}. If the agent is \
                 running from the wrong directory — an easy thing to forget — relaunch it where you \
                 meant to be; to allow it from here, grant that path in ~/.config/safe-chains.toml — \
                 otherwise reconsider whether the agent should reach there. {DOCS_URL}",
                outside_workspace_clause(),
            )
        };
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
            } else if cli.opencode_config {
                print_opencode_config();
            } else if let Some(command) = cli.command {
                let _ctx = safe_chains::pathctx::enter(safe_chains::pathctx::PathCtx {
                    cwd: cli.cwd,
                    root: cli.root,
                });
                if cli.explain {
                    run_explain(&command);
                }
                let (threshold, upper_level) = match cli.level.as_deref() {
                    None => (SafetyLevel::SafeWrite, None), // default: developer
                    Some(name) => match SafetyLevel::resolve_threshold(name) {
                        Some((ceiling, legacy_of)) => {
                            if let Some(current) = legacy_of {
                                eprintln!(
                                    "note: '--level {name}' is a legacy level name — mapping to \
                                     '{current}'. Current levels: paranoid, reader, editor, \
                                     developer, local-admin, network-admin, yolo."
                                );
                            }
                            // A legacy alias maps to its current name before the upper-band lookup.
                            let canonical = legacy_of.unwrap_or(name);
                            (ceiling, safe_chains::upper_level_by_name(canonical))
                        }
                        None => {
                            eprintln!(
                                "Error: unknown --level '{name}'. Levels: paranoid, reader, editor, \
                                 developer, local-admin, network-admin, yolo (legacy: inert, \
                                 safe-read, safe-write)."
                            );
                            process::exit(2);
                        }
                    },
                };
                run_cli(&command, threshold, upper_level);
            } else if io::stdin().is_terminal() {
                Cli::command().print_help().ok();
                println!();
                process::exit(2);
            } else {
                let claude = targets::find("claude").expect("claude target registered");
                let format = claude
                    .hook_format()
                    .expect("claude target has a hook format");
                run_hook_format(format);
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

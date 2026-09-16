use std::io::{self, IsTerminal};
use std::process;

use clap::{CommandFactory, Parser};

use safe_chains::cli::{Cli, Subcommand};
use safe_chains::targets;
use safe_chains::verdict::{SafetyLevel, Verdict};

mod hook_cli;
mod suggest_cli;

/// The page that documents the path model and, with it, the `[[grant]]` that widens it. Every
/// output whose remedy is "grant that path" links HERE — the hook nudge, `--explain` and
/// `--suggest` — because sending a reader to a page that does not mention grants is how they
/// conclude the feature does not exist.
const HOW_IT_WORKS_URL: &str = "https://www.michaeldhopkins.com/docs/safe-chains/how-it-works.html";

fn print_docs() {
    let docs = safe_chains::docs::all_command_docs();
    print!("{}", safe_chains::docs::render_markdown(&docs));
}

/// The CLI's verdict, computed exactly as the hook computes its own.
///
/// The hook decides in two steps: the ceilinged verdict, and if that denies, a coverage fallback
/// that also honors the user's own `permissions.allow` patterns — then gates `overall <= threshold`.
/// This collapses the two into one call, which is equivalent rather than merely similar:
/// `effective_verdict` returns the base verdict untouched whenever the base is allowed, so coverage
/// only ever widens, and with nothing covered `overall` IS the plain verdict.
///
/// It used to run `command_verdict_ceilinged` alone and so never saw the user's patterns, which
/// meant `safe-chains "<cmd>"` reported denied for a command the hook approved. Same footgun as
/// `--cwd` without `--root`: this is the tool people run to ask what the hook decided.
///
/// A covered segment classifies `SafeWrite` — the top of the auto-approve band, and the DEFAULT
/// threshold — so a `permissions.allow` rule is honoured exactly as the hook honours it, while a
/// stricter `--level` still clamps it: `paranoid` and `reader` refuse a covered command rather than
/// being overridden by it. It used to classify `Inert`, the bottom of the ordering, which cleared
/// every threshold and let a `Bash(rm:*)` rule out-rank `--level paranoid`.
fn run_cli(
    command: &str,
    threshold: SafetyLevel,
    engine_level: Option<&'static safe_chains::engine::level::Level>,
    log_mode: safe_chains::decisionlog::Mode,
    cwd: Option<&str>,
    root: Option<&str>,
) {
    let explanation = safe_chains::explain_with_coverage_at_level(command, engine_level);
    let allowed = matches!(explanation.overall, Verdict::Allowed(level) if level <= threshold);
    // The CLI logs too. `--log` used to be accepted here and silently do nothing, which is the
    // worst behaviour available for a flag: `safe-chains --log "cmd"` reported the right verdict and
    // wrote no entry, so the first thing anyone would try to convince themselves logging worked
    // quietly proved the opposite. `harness: "cli"` distinguishes these from real hook traffic.
    let level_name = engine_level.map_or_else(
        || safe_chains::engine::bridge::default_band_top_name().to_string(),
        |l| l.name.clone(),
    );
    let outcome = if allowed {
        safe_chains::decisionlog::Outcome::Allowed
    } else if explanation.parsed {
        safe_chains::decisionlog::Outcome::Denied
    } else {
        safe_chains::decisionlog::Outcome::Unparseable
    };
    safe_chains::decisionlog::record(
        log_mode,
        outcome,
        &safe_chains::decisionlog::Context {
            command,
            cwd,
            root,
            session_id: None,
            harness: "cli",
            level: &level_name,
        },
        Some(&explanation),
    );
    process::exit(i32::from(!allowed));
}

fn run_explain(
    command: &str,
    engine_level: Option<&'static safe_chains::engine::level::Level>,
) -> ! {
    // Coverage here too, and for the same reason: the hook renders THIS explanation into the
    // model's context, so an `--explain` that omitted the user's own patterns showed a `✗` beside a
    // segment the agent had just watched be approved.
    let explanation = safe_chains::explain_with_coverage_at_level(command, engine_level);
    print!("{}", explanation.render());
    // The facet breakdown is CLI-only. `render()` also feeds the hook's injected context, where an
    // agent mid-chain needs the verdict and nothing else; a 27-axis dump there would be noise it
    // cannot act on. Someone who typed `--explain` is asking why, so they get why.
    print!("{}", safe_chains::facet_breakdown(command));
    // …and the reach clause, which is the rest of "why" for anything refused over a PATH.
    //
    // The hook has always emitted this; `--explain` did not, so the two disagreed about the reason
    // as well as (until now) the verdict. What `--explain` said instead was actively wrong: the
    // header reads "safe-chains approves commands it has researched and has no opinion about the
    // rest", which is a claim about an UNKNOWN command, and the advice under it is about splitting
    // chains. For `unzip -l ~/Library/…/x.zip` the command is researched, the opinion is specific,
    // and neither line points at the path — so a reader doing exactly what --help tells them to do
    // ("--explain … names the facet that refused it") concluded safe-chains had no path model and
    // no way to widen it, while the hook was naming both the path and the remedy the whole time.
    //
    // The facet breakdown covers the engine's own commands; this covers the legacy surface, where
    // the breakdown is empty and there is otherwise nothing to read.
    if !explanation.is_allowed()
        && let Some((path, reason)) = safe_chains::workspace_overreach(command)
    {
        // "Why:" rather than the hook's "safe-chains did not auto-approve this:" lead-in, which the
        // header three lines up has already said. `message` opens mid-sentence ("it reaches …"), so
        // it needs something in front of it or it reads as an orphan.
        println!("\nWhy: {}. {HOW_IT_WORKS_URL}", reason.message(&path));
    }
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
fn main() {
    let cli = Cli::try_parse();

    match cli {
        Ok(cli) => {
            let log_mode =
                safe_chains::decisionlog::Mode::from_flags(cli.log, cli.log_everything);
            if let Some(Subcommand::Hook { tool }) = cli.subcommand {
                hook_cli::run_hook_for(&tool, log_mode);
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
                // Default the cwd to the directory the CLI was RUN from, then the root to that cwd
                // exactly as the hook arm above does. Both defaults exist for one reason: a
                // debugging tool that disagrees with the thing it debugs is worse than no tool.
                //
                // The root default alone was not enough. `pathctx::resolve` joins a relative path
                // only when BOTH are known, so with no `--cwd` there was no workspace boundary at
                // all — a `cd` out of the project was invisible and every relative path behind it
                // classified worktree-local. `safe-chains 'cd ~/Library/… && unzip -l x.zip'`
                // answered ALLOW while the hook, which always receives a cwd, abstained on the same
                // command in the same directory. Nobody passes `--cwd` by hand; the `--help`
                // examples don't either, so the lenient path was the one everyone measured with.
                //
                // `current_dir()` and NOT `$PWD`, deliberately. `$PWD` is what a shell would use and
                // is the only thing that preserves a symlinked spelling, but it is an ordinary
                // environment variable — an agent that can set it could name any directory as the
                // workspace. `current_dir()` asks the kernel. The cost is that a cwd reached
                // through a symlink resolves to its physical path here while a harness may report
                // the logical one; both are then classified consistently, just not identically.
                //
                // A process cwd is always available in practice; if it somehow isn't, fall back to
                // the old boundary-less behaviour rather than inventing a root.
                let effective_cwd = cli.cwd.clone().or_else(|| {
                    std::env::current_dir().ok().map(|p| p.to_string_lossy().into_owned())
                });
                // Captured before the PathCtx consumes them, so a logged entry records the same
                // directory context the classification ran under — the EFFECTIVE cwd, not the flag,
                // or the log would disagree with the verdict beside it.
                let (log_cwd, log_root) =
                    (effective_cwd.clone(), cli.root.clone().or_else(|| effective_cwd.clone()));
                let _ctx = safe_chains::pathctx::enter(safe_chains::pathctx::PathCtx {
                    cwd: effective_cwd.clone(),
                    root: cli.root.or(effective_cwd),
                    session_id: cli.session_id,
                });
                // Same reason as the root default above: this is the tool people run to ask why the
                // HOOK decided something, so it has to decide it the same way. Scoping Claude's
                // permission files to the Claude target left the CLI ignoring them, which meant
                // `safe-chains "curl … | sh"` said denied while the Claude hook said allow, for the
                // same command and the same config. The bare stdin mode IS the Claude hook (see the
                // CLI docs), so Claude is this binary's default persona; `hook <tool>` is how you
                // ask about a different harness.
                safe_chains::trust_claude_config();
                // The level is resolved BEFORE `--explain`, which used to run first and so
                // explained at the default band whatever `--level` said. The hook explains under
                // its configured level; asking why something was refused under `reader` should
                // answer about `reader`.
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
                // `--explain` still wins over `--suggest` when both are passed, as it did before the
                // level resolution moved above them. Swapping that precedence would have been a
                // silent side effect of an unrelated change.
                if cli.explain {
                    run_explain(&command, engine_level);
                }
                if cli.suggest {
                    suggest_cli::run_suggest(&command);
                }
                run_cli(
                    &command,
                    threshold,
                    engine_level,
                    log_mode,
                    log_cwd.as_deref(),
                    log_root.as_deref(),
                );
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
                hook_cli::run_hook_format(format, "claude", log_mode);
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

use std::io::{self, IsTerminal, Read, Write};
use std::path::Path;
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

fn run_cli(command: &str, threshold: SafetyLevel) {
    let verdict = safe_chains::command_verdict(command);
    let ok = match verdict {
        Verdict::Allowed(level) => level <= threshold,
        Verdict::Denied => false,
    };
    process::exit(i32::from(!ok));
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

fn run_hook_format(format: &dyn HookFormat) -> ! {
    let mut buf = String::new();
    if io::stdin().read_to_string(&mut buf).is_err() {
        process::exit(0);
    }

    let Ok(input) = format.parse_input(&buf) else {
        process::exit(0);
    };

    let verdict = safe_chains::command_verdict(&input.command);
    if verdict.is_allowed() {
        let response = format.render_response(verdict);
        let _ = io::stdout().write_all(response.stdout.as_bytes());
        process::exit(response.exit_code);
    }

    let project_dir = input.cwd.as_deref().map(Path::new);
    let patterns = safe_chains::allowlist::Matcher::load_with_project_dir(project_dir);
    if patterns.is_empty() {
        process::exit(0);
    }

    let Some(script) = safe_chains::cst::parse(&input.command) else {
        process::exit(0);
    };

    let all_covered = script.0.iter().all(|stmt| {
        safe_chains::cst::is_safe_pipeline(&stmt.pipeline)
            || stmt
                .pipeline
                .commands
                .iter()
                .all(|cmd| safe_chains::allowlist::is_cmd_covered(cmd, &patterns))
    });

    if all_covered {
        let response = format.render_response(Verdict::Allowed(SafetyLevel::Inert));
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
                let threshold = cli.level.unwrap_or(SafetyLevel::SafeWrite);
                run_cli(&command, threshold);
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
        Err(e)
            if e.kind() == clap::error::ErrorKind::DisplayHelp
                || e.kind() == clap::error::ErrorKind::DisplayVersion =>
        {
            print!("{e}");
        }
        Err(_) => {
            let claude = targets::find("claude").expect("claude target registered");
            let format = claude
                .hook_format()
                .expect("claude target has a hook format");
            run_hook_format(format);
        }
    }
}

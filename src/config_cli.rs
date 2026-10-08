//! `safe-chains config …`: the binary's half. The library decides and edits; this prints and exits.

use std::process;

use safe_chains::cli::{ConfigAction, RuleKind};
use safe_chains::userconfig::{self, Access};

pub fn run_config(action: ConfigAction) -> ! {
    let result = match action {
        ConfigAction::List => Ok(userconfig::list()),
        ConfigAction::Add { command, level } => {
            let command = match command.as_slice() {
                [one] => one.clone(),
                words if words.iter().any(|w| w == "--level" || w.starts_with("--level=")) => {
                    eprintln!("{}", userconfig::EditError::LevelInCommand);
                    process::exit(1);
                }
                words => shell_words::join(words),
            };
            userconfig::add_entry(&command, &level)
        }
        ConfigAction::Remove { kind: RuleKind::Command, key } => userconfig::remove_command(&key),
        ConfigAction::Remove { kind: RuleKind::Grant, key } => userconfig::remove_grant(&key),
        ConfigAction::Grant { path, read, write } => Access::from_flags(read, write).and_then(|access| userconfig::grant(&path, access)),
    };
    match result {
        Ok(message) => {
            println!("{}", message.trim_end());
            process::exit(0);
        }
        Err(error) => {
            eprintln!("{error}");
            process::exit(1);
        }
    }
}

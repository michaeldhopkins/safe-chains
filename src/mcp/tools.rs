//! The tools: what a client lists, and how a call reaches the CLI.

use std::path::{Path, PathBuf};

use serde_json::{Map, Value, json};

use super::args::{self, NOTHING_DONE};
use super::client_approval;

/// The longest result a tool returns. Larger results are cut, with a line saying how much was left.
const MAX_RESULT: usize = 8_000;

/// How a tool reaches the `safe-chains` binary. Tests and the fuzz target substitute a stand-in.
pub trait Runner {
    /// Run `safe-chains <args>` in `cwd`, returning its exit code (`None` when it did not exit
    /// normally) and what it printed.
    fn run(&self, args: &[String], cwd: &Path) -> (Option<i32>, String);
    fn home(&self) -> PathBuf;
    fn cwd(&self) -> PathBuf;
}

/// The real runner: this executable, with `SAFE_CHAINS_SURFACE=mcp` so its messages name tools.
pub struct Spawn;

impl Runner for Spawn {
    fn run(&self, args: &[String], cwd: &Path) -> (Option<i32>, String) {
        let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("safe-chains"));
        match std::process::Command::new(exe)
            .args(args)
            .current_dir(cwd)
            .env("SAFE_CHAINS_SURFACE", "mcp")
            .output()
        {
            Ok(out) => {
                let mut text = String::from_utf8_lossy(&out.stdout).into_owned();
                text.push_str(&String::from_utf8_lossy(&out.stderr));
                (out.status.code(), text.trim_end().to_string())
            }
            Err(e) => (None, format!("safe-chains could not start itself to answer: {e}. {NOTHING_DONE}")),
        }
    }

    fn home(&self) -> PathBuf {
        std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default()
    }

    fn cwd(&self) -> PathBuf {
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"))
    }
}

struct Tool {
    name: &'static str,
    description: &'static str,
    schema: Value,
    edits: bool,
}

fn object(properties: Value, required: &[&str]) -> Value {
    let mut schema = json!({ "type": "object", "properties": properties, "additionalProperties": false });
    if !required.is_empty() {
        schema["required"] = json!(required);
    }
    schema
}

fn registry() -> Vec<Tool> {
    vec![
        Tool {
            name: "check_command",
            description: "Check whether safe-chains would let a shell command run without asking a person, and if not, \
                          what would change that. Start here. Pass the command exactly as it would run, and cwd, the \
                          directory it would run in (default: the server's own). The answer ends with a Fix line: add an \
                          entry (add_command_entry), grant a path (grant_path), or nothing, because a built-in rule \
                          refused it. Changes nothing. Example: {\"command\": \"mytool sync --dry-run\", \"cwd\": \"~/work/app\"}",
            schema: object(
                json!({
                    "command": { "type": "string", "description": "The shell command, exactly as it would run." },
                    "cwd": { "type": "string", "description": "The directory it would run in. Default: the server's own." }
                }),
                &["command"],
            ),
            edits: false,
        },
        Tool {
            name: "list_rules",
            description: "List every rule safe-chains reads from a file, with the file it came from. That covers entries and grants \
                          in ~/.config/safe-chains.toml, a project's .safe-chains.toml, and rules borrowed from Claude \
                          Code's settings. Use it to find a rule's name or path. It changes nothing: to take a rule out, call remove_rule.",
            schema: object(json!({}), &[]),
            edits: false,
        },
        Tool {
            name: "add_command_entry",
            description: "Add an entry so safe-chains approves a command it has no entry for. The entry goes in \
                          ~/.config/safe-chains.toml and covers the command used the way it is in command: the same \
                          subcommand and flags, and up to as many other arguments. Use it only when the user asked for \
                          this, after check_command says an entry is the fix. It changes what runs without asking, so \
                          the client must ask the user first. It refuses \
                          a built-in command and a command that runs an AI agent. level is inert, safe-read or \
                          safe-write (default safe-write).",
            schema: object(
                json!({
                    "command": { "type": "string", "description": "The command as it should be approved, e.g. \"mytool sync --dry-run\"." },
                    "level": { "type": "string", "enum": ["inert", "safe-read", "safe-write"], "default": "safe-write",
                               "description": "inert: reads and changes nothing. safe-read: reads files in the project. safe-write: also writes them." }
                }),
                &["command"],
            ),
            edits: true,
        },
        Tool {
            name: "grant_path",
            description: "Let commands read or write a folder outside the places safe-chains approves by default. It \
                          adds a grant to ~/.config/safe-chains.toml. Use it only when the user asked for this, after \
                          check_command says a grant is the fix. access is read, write or read_write. It refuses \
                          credential stores, the settings safe-chains and coding agents read, shell startup files, \
                          folders on PATH, system login files, devices, / and the home folder itself. It changes what \
                          runs without asking, so the client must ask the user first. Example: {\"path\": \"~/notes\", \
                          \"access\": \"read\"}",
            schema: object(
                json!({
                    "path": { "type": "string", "description": "The folder or file: a full path, or one starting with ~/." },
                    "access": { "type": "string", "enum": ["read", "write", "read_write"], "description": "What commands may do there." }
                }),
                &["path", "access"],
            ),
            edits: true,
        },
        Tool {
            name: "remove_rule",
            description: "Remove a command entry or a grant from ~/.config/safe-chains.toml. kind is command (name is the \
                          command's name) or grant (name is the grant's path). list_rules shows both. Removing can also \
                          widen what runs, when the entry removed was refusing a command, so the client must ask the user \
                          first.",
            schema: object(
                json!({
                    "kind": { "type": "string", "enum": ["command", "grant"], "description": "command or grant." },
                    "name": { "type": "string", "description": "The command's name, or the grant's path." }
                }),
                &["kind", "name"],
            ),
            edits: true,
        },
    ]
}

/// The `tools/list` result, in a fixed order.
pub fn listing() -> Vec<Value> {
    registry()
        .into_iter()
        .map(|t| {
            json!({
                "name": t.name,
                "description": t.description,
                "inputSchema": t.schema,
                "annotations": {
                    "readOnlyHint": !t.edits,
                    "destructiveHint": t.edits,
                    "idempotentHint": !t.edits,
                    "openWorldHint": false,
                },
            })
        })
        .collect()
}

/// The `tools/call` result. A failure is a result with `isError`, never a protocol error.
pub fn call(runner: &dyn Runner, params: &Value) -> Value {
    let name = params.get("name").and_then(Value::as_str).unwrap_or_default();
    let tools = registry();
    let outcome = match tools.iter().find(|t| t.name == name) {
        Some(tool) => {
            args::check(tool.name, &tool.schema, params.get("arguments").unwrap_or(&Value::Null)).and_then(|args| run(runner, tool, &args))
        }
        None => {
            let names: Vec<&str> = tools.iter().map(|t| t.name).collect();
            Err(format!("There is no tool {name:?}. The tools are: {}. {NOTHING_DONE}", names.join(", ")))
        }
    };
    let (text, is_error) = match outcome {
        Ok(text) => (text, false),
        Err(text) => (text, true),
    };
    json!({ "content": [{ "type": "text", "text": capped(&text) }], "isError": is_error })
}

fn text<'a>(args: &'a Map<String, Value>, key: &str) -> &'a str {
    args.get(key).and_then(Value::as_str).unwrap_or_default()
}

fn run(runner: &dyn Runner, tool: &Tool, args: &Map<String, Value>) -> Result<String, String> {
    let cwd = runner.cwd();
    if tool.edits
        && let Some(why) = client_approval::auto_approval(&runner.home(), &cwd)
    {
        return Err(format!(
            "safe-chains will not make this change from here: this client runs {} without asking anyone, because {why}. \
             An agent could then approve its own commands. Remove that setting so the client asks first, or ask the \
             user to run `safe-chains config` themselves. {NOTHING_DONE}",
            tool.name
        ));
    }
    let mut note = String::new();
    let owned = |v: &[&str]| v.iter().map(|s| (*s).to_string()).collect::<Vec<String>>();
    let (argv, dir) = match tool.name {
        "check_command" => {
            // A directory that does not exist is checked from the server's own instead, with a note:
            // a small model guessed `/Users/<name>/work/app` for `~/work/app`, was refused, retried
            // the same call and gave up (blind round 2).
            let dir = match args.get("cwd").and_then(Value::as_str).map(|d| expand_home(d, &runner.home())) {
                Some(dir) if dir.is_dir() => dir,
                Some(missing) => {
                    note = format!(
                        "(cwd {} does not exist, so this was checked from {} instead. That matters only for relative \
                         paths in the command.)\n",
                        missing.display(),
                        cwd.display()
                    );
                    cwd
                }
                None => cwd,
            };
            let shown = dir.to_string_lossy().into_owned();
            (owned(&["--explain", "--cwd", &shown, "--", text(args, "command")]), dir)
        }
        "list_rules" => (owned(&["config", "list"]), cwd),
        "add_command_entry" => {
            let level = args.get("level").and_then(Value::as_str).unwrap_or("safe-write");
            (owned(&["config", "add", "--level", level, "--", text(args, "command")]), cwd)
        }
        "grant_path" => {
            let flags: &[&str] = match text(args, "access") {
                "read" => &["--read"],
                "write" => &["--write"],
                _ => &["--read", "--write"],
            };
            let mut argv = owned(&["config", "grant"]);
            argv.extend(owned(flags));
            argv.extend(owned(&["--", text(args, "path")]));
            (argv, cwd)
        }
        _ => (owned(&["config", "remove", text(args, "kind"), "--", text(args, "name")]), cwd),
    };
    let (code, out) = runner.run(&argv, &dir);
    // check_command answers with exit 1 for a refusal, which is an answer. Anything else that is not
    // 0 (a crash, a signal) is a failure, and is reported as one rather than as a verdict.
    let answered = code == Some(0) || tool.name == "check_command" && code == Some(1);
    if answered {
        Ok(format!("{note}{out}"))
    } else {
        Err(if out.is_empty() { format!("safe-chains stopped without an answer. {NOTHING_DONE}") } else { out })
    }
}

fn expand_home(dir: &str, home: &Path) -> PathBuf {
    match dir.strip_prefix("~") {
        Some(rest) if rest.is_empty() || rest.starts_with('/') => home.join(rest.trim_start_matches('/')),
        _ => PathBuf::from(dir),
    }
}

fn capped(text: &str) -> String {
    if text.len() <= MAX_RESULT {
        return text.to_string();
    }
    let mut cut = MAX_RESULT;
    while !text.is_char_boundary(cut) {
        cut -= 1;
    }
    let kept = &text[..cut];
    let kept = &kept[..kept.rfind('\n').unwrap_or(kept.len())];
    let more = text[kept.len()..].lines().filter(|l| !l.trim().is_empty()).count();
    format!("{kept}\n… {more} more lines not shown.")
}

/// `tests/mcp_tools.toml`: each tool and what it can do, for review and the allowlist check.
pub fn manifest() -> String {
    let mut out = String::from(
        "# Every MCP tool safe-chains lists, with what it can do. Written by the test\n\
         # `the_tool_manifest_is_what_the_server_lists` (SAFE_CHAINS_WRITE_MCP_MANIFEST=1 rewrites it).\n",
    );
    for t in registry() {
        out.push_str(&format!(
            "\n[{}]\nread_only = {}\ndestructive = {}\nspends = false\nnetwork = false\nsends = false\n",
            t.name, !t.edits, t.edits
        ));
    }
    out
}

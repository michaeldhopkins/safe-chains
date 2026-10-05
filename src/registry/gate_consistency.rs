//! Consistency between a command's declared `level` and the path gate applied to it.
//!
//! Its own file rather than another block in `tests.rs`, which the file-length gate pins: new code
//! goes in a new module. That is also the better place for it — this reads two SEPARATE sources of
//! truth about one command (`commands/**/*.toml` and `pathgates.toml`) and checks they agree, which
//! is not what the rest of `tests.rs` does.
use crate::pathgate::{PositionalWriteGate, positional_write_gate};
use crate::registry::types::{TomlFile, TomlLevel};

/// The path gate and the command's own `level` must not contradict each other.
///
/// `level = "Inert"` is a researched claim that the command observes and changes nothing real.
/// Gating its POSITIONALS as writes is the opposite claim — that an operand is a thing it
/// destroys — and the two cannot both be true. When they disagree the gate silently wins,
/// because it is strictly the tighter of the two, so the contradiction shows up as an
/// unexplainable false deny rather than as an error.
///
/// `unzip` was the case that proved it. Its TOML is `level = "Inert"` with
/// `require_any = ["-c", "-l", "-p", "-t", "-Z"]`, so the only invocations safe-chains can ever
/// approve are the read-only modes — listing, testing, streaming a member to stdout — and `-d`,
/// which chooses an extraction directory, is not in its flag set at all. There is no approvable
/// `unzip` that writes. It sat in the flat `write` list all the same, so `unzip -l` on a file
/// outside the workspace was gated by WRITE locus and refused, while `cat`, `less`, `od` and
/// `tar -tf` read the very same file, and `zipinfo -1` — which is literally what `unzip -Z`
/// runs — was approved on it too. Two spellings of one operation, opposite answers.
///
/// Scoped to `Inert` deliberately. A `SafeRead` command with a blanket write gate is usually a
/// tool whose DEFAULT mode writes (`yapf`, `xz`, the autofix formatters): the gate is a
/// conservative stand-in until its write-flag surface is enumerated, which is per-command
/// research rather than a contradiction in what we already know. `Inert` admits no such
/// reading — it says there is no writing mode to enumerate.
#[test]
fn no_inert_command_is_gated_as_a_writer() {
    fn toml_files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                toml_files(&path, out);
            } else if path.extension().is_some_and(|e| e == "toml") {
                out.push(path);
            }
        }
    }

    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("commands");
    let mut files = Vec::new();
    toml_files(&root, &mut files);

    let (mut inert, mut blanket, mut unpromoted) = (0usize, Vec::new(), Vec::new());
    for file in &files {
        let src = std::fs::read_to_string(file).unwrap();
        let parsed: TomlFile = toml::from_str(&src).unwrap_or_else(|e| panic!("{}: {e}", file.display()));
        for cmd in &parsed.command {
            if !matches!(cmd.level, Some(TomlLevel::Inert)) {
                continue;
            }
            // Aliases too: the gate is keyed on the token as WRITTEN, so `cat` and `gcat` are
            // two spellings of one command and a contradiction can be declared against either.
            for name in std::iter::once(&cmd.name).chain(cmd.aliases.iter()) {
                inert += 1;
                let where_ = format!("  {name} (in {})", file.file_name().unwrap().to_string_lossy());
                match positional_write_gate(name) {
                    PositionalWriteGate::Unconditional => blanket.push(where_),
                    // A conditional gate is only half a statement. The other half is the LEVEL
                    // rising in the same mode, which `write_flags` is what says so. Without it
                    // the gate would catch the path while the level still called the
                    // invocation Inert, so `--level paranoid` would approve a mutation.
                    PositionalWriteGate::Conditional if cmd.write_flags.is_empty() => {
                        unpromoted.push(where_);
                    }
                    _ => {}
                }
            }
        }
    }

    assert!(
        blanket.is_empty(),
        "declared `level = \"Inert\"` but gated as writing their positionals in EVERY \
         invocation:\n{}\nEither the level is wrong (there is a writing mode, so it is not \
         Inert) or the gate is (its operands are reads — give it \
         `[roles.X] positional = \"read\"`).",
        blanket.join("\n"),
    );
    assert!(
        unpromoted.is_empty(),
        "declared `level = \"Inert\"` with a gate that promotes their positionals to writes \
         under a flag, but no `write_flags` to raise the LEVEL in that same mode:\n{}\nThe \
         gate would refuse the path while the level still called the invocation Inert.",
        unpromoted.join("\n"),
    );

    // Non-vacuity: the sweep must have found commands, and the predicate must see real gates
    // of both shapes — `shred` writes every operand, `afhash` only under `-w`.
    assert!(inert > 100, "the Inert roster collapsed — this guard is sweeping nothing");
    assert_eq!(positional_write_gate("shred"), PositionalWriteGate::Unconditional);
    assert_eq!(positional_write_gate("afhash"), PositionalWriteGate::Conditional);
    assert_eq!(positional_write_gate("cat"), PositionalWriteGate::None);
    // …and a SUB-scoped gate is seen, or commands whose only write gate hangs off a subcommand
    // would be swept without ever being looked at.
    assert_eq!(positional_write_gate("dart"), PositionalWriteGate::Unconditional);
}

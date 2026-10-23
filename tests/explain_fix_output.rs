//! What `--explain` prints after the segments it refused: one fix per refused segment, headed by
//! the segment only when there is more than one.
use std::process::{Command, Stdio};

fn explain(args: &[&str], command: &str) -> String {
    let home = tempfile::tempdir().expect("tempdir");
    let out = Command::new(env!("CARGO_BIN_EXE_safe-chains"))
        .args(args)
        .args(["--explain", command])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .env("HOME", home.path())
        .stdin(Stdio::null())
        .output()
        .expect("run safe-chains");
    format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr))
}

#[test]
fn one_refused_segment_has_its_fix_without_a_heading() {
    for command in ["zzfixtool", "ls && zzfixtool"] {
        let text = explain(&[], command);
        assert!(text.contains("safe-chains config add zzfixtool"), "{text}");
        assert!(!text.contains("For `"), "`{command}` headed its only fix:\n{text}");
    }
}

#[test]
fn several_refused_segments_each_have_a_headed_fix() {
    let text = explain(&[], "zzfirsttool && zzsecondtool");
    let first = text.find("For `zzfirsttool`:").expect("the first segment is headed");
    let second = text.find("For `zzsecondtool`:").expect("the second segment is headed");
    assert!(first < second, "{text}");
    assert!(text.contains("safe-chains config add zzfirsttool"), "{text}");
    assert!(text.contains("safe-chains config add zzsecondtool"), "{text}");
}

#[test]
fn a_command_that_does_not_parse_still_gets_its_fix() {
    let text = explain(&[], "zzfixtool ||| (");
    assert!(text.contains("could not parse the command"), "{text}");
}

#[test]
fn an_unknown_command_is_fixed_by_an_entry_whatever_the_level_in_force() {
    let text = explain(&["--level", "paranoid"], "zzfixtool");
    assert!(text.contains("safe-chains config add zzfixtool"), "{text}");
    assert!(!text.contains("the level in force is stricter"), "{text}");
}

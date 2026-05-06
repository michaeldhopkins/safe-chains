use crate::parse::{Token, WordSet};
use crate::verdict::{SafetyLevel, Verdict};

// Image-processing legacy tools that read input files and write output
// files. SafeWrite under positional-style semantics matching convert.
static MAGICK_SAFE_SUBS: WordSet = WordSet::new(&[
    "combine", "compare", "composite", "convert", "identify",
    "mogrify", "montage", "stream",
]);

// Subcommands intentionally not in coverage:
//   animate, display — long-running interactive GUI viewers
//                      (require X11/screen, never finish on their own)
//   import           — screen capture; reads the user's screen content
//                      to a file, requires a display server
//   conjure          — executes MSL (Magick Scripting Language)
//                      scripts; arbitrary code execution path

// ImageMagick 7's `magick` accepts three documented calling forms:
//
//   1. Subcommand-explicit:   `magick convert in.png out.png`,
//                             `magick identify in.png`, etc.
//   2. Convert-implicit:      `magick in.png -resize 1200x out.png`
//                             (the IM6 `convert` legacy syntax that
//                             IM7 preserves)
//   3. Script-driven:         `magick [...] -script script.msl [...]`
//                             — runs an MSL script. Out of coverage:
//                             arbitrary script execution.
//
// We accept forms 1 and 2 and deny form 3. For the explicit form we
// route to the existing top-level `convert` and `identify` surfaces
// (their TOMLs define the precise flag/positional rules); for the
// other safe legacy tools we apply the same SafeWrite + positional-
// style verdict that the magick.toml subs originally had. For the
// implicit form we treat the entire invocation as `convert` and
// re-dispatch.
//
// Pseudo-format prefixes (`msl:`, `mvg:`, `ephemeral:`, etc.) and
// HTTP-fetched inputs are governed by ImageMagick's policy.xml, the
// same defense layer the standalone `convert` command relies on.
pub fn is_safe_magick(tokens: &[Token]) -> Verdict {
    if tokens.len() == 2
        && matches!(tokens[1].as_str(), "--help" | "-h" | "--version" | "-V")
    {
        return Verdict::Allowed(SafetyLevel::Inert);
    }

    if tokens[1..].iter().any(|t| t.as_str() == "-script") {
        return Verdict::Denied;
    }

    let Some(first) = tokens.get(1) else {
        return Verdict::Denied;
    };
    let first_str = first.as_str();

    if matches!(first_str, "animate" | "conjure" | "display" | "import") {
        return Verdict::Denied;
    }

    if first_str == "convert" || first_str == "identify" {
        let inner = shell_words::join(tokens[1..].iter().map(|t| t.as_str()));
        return crate::command_verdict(&inner);
    }

    if MAGICK_SAFE_SUBS.contains(first_str) {
        return Verdict::Allowed(SafetyLevel::SafeWrite);
    }

    let mut parts: Vec<&str> = Vec::with_capacity(tokens.len());
    parts.push("convert");
    for t in &tokens[1..] {
        parts.push(t.as_str());
    }
    let inner = shell_words::join(parts);
    crate::command_verdict(&inner)
}

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    use crate::verdict::{SafetyLevel, Verdict};

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    fn verdict(cmd: &str) -> Verdict {
        crate::command_verdict(cmd)
    }

    safe! {
        magick_help: "magick --help",
        magick_version: "magick --version",
        magick_convert_explicit: "magick convert in.png out.png",
        magick_convert_resize: "magick convert in.png -resize 1200x out.png",
        magick_identify_explicit: "magick identify photo.jpg",
        magick_implicit_convert: "magick in.png out.png",
        magick_implicit_with_resize: "magick in.png -resize 1200x out.png",
        magick_implicit_avif_to_png: "magick /Users/me/Downloads/x.avif -resize 1200x /tmp/out.png",
        magick_implicit_with_quality: "magick in.jpg -quality 85 out.jpg",
        magick_mogrify: "magick mogrify -resize 50% photo.jpg",
        magick_compare: "magick compare a.png b.png diff.png",
        magick_montage: "magick montage *.png montage.png",
        magick_combine: "magick combine a.png b.png combined.png",
        magick_stream: "magick stream image.png pixels.gray",
    }

    denied! {
        magick_conjure_msl: "magick conjure script.msl",
        magick_display_window: "magick display photo.jpg",
        magick_animate_gif: "magick animate animation.gif",
        magick_import_screen: "magick import screen.png",
        magick_script_flag: "magick -script attack.msl",
        magick_script_after_input: "magick in.png -script attack.msl out.png",
        magick_convert_with_script: "magick convert -script attack.msl",
    }

    #[test]
    fn magick_implicit_is_safewrite() {
        assert_eq!(
            verdict("magick in.png -resize 1200x out.png"),
            Verdict::Allowed(SafetyLevel::SafeWrite),
        );
    }

    #[test]
    fn magick_identify_routes_through_identify_top_level() {
        // identify alone is Inert (read-only inspection); routing
        // `magick identify ...` through the identify top-level should
        // preserve that level.
        assert_eq!(
            verdict("magick identify photo.jpg"),
            Verdict::Allowed(SafetyLevel::Inert),
        );
    }

    #[test]
    fn magick_help_is_inert() {
        assert_eq!(
            verdict("magick --help"),
            Verdict::Allowed(SafetyLevel::Inert),
        );
    }
}

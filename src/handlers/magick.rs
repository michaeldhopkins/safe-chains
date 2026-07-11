//! `magick` is ImageMagick 7's unified entry point. The grammar is
//! routed by the first token: explicit subs (`magick convert ...`)
//! match a TOML-declared sub; bare diagnostic flags match the fallback
//! grammar; anything else is the IM6-legacy implicit-convert form
//! (`magick in.png -resize 1200x out.png`), which dispatches through
//! the top-level `convert` command's policy.
//!
//! Two pieces of LOGIC the declarative TOML can't express:
//!   1. `-script` (MSL execution) is denied anywhere in the token
//!      stream — not just as a leading flag.
//!   2. The implicit-convert delegation is gated on the first
//!      positional looking like a file path. Without that gate, bare
//!      words like `animate`, `conjure`, `display`, or `import` would
//!      flow into convert's permissive positional list and be silently
//!      allowed.
//!
//! All sub names, allowed flags, and safety levels live in
//! `commands/magick.toml`.
use crate::parse::Token;
use crate::registry;
use crate::verdict::Verdict;

pub fn is_safe_magick(tokens: &[Token]) -> Verdict {
    if tokens.len() < 2 {
        return Verdict::Denied; // bare `magick` prints usage — nothing to classify (and `tokens[1]` below would panic)
    }
    if tokens[1..].iter().any(|t| t.as_str() == "-script") {
        return Verdict::Denied;
    }
    if let Some(verdict) = registry::try_sub_dispatch("magick", tokens) {
        return verdict;
    }
    if let Some(v @ Verdict::Allowed(_)) = registry::try_fallback_grammar("magick", tokens) {
        return v;
    }
    // Implicit-convert delegation. If the first token after `magick` is
    // flag-shaped (starts with `-` and isn't bare `-`), pass through to
    // convert without further checks — convert's policy decides. This
    // covers IM6-legacy forms like `magick -list font`, `magick -resize
    // 1200x in.png out.png`, where leading options precede the file.
    // Otherwise the first token must look like a file path; this gate
    // is what excludes bare-word bypasses (`magick conjure script.msl`,
    // `magick display photo.jpg`) that convert's permissive positional
    // policy would otherwise rescue.
    let first = tokens[1].as_str();
    let leading_flag = first.starts_with('-') && first != "-";
    if !leading_flag && !crate::policy::looks_like_path(first) {
        return Verdict::Denied;
    }
    let inner = shell_words::join(
        std::iter::once("convert").chain(tokens[1..].iter().map(|t| t.as_str())),
    );
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

        // IM6-legacy implicit-convert forms with leading options: the
        // first token is flag-shaped, so the path-shape gate is bypassed
        // and delegation flows through convert's policy.
        magick_implicit_list_font: "magick -list font",
        magick_implicit_list_color: "magick -list color",
        magick_implicit_leading_resize: "magick -resize 1200x in.png out.png",
        magick_implicit_leading_quality: "magick -quality 85 in.jpg out.jpg",
    }

    denied! {
        magick_bare_no_args: "magick",
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

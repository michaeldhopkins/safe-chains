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
    let inner = shell_words::join(std::iter::once("convert").chain(tokens[1..].iter().map(|t| t.as_str())));
    crate::command_verdict(&inner)
}

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    use crate::parse::Token;
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
        assert_eq!(verdict("magick in.png -resize 1200x out.png"), Verdict::Allowed(SafetyLevel::SafeWrite),);
    }

    #[test]
    fn magick_identify_routes_through_identify_top_level() {
        // identify alone is Inert (read-only inspection); routing
        // `magick identify ...` through the identify top-level should
        // preserve that level.
        assert_eq!(verdict("magick identify photo.jpg"), Verdict::Allowed(SafetyLevel::Inert),);
    }

    #[test]
    fn magick_help_is_inert() {
        assert_eq!(verdict("magick --help"), Verdict::Allowed(SafetyLevel::Inert),);
    }

    proptest::proptest! {
        // INVARIANT (token-count guard). `is_safe_magick` is dispatched on `tokens[1..]` — with
        // zero or one token there is nothing to classify, so the function must return
        // `Verdict::Denied` for both. (The handler documents this as "bare `magick` prints usage".)
        #[test]
        fn magick_with_fewer_than_two_tokens_is_denied(n in 0..=1usize) {
            let tokens: Vec<Token> = (0..n).map(|_| Token::from_test("magick")).collect();
            proptest::prop_assert_eq!(
                super::is_safe_magick(&tokens),
                Verdict::Denied,
                "magick with {} tokens must be denied",
                n,
            );
        }

        // INVARIANT (MSL-script safety rule). `-script` executes an MSL (Magick Scripting Language)
        // script — arbitrary code execution — so the handler rejects the token ANYWHERE in the
        // tail, not just as a leading flag. The position can be the leading option, after an input
        // path, or after a subcommand; the verdict must be `Denied` in every case.
        #[test]
        fn a_script_flag_anywhere_in_the_tail_denies_magick(
            head in proptest::collection::vec("[a-zA-Z0-9._]{1,8}", 0..=2),
            tail in proptest::collection::vec("[a-zA-Z0-9._]{1,8}", 0..=2),
        ) {
            let mut tokens: Vec<Token> = Vec::with_capacity(2 + head.len() + tail.len());
            tokens.push(Token::from_test("magick"));
            for t in &head { tokens.push(Token::from_test(t)); }
            tokens.push(Token::from_test("-script"));
            for t in &tail { tokens.push(Token::from_test(t)); }
            proptest::prop_assert_eq!(
                super::is_safe_magick(&tokens),
                Verdict::Denied,
                "-script at index {} must deny magick",
                1 + head.len(),
            );
        }

        // INVARIANT (bare-word bypass gate). The implicit-convert delegation passes the tail to
        // `convert` only when the first positional looks like a file path or is a flag; otherwise
        // it denies. A bare alpha word that is not a registered `magick` sub (convert/identify/
        // combine/compare/composite/mogrify/montage/stream) and is not in the fallback's
        // standalone flag set (`--help`/`--version`/`-V`/`-h`) must therefore be denied —
        // regardless of what the rest of the tail looks like. This is the gate that excludes
        // `magick conjure ...`, `magick display ...`, `magick animate ...`, `magick import ...`,
        // which `convert`'s permissive positional list would otherwise rescue.
        #[test]
        fn a_first_token_that_is_a_bare_non_path_word_outside_known_subs_denies_magick(
            bare in "[a-z][a-z0-9]{0,8}",
            tail in proptest::collection::vec("[a-zA-Z0-9._/-]{1,8}", 0..=4),
        ) {
            // The bare word must be neither a registered magick sub nor a fallback standalone flag.
            // The fallback flag set is the long flags (short flags can't be generated by the bare
            // regex above anyway).
            const KNOWN_SUBS: &[&str] = &[
                "convert", "identify", "combine", "compare", "composite",
                "mogrify", "montage", "stream",
            ];
            const FALLBACK_LONG_FLAGS: &[&str] = &["--help", "--version"];
            proptest::prop_assume!(!KNOWN_SUBS.contains(&bare.as_str()));
            proptest::prop_assume!(!FALLBACK_LONG_FLAGS.contains(&bare.as_str()));
            let mut tokens: Vec<Token> = Vec::with_capacity(2 + tail.len());
            tokens.push(Token::from_test("magick"));
            tokens.push(Token::from_test(&bare));
            for t in &tail { tokens.push(Token::from_test(t)); }
            proptest::prop_assert_eq!(
                super::is_safe_magick(&tokens),
                Verdict::Denied,
                "bare non-path first token {:?} must deny magick (tokens: {:?})",
                bare, tokens.iter().map(Token::as_str).collect::<Vec<_>>(),
            );
        }

        // DIFFERENTIAL (delegation contract). When the first token after `magick` is a long flag
        // that does NOT match the fallback's standalone list (`--help`/`--version`), the handler
        // skips the path-shape gate and delegates by re-quoting the tail as a `convert` invocation
        // and passing it to `command_verdict`. The verdict must equal what `command_verdict`
        // returns for that reconstructed `convert …` command — convert's policy IS the verdict.
        #[test]
        fn magick_with_a_non_fallback_long_flag_delegates_to_convert(
            flag in "--[a-z][a-z0-9]{2,8}",
            tail in proptest::collection::vec("[a-zA-Z0-9._/-]{1,8}", 0..=4),
        ) {
            const FALLBACK_LONG_FLAGS: &[&str] = &["--help", "--version"];
            proptest::prop_assume!(!FALLBACK_LONG_FLAGS.contains(&flag.as_str()));
            let mut tokens: Vec<Token> = Vec::with_capacity(2 + tail.len());
            tokens.push(Token::from_test("magick"));
            tokens.push(Token::from_test(&flag));
            for t in &tail { tokens.push(Token::from_test(t)); }
            // Reconstruct the exact string the handler builds: `convert` followed by the same
            // shell_words-joined tail it forwards to `command_verdict`. Tail tokens are restricted
            // to no-whitespace ASCII so `shell_words::join` is just space-separated concatenation,
            // matching the verbatim contract the handler relies on.
            let inner = shell_words::join(
                std::iter::once("convert").chain(tokens[1..].iter().map(Token::as_str)),
            );
            proptest::prop_assert_eq!(
                super::is_safe_magick(&tokens),
                crate::command_verdict(&inner),
                "delegation to convert diverged (inner: {:?})",
                inner,
            );
        }
    }
}

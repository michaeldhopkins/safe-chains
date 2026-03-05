use crate::parse::{Token, WordSet};
use crate::policy::{self, FlagPolicy};

static IDENTIFY_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "-matte", "-moments", "-ping", "-quiet",
        "-regard-warnings", "-unique", "-verbose",
    ]),
    standalone_short: b"",
    valued: WordSet::new(&[
        "-alpha", "-colorspace", "-define", "-density",
        "-depth", "-endian", "-format", "-interlace",
        "-limit", "-precision", "-sampling-factor",
        "-size", "-units", "-virtual-pixel",
    ]),
    valued_short: b"",
    bare: false,
    max_positional: None,
};

pub fn is_safe_magick(tokens: &[Token]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    if tokens[1] == "identify" {
        return policy::check(&tokens[1..], &IDENTIFY_POLICY);
    }
    false
}

pub(crate) fn dispatch(cmd: &str, tokens: &[Token]) -> Option<bool> {
    match cmd {
        "magick" => Some(is_safe_magick(tokens)),
        _ => None,
    }
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    vec![
        crate::docs::CommandDoc::handler("magick",
            "Subcommand: identify."),
    ]
}

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        magick_identify: "magick identify /tmp/image.png",
        magick_identify_verbose: "magick identify -verbose /tmp/image.png",
        magick_identify_multi: "magick identify /tmp/a.png /tmp/b.png",
        magick_identify_format: "magick identify -format '%w %h' /tmp/image.png",
        magick_identify_ping: "magick identify -ping /tmp/image.png",
        magick_identify_density: "magick identify -density 72 /tmp/image.png",
        magick_identify_colorspace: "magick identify -colorspace sRGB /tmp/image.png",
        magick_help: "magick --help",
        magick_version: "magick --version",
    }

    denied! {
        magick_convert_denied: "magick input.png output.jpg",
        magick_mogrify_denied: "magick mogrify -resize 50% image.png",
        magick_composite_denied: "magick composite overlay.png base.png result.png",
        magick_conjure_denied: "magick conjure script.msl",
        bare_magick_denied: "magick",
        magick_identify_unknown_denied: "magick identify --unknown /tmp/image.png",
        magick_identify_write_denied: "magick identify -write /tmp/out.txt /tmp/image.png",
        magick_identify_set_denied: "magick identify -set comment evil /tmp/image.png",
    }
}

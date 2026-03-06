use crate::parse::{Token, WordSet};
use crate::policy::{self, FlagPolicy, FlagStyle};

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
    flag_style: FlagStyle::Strict,
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
pub(super) const REGISTRY: &[super::CommandEntry] = &[
    super::CommandEntry::Subcommand { cmd: "magick", subs: &[
        super::SubEntry::Policy { name: "identify" },
    ]},
];

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
        bare_magick_denied: "magick",
        magick_identify_write_denied: "magick identify -write /tmp/out.txt /tmp/image.png",
        magick_identify_set_denied: "magick identify -set comment evil /tmp/image.png",
    }
}

use crate::command::{CommandDef, SubDef};
use crate::parse::{Token, WordSet};
use crate::policy::{FlagPolicy, FlagStyle};

static IDENTIFY_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "-matte", "-moments", "-ping", "-quiet",
        "-regard-warnings", "-unique", "-verbose",
    ]),
    valued: WordSet::flags(&[
        "-alpha", "-colorspace", "-define", "-density",
        "-depth", "-endian", "-format", "-interlace",
        "-limit", "-precision", "-sampling-factor",
        "-size", "-units", "-virtual-pixel",
    ]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(crate) static MAGICK: CommandDef = CommandDef {
    name: "magick",
    subs: &[
        SubDef::Policy { name: "identify", policy: &IDENTIFY_POLICY },
    ],
    bare_flags: &[],
    help_eligible: true,
    url: "https://imagemagick.org/script/command-line-tools.php",
};

pub(crate) fn dispatch(cmd: &str, tokens: &[Token]) -> Option<bool> {
    if cmd == MAGICK.name {
        Some(MAGICK.check(tokens, &|_| false))
    } else {
        None
    }
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    vec![MAGICK.to_doc()]
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
        magick_identify_write_denied: "magick identify -write /tmp/out.txt /tmp/image.png",
        magick_identify_set_denied: "magick identify -set comment evil /tmp/image.png",
    }
}

use crate::command::FlatDef;
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static SIPS_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "--version", "-V", "-h"]),
    valued: WordSet::flags(&["--getProperty", "-g"]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "sips", policy: &SIPS_POLICY, level: SafetyLevel::Inert, url: "https://ss64.com/mac/sips.html", aliases: &[] },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        sips_get_width: "sips -g pixelWidth image.jpg",
        sips_get_height: "sips -g pixelHeight image.jpg",
        sips_get_multiple: "sips -g pixelWidth -g pixelHeight image.jpg",
        sips_get_long: "sips --getProperty pixelWidth image.jpg",
        sips_get_format: "sips -g format image.png",
        sips_get_dpi: "sips -g dpiWidth -g dpiHeight image.jpg",
        sips_help: "sips --help",
        sips_version: "sips --version",
    }

    denied! {
        sips_bare: "sips",
        sips_set_format: "sips -s format png image.jpg",
        sips_rotate: "sips -r 90 image.jpg",
        sips_flip: "sips -f horizontal image.jpg",
        sips_resize: "sips -z 100 100 image.jpg",
        sips_resize_max: "sips -Z 500 image.jpg",
        sips_output: "sips -g pixelWidth image.jpg --out output.jpg",
        sips_set_property: "sips --setProperty format png image.jpg",
        sips_resample: "sips --resampleWidth 800 image.jpg",
    }
}

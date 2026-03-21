use crate::command::FlatDef;
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static XMLLINT_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--auto", "--catalogs", "--compress", "--copy",
        "--debug", "--debugent", "--dropdtd", "--format",
        "--help", "--html", "--htmlout", "--huge", "--load-trace",
        "--loaddtd", "--memory", "--noblanks", "--nocatalogs",
        "--nocdata", "--nocompact", "--nodefdtd", "--noenc",
        "--noent", "--nonet", "--noout", "--nowarning",
        "--nowrap", "--nsclean", "--oldxml10", "--postvalid",
        "--push", "--pushsmall", "--quiet", "--recover",
        "--repeat", "--sax", "--sax1", "--stream",
        "--testIO", "--timing", "--valid", "--version",
        "--walker", "--xinclude", "--xmlout",
        "-V", "-h",
    ]),
    valued: WordSet::flags(&[
        "--dtdvalid", "--dtdvalidfpi", "--encode",
        "--maxmem", "--path", "--pattern",
        "--pretty", "--relaxng", "--schema",
        "--schematron", "--xpath",
    ]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "xmllint", policy: &XMLLINT_POLICY, level: SafetyLevel::Inert, url: "https://gnome.pages.gitlab.gnome.org/libxml2/xmllint.html", aliases: &[] },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        xmllint_read: "xmllint --xpath '//name' file.xml",
        xmllint_format: "xmllint --format file.xml",
    }

    denied! {
        xmllint_output_denied: "xmllint --output result.xml file.xml",
        xmllint_output_eq_denied: "xmllint --output=result.xml file.xml",
    }
}

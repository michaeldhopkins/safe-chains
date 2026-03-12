use crate::command::FlatDef;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static RUBY_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["-v"]),
    valued: WordSet::flags(&[]),
    bare: false,
    max_positional: Some(0),
    flag_style: FlagStyle::Strict,
};

static RI_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--all", "--interactive", "--list", "--no-pager",
        "-a", "-i", "-l",
    ]),
    valued: WordSet::flags(&[
        "--format", "--width",
        "-f", "-w",
    ]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::ruby) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "ruby", policy: &RUBY_POLICY, help_eligible: true, url: "https://www.ruby-lang.org/en/documentation/", aliases: &[] },
    FlatDef { name: "ri", policy: &RI_POLICY, help_eligible: false, url: "https://ruby.github.io/rdoc/RI_md.html", aliases: &[] },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        ruby_version: "ruby -v",
        ruby_version_long: "ruby --version",
        ruby_help: "ruby --help",
        ri_bare: "ri",
        ri_class: "ri String",
        ri_method: "ri Array#map",
        ri_list: "ri --list",
        ri_all: "ri --all String",
        ri_format: "ri --format markdown String",
        ri_interactive: "ri -i",
    }

    denied! {
        ruby_bare_denied: "ruby",
        ruby_script_denied: "ruby script.rb",
        ruby_eval_denied: "ruby -e 'puts 1'",
        ruby_inline_denied: "ruby -e 'system(\"rm -rf /\")'",
        ri_server_denied: "ri --server 8080",
    }
}

use crate::command::{CommandDef, SubDef};
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static RUSTUP_SHOW_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--installed", "-v"]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static RUSTUP_WHICH_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[]),
    valued: WordSet::flags(&["--toolchain"]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static RUSTUP_DOC_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--alloc", "--book", "--cargo", "--core", "--edition-guide",
        "--embedded-book", "--nomicon", "--path", "--proc_macro",
        "--reference", "--rust-by-example", "--rustc", "--rustdoc",
        "--std", "--test", "--unstable-book",
    ]),
    valued: WordSet::flags(&["--toolchain"]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static RUSTUP_LIST_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--installed", "-v"]),
    valued: WordSet::flags(&["--toolchain"]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(crate) static RUSTUP: CommandDef = CommandDef {
    name: "rustup",
    subs: &[
        SubDef::Nested { name: "component", subs: &[
            SubDef::Policy { name: "list", policy: &RUSTUP_LIST_POLICY },
        ]},
        SubDef::Policy { name: "doc", policy: &RUSTUP_DOC_POLICY },
        SubDef::Delegation { name: "run", skip: 2, doc: "run <toolchain> delegates to inner command." },
        SubDef::Policy { name: "show", policy: &RUSTUP_SHOW_POLICY },
        SubDef::Nested { name: "target", subs: &[
            SubDef::Policy { name: "list", policy: &RUSTUP_LIST_POLICY },
        ]},
        SubDef::Nested { name: "toolchain", subs: &[
            SubDef::Policy { name: "list", policy: &RUSTUP_LIST_POLICY },
        ]},
        SubDef::Policy { name: "which", policy: &RUSTUP_WHICH_POLICY },
    ],
    bare_flags: &[],
    help_eligible: true,
    url: "https://rust-lang.github.io/rustup/",
};

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        rustup_show: "rustup show",
        rustup_show_installed: "rustup show --installed",
        rustup_which: "rustup which rustc",
        rustup_which_toolchain: "rustup which --toolchain nightly rustc",
        rustup_doc: "rustup doc",
        rustup_doc_std: "rustup doc --std",
        rustup_doc_book: "rustup doc --book",
        rustup_doc_path: "rustup doc --path",
        rustup_version: "rustup --version",
        rustup_component_list: "rustup component list",
        rustup_component_list_installed: "rustup component list --installed",
        rustup_target_list: "rustup target list",
        rustup_target_list_installed: "rustup target list --installed",
        rustup_toolchain_list: "rustup toolchain list",
        rustup_toolchain_list_verbose: "rustup toolchain list -v",
        rustup_run_cargo_test: "rustup run nightly cargo test",
        rustup_run_cargo_clippy: "rustup run stable cargo clippy -- -D warnings",
        rustup_run_cargo_fmt_check: "rustup run nightly cargo fmt --check",
        rustup_run_env_cargo_test: "rustup run stable env FOO=bar cargo test",
    }

    denied! {
        rustup_run_rustc_version_denied: "rustup run stable rustc --version",
        rustup_run_cargo_fmt_denied: "rustup run nightly cargo fmt",
        rustup_run_unsafe_inner_denied: "rustup run stable rm -rf /",
        rustup_run_no_inner_denied: "rustup run stable",
        rustup_run_no_toolchain_denied: "rustup run",
        rustup_run_cargo_publish_denied: "rustup run nightly cargo publish",
        rustup_run_bash_c_denied: "rustup run stable bash -c 'rm -rf /'",
        rustup_run_env_unsafe_denied: "rustup run stable env rm foo",
        rustup_run_nested_denied: "rustup run nightly rustup run stable rm -rf /",
    }
}

use crate::command::FlatDef;
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static JAVAP_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--help", "--version",
        "-V", "-c", "-constants", "-h", "-l", "-p", "-private", "-protected",
        "-public", "-s", "-sysinfo", "-v", "-verbose",
    ]),
    valued: WordSet::flags(&[
        "--module", "-bootclasspath", "-classpath", "-cp", "-m",
    ]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub static DEFS: &[FlatDef] = &[
    FlatDef {
        name: "javap",
        policy: &JAVAP_POLICY,
        level: SafetyLevel::Inert,
        url: "https://docs.oracle.com/en/java/javase/21/docs/specs/man/javap.html",
        aliases: &[],
    },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        javap_class: "javap java.lang.String",
        javap_bytecode: "javap -c java.lang.String",
        javap_private: "javap -p java.lang.String",
        javap_verbose: "javap -v java.lang.String",
        javap_signatures: "javap -s java.lang.String",
        javap_constants: "javap -constants java.lang.String",
        javap_classpath: "javap -classpath lib/ com.example.Foo",
        javap_cp: "javap -cp build/classes com.example.Foo",
        javap_version: "javap --version",
        javap_help: "javap --help",
    }

    denied! {
        javap_bare_denied: "javap",
        javap_unknown_denied: "javap --unknown-flag",
    }
}

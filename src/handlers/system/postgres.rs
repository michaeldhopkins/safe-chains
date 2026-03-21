use crate::command::FlatDef;
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static PSQL_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "--version", "-V", "-h"]),
    valued: WordSet::flags(&[]),
    bare: false,
    max_positional: Some(0),
    flag_style: FlagStyle::Strict,
};

static PG_ISREADY_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--help", "--quiet", "--version",
        "-V", "-q",
    ]),
    valued: WordSet::flags(&[
        "--dbname", "--host", "--port", "--timeout", "--username",
        "-U", "-d", "-h", "-p", "-t",
    ]),
    bare: true,
    max_positional: Some(0),
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::system) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "psql", policy: &PSQL_POLICY, level: SafetyLevel::Inert, url: "https://www.postgresql.org/docs/current/app-psql.html", aliases: &[] },
    FlatDef { name: "pg_isready", policy: &PG_ISREADY_POLICY, level: SafetyLevel::Inert, url: "https://www.postgresql.org/docs/current/app-pg-isready.html", aliases: &[] },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        psql_version: "psql --version",
        psql_help: "psql --help",
        pg_isready_bare: "pg_isready",
        pg_isready_host: "pg_isready -h localhost",
        pg_isready_port: "pg_isready -p 5432",
        pg_isready_quiet: "pg_isready -q",
        pg_isready_full: "pg_isready -h localhost -p 5432 -U postgres -d mydb -t 5",
    }

    denied! {
        psql_bare_denied: "psql",
        psql_db_denied: "psql mydb",
        psql_command_denied: "psql -c 'DROP TABLE users'",
        psql_file_denied: "psql -f script.sql",
    }
}

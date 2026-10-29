//! The Gemfile Bundler loads, found as Bundler finds it.
//!
//! Every subcommand that loads the bundle evaluates the Gemfile as Ruby, and Bundler looks for it
//! (`gems.rb` or `Gemfile`) in the folder it runs in and then in each folder above. A project with
//! no Gemfile of its own therefore loads one from outside it. The Gemfile found is judged as code
//! run from a named file, after symbolic links are resolved; none found means Bundler stops before
//! loading anything.

use crate::parse::Token;

const LOADS_THE_BUNDLE: [&str; 6] = ["check", "exec", "info", "install", "list", "show"];

/// Whether a Bundler line loads a Gemfile that lies outside the project.
pub(crate) fn loads_a_gemfile_out_of_sight(tokens: &[Token]) -> bool {
    let Some(sub) = tokens.iter().skip(1).position(|t| !t.as_str().starts_with('-')).map(|at| at + 1) else {
        return false;
    };
    if !LOADS_THE_BUNDLE.contains(&tokens[sub].as_str()) {
        return false;
    }
    let names_its_gemfile =
        tokens[sub].as_str() != "exec" && tokens.iter().any(|t| t.as_str() == "--gemfile" || t.as_str().starts_with("--gemfile="));
    if names_its_gemfile {
        return false;
    }
    nearest_gemfile().is_some_and(|path| !crate::engine::resolve::execute_file_verdict(&path.to_string_lossy()).is_allowed())
}

fn nearest_gemfile() -> Option<std::path::PathBuf> {
    let cwd = crate::pathctx::cwd()
        .filter(|c| c.starts_with('/') && c != crate::pathctx::UNRESOLVED_CWD && !crate::pathctx::folder::is_unknown(c))?;
    let start = std::fs::canonicalize(cwd).ok()?;
    start
        .ancestors()
        .find_map(|dir| ["gems.rb", "Gemfile"].iter().map(|name| dir.join(name)).find(|path| path.is_file()))
}

//! perl's module flags. `-MMod` and `-mMod` load a module and run its code; `-I DIR` and
//! `-Mlib=DIR` decide which directories that code comes from. An installed module is perl's own
//! code, as trusted as perl itself, but a module found in a directory the command names is that
//! directory's code, so those directories are reported for the engine to gate as code it runs.

use crate::parse::Token;

/// Reads the flag at `tokens[i]` when it is `-M`, `-m` or `-I`, glued (`-Ilib`) or split (`-I lib`).
/// `Some(Some(n))`: it used `n` tokens and any directory it names is in `load_paths`.
/// `Some(None)`: not one of these flags. `None`: a form not modeled (a missing value, or `lib`
/// given its directories some other way than `lib=A,B`), so the caller worst-cases.
pub(crate) fn load_flag(tokens: &[Token], i: usize, load_paths: &mut Vec<String>) -> Option<Option<usize>> {
    let token = tokens.get(i)?.as_str();
    let Some(flag @ ('M' | 'm' | 'I')) = token.strip_prefix('-').and_then(|r| r.chars().next()) else {
        return Some(None);
    };
    let glued = &token[2..];
    let (value, used) = if glued.is_empty() { (tokens.get(i + 1)?.as_str(), 2) } else { (glued, 1) };
    if flag == 'I' {
        load_paths.push(value.to_string());
        return Some(Some(used));
    }
    let module = value.trim_start_matches('-');
    if let Some(dirs) = module.strip_prefix("lib=") {
        load_paths.extend(dirs.split(',').filter(|d| !d.is_empty()).map(str::to_string));
    } else if module
        .strip_prefix("lib")
        .is_some_and(|rest| rest.starts_with(|c: char| !c.is_alphanumeric() && c != ':' && c != '_'))
    {
        return None;
    }
    Some(Some(used))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(line: &str) -> (Option<Option<usize>>, Vec<String>) {
        let tokens: Vec<Token> = line.split(' ').map(|w| Token::from_test(&w.replace('~', " "))).collect();
        let mut dirs = Vec::new();
        (load_flag(&tokens, 1, &mut dirs), dirs)
    }

    #[test]
    fn each_module_flag_spelling() {
        let dirs = |d: &[&str]| d.iter().map(|s| (*s).to_string()).collect::<Vec<_>>();
        assert_eq!(run("perl -I. -e 1"), (Some(Some(1)), dirs(&["."])));
        assert_eq!(run("perl -I lib -e 1"), (Some(Some(2)), dirs(&["lib"])));
        assert_eq!(run("perl -MList::Util -e 1"), (Some(Some(1)), dirs(&[])));
        assert_eq!(run("perl -m Foo -e 1"), (Some(Some(2)), dirs(&[])));
        assert_eq!(run("perl -Mlib=.,t/lib -e 1"), (Some(Some(1)), dirs(&[".", "t/lib"])));
        assert_eq!(run("perl -M-lib=x -e 1"), (Some(Some(1)), dirs(&["x"])));
        assert_eq!(run("perl -Mlibrary -e 1"), (Some(Some(1)), dirs(&[])));
        assert_eq!(run("perl -Mlib~qw(.) -e 1"), (None, dirs(&[])));
        assert_eq!(run("perl -I"), (None, dirs(&[])));
        assert_eq!(run("perl -pe 1"), (Some(None), dirs(&[])));
    }

    /// Loading modules from a directory is running that directory's code: at every level, the
    /// verdict matches running a script from it.
    #[test]
    fn a_module_directory_is_judged_as_a_script_directory() {
        for level in ["paranoid", "reader", "editor", "developer", "local-admin"] {
            let (ceiling, engine) = crate::level_ceiling(level).expect("known level");
            let allowed = |cmd: &str| crate::command_verdict_ceilinged(cmd, ceiling, engine).is_allowed();
            for dir in [".", "lib", "/tmp/m", "/var/m", "/usr/local/lib"] {
                let script = allowed(&format!("ruby {dir}/x.rb"));
                for perl in [format!("perl -I{dir} -MMod -pe 1 ./f"), format!("perl -Mlib={dir} -MMod -pe 1 ./f")] {
                    assert_eq!(allowed(&perl), script, "{level}: `{perl}` vs `ruby {dir}/x.rb`");
                }
            }
        }
    }
}

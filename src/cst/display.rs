use std::fmt;
use super::*;

fn write_sep(f: &mut fmt::Formatter<'_>, trailing_op: Option<ListOp>) -> fmt::Result {
    if !matches!(trailing_op, Some(ListOp::Semi)) {
        f.write_str(";")?;
    }
    Ok(())
}

fn write_body(f: &mut fmt::Formatter<'_>, script: &Script) -> fmt::Result {
    for (i, stmt) in script.0.iter().enumerate() {
        if i > 0 {
            f.write_str(" ")?;
        }
        write!(f, "{}", stmt.pipeline)?;
        match &stmt.op {
            Some(ListOp::Semi) | None => f.write_str(";")?,
            Some(op) => write!(f, " {op}")?,
        }
    }
    Ok(())
}

impl fmt::Display for Script {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, stmt) in self.0.iter().enumerate() {
            if i > 0 {
                f.write_str(" ")?;
            }
            write!(f, "{}", stmt.pipeline)?;
            match &stmt.op {
                Some(ListOp::Semi) => f.write_str(";")?,
                Some(op) => write!(f, " {op}")?,
                None => {}
            }
        }
        Ok(())
    }
}

impl fmt::Display for ListOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ListOp::And => f.write_str("&&"),
            ListOp::Or => f.write_str("||"),
            ListOp::Semi => f.write_str(";"),
            ListOp::Amp => f.write_str("&"),
        }
    }
}

impl fmt::Display for Pipeline {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.bang {
            f.write_str("! ")?;
        }
        for (i, cmd) in self.commands.iter().enumerate() {
            if i > 0 {
                f.write_str(" | ")?;
            }
            write!(f, "{cmd}")?;
        }
        Ok(())
    }
}

impl fmt::Display for Cmd {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Cmd::Simple(s) => write!(f, "{s}"),
            Cmd::Subshell(s) => write!(f, "({s})"),
            Cmd::For { var, items, body } => {
                write!(f, "for {var}")?;
                if !items.is_empty() {
                    f.write_str(" in")?;
                    for item in items {
                        write!(f, " {item}")?;
                    }
                }
                write_sep(f, None)?;
                write!(f, " do ")?;
                write_body(f, body)?;
                f.write_str(" done")
            }
            Cmd::While { cond, body } => {
                write!(f, "while {cond}")?;
                write_sep(f, cond.0.last().and_then(|s| s.op))?;
                write!(f, " do ")?;
                write_body(f, body)?;
                f.write_str(" done")
            }
            Cmd::Until { cond, body } => {
                write!(f, "until {cond}")?;
                write_sep(f, cond.0.last().and_then(|s| s.op))?;
                write!(f, " do ")?;
                write_body(f, body)?;
                f.write_str(" done")
            }
            Cmd::If { branches, else_body } => {
                for (i, branch) in branches.iter().enumerate() {
                    if i == 0 {
                        write!(f, "if {}", branch.cond)?;
                    } else {
                        write!(f, " elif {}", branch.cond)?;
                    }
                    write_sep(f, branch.cond.0.last().and_then(|s| s.op))?;
                    write!(f, " then ")?;
                    write_body(f, &branch.body)?;
                    f.write_str("")?;
                }
                if let Some(eb) = else_body {
                    write!(f, " else ")?;
                    write_body(f, eb)?;
                }
                f.write_str(" fi")
            }
        }
    }
}

impl fmt::Display for SimpleCmd {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut first = true;
        for (name, val) in &self.env {
            if !first { f.write_str(" ")?; }
            first = false;
            write!(f, "{name}={val}")?;
        }
        for w in &self.words {
            if !first { f.write_str(" ")?; }
            first = false;
            write!(f, "{w}")?;
        }
        for r in &self.redirs {
            if !first { f.write_str(" ")?; }
            first = false;
            write!(f, "{r}")?;
        }
        Ok(())
    }
}

impl fmt::Display for Word {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for part in &self.0 {
            write!(f, "{part}")?;
        }
        Ok(())
    }
}

impl fmt::Display for WordPart {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WordPart::Lit(s) => f.write_str(s),
            WordPart::Escape(c) => write!(f, "\\{c}"),
            WordPart::SQuote(s) => write!(f, "'{s}'"),
            WordPart::DQuote(w) => write!(f, "\"{w}\""),
            WordPart::CmdSub(s) => {
                let rendered = s.to_string();
                if rendered.starts_with('(') {
                    write!(f, "$( {rendered})")
                } else {
                    write!(f, "$({rendered})")
                }
            }
            WordPart::Backtick(s) => write!(f, "`{s}`"),
            WordPart::Arith(s) => write!(f, "$(({s}))"),
        }
    }
}

impl fmt::Display for Redir {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Redir::Write { fd, target, append } => {
                if *fd != 1 { write!(f, "{fd}")?; }
                if *append { write!(f, ">> {target}") } else { write!(f, "> {target}") }
            }
            Redir::Read { fd, target } => {
                if *fd != 0 { write!(f, "{fd}")?; }
                write!(f, "< {target}")
            }
            Redir::HereStr(w) => write!(f, "<<< {w}"),
            Redir::HereDoc { delimiter, strip_tabs } => {
                if *strip_tabs { write!(f, "<<-{delimiter}") } else { write!(f, "<<{delimiter}") }
            }
            Redir::DupFd { src, dst } => {
                if *src != 1 { write!(f, "{src}")?; }
                write!(f, ">&{dst}")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::cst::parse;

    #[test]
    fn display_simple() {
        let s = parse("echo hello").unwrap();
        assert_eq!(s.to_string(), "echo hello");
    }

    #[test]
    fn display_pipeline() {
        let s = parse("grep foo | head -5").unwrap();
        assert_eq!(s.to_string(), "grep foo | head -5");
    }

    #[test]
    fn display_sequence() {
        let s = parse("ls && echo done").unwrap();
        assert_eq!(s.to_string(), "ls && echo done");
    }

    #[test]
    fn display_single_quoted() {
        let s = parse("echo 'hello world'").unwrap();
        assert_eq!(s.to_string(), "echo 'hello world'");
    }

    #[test]
    fn display_double_quoted() {
        let s = parse("echo \"hello world\"").unwrap();
        assert_eq!(s.to_string(), "echo \"hello world\"");
    }

    #[test]
    fn display_redirect() {
        let s = parse("echo hello > /dev/null").unwrap();
        assert_eq!(s.to_string(), "echo hello > /dev/null");
    }

    #[test]
    fn display_fd_redirect() {
        let s = parse("echo hello 2>&1").unwrap();
        assert_eq!(s.to_string(), "echo hello 2>&1");
    }

    #[test]
    fn display_cmd_sub() {
        let s = parse("echo $(ls)").unwrap();
        assert_eq!(s.to_string(), "echo $(ls)");
    }

    #[test]
    fn display_for() {
        let s = parse("for x in 1 2 3; do echo $x; done").unwrap();
        assert_eq!(s.to_string(), "for x in 1 2 3; do echo $x; done");
    }

    #[test]
    fn display_if() {
        let s = parse("if true; then echo yes; else echo no; fi").unwrap();
        assert_eq!(s.to_string(), "if true; then echo yes; else echo no; fi");
    }

    #[test]
    fn display_env_prefix() {
        let s = parse("FOO=bar ls").unwrap();
        assert_eq!(s.to_string(), "FOO=bar ls");
    }

    #[test]
    fn display_subshell() {
        let s = parse("(echo hello)").unwrap();
        assert_eq!(s.to_string(), "(echo hello)");
    }

    #[test]
    fn display_negation() {
        let s = parse("! echo hello").unwrap();
        assert_eq!(s.to_string(), "! echo hello");
    }
}

pub(crate) mod check;
#[cfg(test)]
mod display;
mod eval;
mod parse;
#[cfg(test)]
mod proptests;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Script(pub Vec<Stmt>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stmt {
    pub pipeline: Pipeline,
    pub op: Option<ListOp>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListOp {
    And,
    Or,
    Semi,
    Amp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pipeline {
    pub bang: bool,
    pub commands: Vec<Cmd>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Cmd {
    Simple(SimpleCmd),
    Subshell(Script),
    For {
        var: String,
        items: Vec<Word>,
        body: Script,
    },
    While {
        cond: Script,
        body: Script,
    },
    Until {
        cond: Script,
        body: Script,
    },
    If {
        branches: Vec<Branch>,
        else_body: Option<Script>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Branch {
    pub cond: Script,
    pub body: Script,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimpleCmd {
    pub env: Vec<(String, Word)>,
    pub words: Vec<Word>,
    pub redirs: Vec<Redir>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Word(pub Vec<WordPart>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WordPart {
    Lit(String),
    Escape(char),
    SQuote(String),
    DQuote(Word),
    CmdSub(Script),
    Backtick(String),
    Arith(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Redir {
    Write {
        fd: u32,
        target: Word,
        append: bool,
    },
    Read {
        fd: u32,
        target: Word,
    },
    HereStr(Word),
    HereDoc {
        delimiter: String,
        strip_tabs: bool,
    },
    DupFd {
        src: u32,
        dst: String,
    },
}

pub use check::{command_verdict, is_safe_command, is_safe_pipeline};
pub use parse::parse;

impl Word {
    pub fn eval(&self) -> String {
        eval::eval_word(self)
    }

    pub fn literal(s: &str) -> Self {
        Word(vec![WordPart::Lit(s.to_string())])
    }

    pub fn normalize(&self) -> Self {
        let mut parts = Vec::new();
        for part in &self.0 {
            let part = match part {
                WordPart::DQuote(inner) => WordPart::DQuote(inner.normalize()),
                WordPart::CmdSub(s) => WordPart::CmdSub(s.normalize()),
                other => other.clone(),
            };
            if let WordPart::Lit(s) = &part
                && let Some(WordPart::Lit(prev)) = parts.last_mut()
            {
                prev.push_str(s);
                continue;
            }
            parts.push(part);
        }
        Word(parts)
    }
}

impl Script {
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn normalize(&self) -> Self {
        Script(
            self.0
                .iter()
                .map(|stmt| Stmt {
                    pipeline: stmt.pipeline.normalize(),
                    op: stmt.op,
                })
                .collect(),
        )
    }

    pub fn normalize_as_body(&self) -> Self {
        let mut s = self.normalize();
        if let Some(last) = s.0.last_mut()
            && last.op.is_none()
        {
            last.op = Some(ListOp::Semi);
        }
        s
    }
}

impl Pipeline {
    fn normalize(&self) -> Self {
        Pipeline {
            bang: self.bang,
            commands: self.commands.iter().map(|c| c.normalize()).collect(),
        }
    }
}

impl Cmd {
    fn normalize(&self) -> Self {
        match self {
            Cmd::Simple(s) => Cmd::Simple(s.normalize()),
            Cmd::Subshell(s) => Cmd::Subshell(s.normalize()),
            Cmd::For { var, items, body } => Cmd::For {
                var: var.clone(),
                items: items.iter().map(|w| w.normalize()).collect(),
                body: body.normalize_as_body(),
            },
            Cmd::While { cond, body } => Cmd::While {
                cond: cond.normalize_as_body(),
                body: body.normalize_as_body(),
            },
            Cmd::Until { cond, body } => Cmd::Until {
                cond: cond.normalize_as_body(),
                body: body.normalize_as_body(),
            },
            Cmd::If { branches, else_body } => Cmd::If {
                branches: branches
                    .iter()
                    .map(|b| Branch {
                        cond: b.cond.normalize_as_body(),
                        body: b.body.normalize_as_body(),
                    })
                    .collect(),
                else_body: else_body.as_ref().map(|e| e.normalize_as_body()),
            },
        }
    }
}

impl SimpleCmd {
    fn normalize(&self) -> Self {
        SimpleCmd {
            env: self
                .env
                .iter()
                .map(|(k, v)| (k.clone(), v.normalize()))
                .collect(),
            words: self.words.iter().map(|w| w.normalize()).collect(),
            redirs: self
                .redirs
                .iter()
                .map(|r| match r {
                    Redir::Write { fd, target, append } => Redir::Write {
                        fd: *fd,
                        target: target.normalize(),
                        append: *append,
                    },
                    Redir::Read { fd, target } => Redir::Read {
                        fd: *fd,
                        target: target.normalize(),
                    },
                    Redir::HereStr(w) => Redir::HereStr(w.normalize()),
                    Redir::HereDoc { .. } | Redir::DupFd { .. } => r.clone(),
                })
                .collect(),
        }
    }
}

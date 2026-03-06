#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShellUnit {
    Simple(String),
    For {
        header: String,
        body: Vec<ShellUnit>,
    },
    Loop {
        kind: LoopKind,
        condition: Vec<ShellUnit>,
        body: Vec<ShellUnit>,
    },
    If {
        branches: Vec<Branch>,
        else_body: Vec<ShellUnit>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopKind {
    While,
    Until,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Branch {
    pub condition: Vec<ShellUnit>,
    pub body: Vec<ShellUnit>,
}

pub fn parse<S: AsRef<str>>(segments: &[S]) -> Option<Vec<ShellUnit>> {
    let strs: Vec<&str> = segments.iter().map(|s| s.as_ref()).collect();
    parse_inner(&strs)
}

fn first_word(s: &str) -> &str {
    s.split_whitespace().next().unwrap_or("")
}

fn rest_after_first_word(s: &str) -> &str {
    let s = s.trim();
    match s.find(char::is_whitespace) {
        Some(pos) => s[pos..].trim_start(),
        None => "",
    }
}

fn opens_loop(s: &str) -> bool {
    let fw = first_word(s);
    matches!(fw, "for" | "while" | "until")
        || (matches!(fw, "do" | "then" | "else" | "elif") && {
            let rest = rest_after_first_word(s);
            !rest.is_empty() && opens_loop(rest)
        })
}

fn opens_if(s: &str) -> bool {
    let fw = first_word(s);
    fw == "if"
        || (matches!(fw, "do" | "then" | "else" | "elif") && {
            let rest = rest_after_first_word(s);
            !rest.is_empty() && opens_if(rest)
        })
}

fn find_do(segments: &[&str]) -> Option<usize> {
    (1..segments.len()).find(|&i| first_word(segments[i]) == "do")
}

fn find_closing_done(segments: &[&str], do_pos: usize) -> Option<usize> {
    let do_rest = rest_after_first_word(segments[do_pos]);
    let mut depth: usize = 1;
    if !do_rest.is_empty() && opens_loop(do_rest) {
        depth += 1;
    }
    for (i, seg) in segments.iter().enumerate().skip(do_pos + 1) {
        if opens_loop(seg) {
            depth += 1;
        } else if first_word(seg) == "done" {
            depth -= 1;
            if depth == 0 {
                return Some(i);
            }
        }
    }
    None
}

fn find_closing_fi(segments: &[&str]) -> Option<usize> {
    let mut depth: usize = 1;
    for (i, seg) in segments.iter().enumerate().skip(1) {
        if opens_if(seg) {
            depth += 1;
        } else if first_word(seg) == "fi" {
            depth -= 1;
            if depth == 0 {
                return Some(i);
            }
        }
    }
    None
}

fn extract_body<'a>(segments: &[&'a str], do_pos: usize, close_pos: usize) -> Vec<&'a str> {
    let mut body = Vec::new();
    let do_rest = rest_after_first_word(segments[do_pos]);
    if !do_rest.is_empty() {
        body.push(do_rest);
    }
    body.extend_from_slice(&segments[(do_pos + 1)..close_pos]);
    body
}

fn parse_for(segments: &[&str]) -> Option<(ShellUnit, usize)> {
    let do_pos = find_do(segments)?;
    let done_pos = find_closing_done(segments, do_pos)?;

    let mut header_parts = Vec::new();
    let first_rest = rest_after_first_word(segments[0]);
    if !first_rest.is_empty() {
        header_parts.push(first_rest);
    }
    for seg in &segments[1..do_pos] {
        let trimmed = seg.trim();
        if !trimmed.is_empty() {
            header_parts.push(trimmed);
        }
    }
    let header = header_parts.join(" ");

    let body_segs = extract_body(segments, do_pos, done_pos);
    let body = parse_inner(&body_segs)?;

    Some((ShellUnit::For { header, body }, done_pos + 1))
}

fn parse_loop(segments: &[&str]) -> Option<(ShellUnit, usize)> {
    let kind = match first_word(segments[0]) {
        "while" => LoopKind::While,
        "until" => LoopKind::Until,
        _ => return None,
    };

    let do_pos = find_do(segments)?;
    let done_pos = find_closing_done(segments, do_pos)?;

    let mut cond_segs = Vec::new();
    let first_rest = rest_after_first_word(segments[0]);
    if !first_rest.is_empty() {
        cond_segs.push(first_rest);
    }
    for seg in &segments[1..do_pos] {
        let trimmed = seg.trim();
        if !trimmed.is_empty() {
            cond_segs.push(trimmed);
        }
    }
    let condition = parse_inner(&cond_segs)?;

    let body_segs = extract_body(segments, do_pos, done_pos);
    let body = parse_inner(&body_segs)?;

    Some((
        ShellUnit::Loop {
            kind,
            condition,
            body,
        },
        done_pos + 1,
    ))
}

fn parse_if(segments: &[&str]) -> Option<(ShellUnit, usize)> {
    let fi_pos = find_closing_fi(segments)?;

    let mut depth = 0usize;
    let mut markers: Vec<(usize, &str)> = Vec::new();

    for (i, seg) in segments.iter().enumerate().take(fi_pos + 1) {
        let fw = first_word(seg);

        if depth == 0 && fw == "if" {
            markers.push((i, "if"));
        }
        if depth == 1 {
            match fw {
                "then" | "elif" | "else" => markers.push((i, fw)),
                _ => {}
            }
        }

        if opens_if(seg) {
            depth += 1;
        }
        if fw == "fi" {
            depth = depth.checked_sub(1)?;
        }
    }

    let mut branches = Vec::new();
    let mut else_body = Vec::new();
    let mut mi = 0;

    while mi < markers.len() {
        let (pos, kw) = markers[mi];
        if !matches!(kw, "if" | "elif") {
            return None;
        }
        mi += 1;

        if mi >= markers.len() {
            return None;
        }
        let (then_pos, then_kw) = markers[mi];
        if then_kw != "then" {
            return None;
        }
        mi += 1;

        let mut cond_segs = Vec::new();
        let rest = rest_after_first_word(segments[pos]);
        if !rest.is_empty() {
            cond_segs.push(rest);
        }
        for seg in &segments[(pos + 1)..then_pos] {
            let trimmed = seg.trim();
            if !trimmed.is_empty() {
                cond_segs.push(trimmed);
            }
        }
        let condition = parse_inner(&cond_segs)?;

        let body_end = if mi < markers.len() {
            markers[mi].0
        } else {
            fi_pos
        };

        let mut body_segs = Vec::new();
        let then_rest = rest_after_first_word(segments[then_pos]);
        if !then_rest.is_empty() {
            body_segs.push(then_rest);
        }
        body_segs.extend_from_slice(&segments[(then_pos + 1)..body_end]);
        let body = parse_inner(&body_segs)?;

        branches.push(Branch { condition, body });

        if mi < markers.len() && markers[mi].1 == "else" {
            let else_pos = markers[mi].0;
            let mut else_segs = Vec::new();
            let else_rest = rest_after_first_word(segments[else_pos]);
            if !else_rest.is_empty() {
                else_segs.push(else_rest);
            }
            else_segs.extend_from_slice(&segments[(else_pos + 1)..fi_pos]);
            else_body = parse_inner(&else_segs)?;
            break;
        }
    }

    if branches.is_empty() {
        return None;
    }

    Some((ShellUnit::If { branches, else_body }, fi_pos + 1))
}

fn parse_inner(segments: &[&str]) -> Option<Vec<ShellUnit>> {
    let mut result = Vec::new();
    let mut i = 0;
    while i < segments.len() {
        let seg = segments[i].trim();
        if seg.is_empty() {
            i += 1;
            continue;
        }
        match first_word(seg) {
            "for" => {
                let (unit, consumed) = parse_for(&segments[i..])?;
                result.push(unit);
                i += consumed;
            }
            "while" | "until" => {
                let (unit, consumed) = parse_loop(&segments[i..])?;
                result.push(unit);
                i += consumed;
            }
            "if" => {
                let (unit, consumed) = parse_if(&segments[i..])?;
                result.push(unit);
                i += consumed;
            }
            "do" | "done" | "then" | "elif" | "else" | "fi" => return None,
            _ => {
                result.push(ShellUnit::Simple(seg.to_string()));
                i += 1;
            }
        }
    }
    Some(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn segs(cmd: &str) -> Vec<String> {
        crate::parse::CommandLine::new(cmd).segments().into_iter().map(|s| s.as_str().to_string()).collect()
    }

    #[test]
    fn simple_commands() {
        assert_eq!(
            parse(&segs("echo hello; ls")),
            Some(vec![
                ShellUnit::Simple("echo hello".into()),
                ShellUnit::Simple("ls".into()),
            ])
        );
    }

    #[test]
    fn for_loop() {
        assert_eq!(
            parse(&segs("for x in 1 2 3; do echo $x; done")),
            Some(vec![ShellUnit::For {
                header: "x in 1 2 3".into(),
                body: vec![ShellUnit::Simple("echo $x".into())],
            }])
        );
    }

    #[test]
    fn for_empty_body() {
        assert_eq!(
            parse(&segs("for x in 1 2 3; do; done")),
            Some(vec![ShellUnit::For {
                header: "x in 1 2 3".into(),
                body: vec![],
            }])
        );
    }

    #[test]
    fn for_multi_body() {
        assert_eq!(
            parse(&segs("for f in *.txt; do cat $f | grep pattern; done")),
            Some(vec![ShellUnit::For {
                header: "f in *.txt".into(),
                body: vec![
                    ShellUnit::Simple("cat $f".into()),
                    ShellUnit::Simple("grep pattern".into()),
                ],
            }])
        );
    }

    #[test]
    fn sequential_for_loops() {
        let result = parse(&segs("for x in 1 2; do echo $x; done; for y in a b; do echo $y; done"));
        assert!(result.is_some());
        let units = result.unwrap();
        assert_eq!(units.len(), 2);
        assert!(matches!(&units[0], ShellUnit::For { header, .. } if header == "x in 1 2"));
        assert!(matches!(&units[1], ShellUnit::For { header, .. } if header == "y in a b"));
    }

    #[test]
    fn nested_for_loops() {
        let result = parse(&segs("for x in 1 2; do for y in a b; do echo $x $y; done; done"));
        assert_eq!(
            result,
            Some(vec![ShellUnit::For {
                header: "x in 1 2".into(),
                body: vec![ShellUnit::For {
                    header: "y in a b".into(),
                    body: vec![ShellUnit::Simple("echo $x $y".into())],
                }],
            }])
        );
    }

    #[test]
    fn for_then_command() {
        let result = parse(&segs("for x in 1 2; do echo $x; done && echo finished"));
        assert!(result.is_some());
        let units = result.unwrap();
        assert_eq!(units.len(), 2);
        assert!(matches!(&units[0], ShellUnit::For { .. }));
        assert_eq!(units[1], ShellUnit::Simple("echo finished".into()));
    }

    #[test]
    fn while_loop() {
        assert_eq!(
            parse(&segs("while test -f /tmp/foo; do sleep 1; done")),
            Some(vec![ShellUnit::Loop {
                kind: LoopKind::While,
                condition: vec![ShellUnit::Simple("test -f /tmp/foo".into())],
                body: vec![ShellUnit::Simple("sleep 1".into())],
            }])
        );
    }

    #[test]
    fn until_loop() {
        assert_eq!(
            parse(&segs("until test -f /tmp/ready; do sleep 1; done")),
            Some(vec![ShellUnit::Loop {
                kind: LoopKind::Until,
                condition: vec![ShellUnit::Simple("test -f /tmp/ready".into())],
                body: vec![ShellUnit::Simple("sleep 1".into())],
            }])
        );
    }

    #[test]
    fn if_then_fi() {
        assert_eq!(
            parse(&segs("if test -f foo; then echo exists; fi")),
            Some(vec![ShellUnit::If {
                branches: vec![Branch {
                    condition: vec![ShellUnit::Simple("test -f foo".into())],
                    body: vec![ShellUnit::Simple("echo exists".into())],
                }],
                else_body: vec![],
            }])
        );
    }

    #[test]
    fn if_then_else_fi() {
        assert_eq!(
            parse(&segs("if test -f foo; then echo yes; else echo no; fi")),
            Some(vec![ShellUnit::If {
                branches: vec![Branch {
                    condition: vec![ShellUnit::Simple("test -f foo".into())],
                    body: vec![ShellUnit::Simple("echo yes".into())],
                }],
                else_body: vec![ShellUnit::Simple("echo no".into())],
            }])
        );
    }

    #[test]
    fn if_elif_else() {
        let result = parse(&segs("if test -f a; then echo a; elif test -f b; then echo b; else echo c; fi"));
        assert!(result.is_some());
        let units = result.unwrap();
        assert_eq!(units.len(), 1);
        if let ShellUnit::If { branches, else_body } = &units[0] {
            assert_eq!(branches.len(), 2);
            assert_eq!(else_body, &[ShellUnit::Simple("echo c".into())]);
        } else {
            panic!("expected If");
        }
    }

    #[test]
    fn nested_if_in_for() {
        let result = parse(&segs("for x in 1 2; do if test $x = 1; then echo one; fi; done"));
        assert!(result.is_some());
        let units = result.unwrap();
        assert_eq!(units.len(), 1);
        if let ShellUnit::For { body, .. } = &units[0] {
            assert_eq!(body.len(), 1);
            assert!(matches!(&body[0], ShellUnit::If { .. }));
        } else {
            panic!("expected For");
        }
    }

    #[test]
    fn nested_for_in_if() {
        let result = parse(&segs("if true; then for x in 1 2; do echo $x; done; fi"));
        assert!(result.is_some());
        let units = result.unwrap();
        assert_eq!(units.len(), 1);
        if let ShellUnit::If { branches, .. } = &units[0] {
            assert_eq!(branches.len(), 1);
            assert_eq!(branches[0].body.len(), 1);
            assert!(matches!(&branches[0].body[0], ShellUnit::For { .. }));
        } else {
            panic!("expected If");
        }
    }

    #[test]
    fn keyword_as_data() {
        assert_eq!(
            parse(&segs("echo for; echo done; echo if; echo fi")),
            Some(vec![
                ShellUnit::Simple("echo for".into()),
                ShellUnit::Simple("echo done".into()),
                ShellUnit::Simple("echo if".into()),
                ShellUnit::Simple("echo fi".into()),
            ])
        );
    }

    #[test]
    fn stray_done() {
        assert_eq!(parse(&segs("echo hello; done")), None);
    }

    #[test]
    fn stray_fi() {
        assert_eq!(parse(&segs("fi")), None);
    }

    #[test]
    fn unclosed_for() {
        assert_eq!(parse(&segs("for x in 1 2 3; do echo $x")), None);
    }

    #[test]
    fn unclosed_if() {
        assert_eq!(parse(&segs("if true; then echo hello")), None);
    }

    #[test]
    fn for_missing_do() {
        assert_eq!(parse(&segs("for x in 1 2 3; echo $x; done")), None);
    }

    #[test]
    fn triple_nested_for() {
        let result = parse(&segs(
            "for x in 1; do for y in 2; do for z in 3; do echo $x $y $z; done; done; done"
        ));
        assert!(result.is_some());
        let units = result.unwrap();
        if let ShellUnit::For { body, .. } = &units[0] {
            if let ShellUnit::For { body, .. } = &body[0] {
                if let ShellUnit::For { body, .. } = &body[0] {
                    assert_eq!(body, &[ShellUnit::Simple("echo $x $y $z".into())]);
                } else {
                    panic!("expected innermost For");
                }
            } else {
                panic!("expected middle For");
            }
        } else {
            panic!("expected outer For");
        }
    }

    #[test]
    fn while_negation() {
        let result = parse(&segs("while ! test -f /tmp/done; do sleep 1; done"));
        assert!(result.is_some());
        if let ShellUnit::Loop { condition, .. } = &result.unwrap()[0] {
            assert_eq!(condition, &[ShellUnit::Simple("! test -f /tmp/done".into())]);
        } else {
            panic!("expected Loop");
        }
    }
}

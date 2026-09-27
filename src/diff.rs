use serde::Serialize;
use similar::{ChangeTag, TextDiff};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
pub enum LineKind {
    Added,
    Removed,
    Modified,
    Unchanged,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct DiffLine {
    pub kind: LineKind,
    pub source: String,
    pub target: String,
}

/// Line diff of two canonical texts; delete/insert runs are paired into
/// Modified rows by row key (identifier after the COLUMN/CONSTRAINT/INDEX
/// prefix). Unmatched rows stay Removed/Added — a dropped row next to a
/// changed one is not misreported as a modification.
pub fn diff_lines(source: &str, target: &str) -> Vec<DiffLine> {
    let diff = TextDiff::from_lines(source, target);
    let mut out = Vec::new();
    let mut dels: Vec<String> = Vec::new();
    let mut ins: Vec<String> = Vec::new();
    for ch in diff.iter_all_changes() {
        match ch.tag() {
            ChangeTag::Equal => {
                flush(&mut out, &mut dels, &mut ins);
                let s = ch.value().trim_end().to_string();
                out.push(DiffLine { kind: LineKind::Unchanged, source: s.clone(), target: s });
            }
            ChangeTag::Delete => dels.push(ch.value().trim_end().to_string()),
            ChangeTag::Insert => ins.push(ch.value().trim_end().to_string()),
        }
    }
    flush(&mut out, &mut dels, &mut ins);
    out
}

fn flush(out: &mut Vec<DiffLine>, dels: &mut Vec<String>, ins: &mut Vec<String>) {
    let mut free: Vec<String> = Vec::new();
    for s in dels.drain(..) {
        match ins.iter().position(|t| row_key(t) == row_key(&s)) {
            Some(j) => out.push(DiffLine {
                kind: LineKind::Modified,
                source: s,
                target: ins.remove(j),
            }),
            None => free.push(s),
        }
    }
    for s in free {
        out.push(DiffLine { kind: LineKind::Removed, source: s, target: String::new() });
    }
    for t in ins.drain(..) {
        out.push(DiffLine { kind: LineKind::Added, source: String::new(), target: t });
    }
}

/// Identifier used to pair a deleted row with its modified replacement:
/// the name token after a COLUMN/CONSTRAINT/INDEX-style prefix. Rows
/// without a prefix key on their first token.
fn row_key(s: &str) -> Option<&str> {
    let mut w = s.split_whitespace();
    match w.next()? {
        "CONSTRAINT" => {
            w.next()?; // constraint kind (PK/FK/CHECK/...)
            w.next()
        }
        "COLUMN" | "INDEX" | "PK" | "FK" | "CHECK" | "DEFAULT" => w.next(),
        k => Some(k),
    }
}

/// "- name NVARCHAR(50)" style entry; strips COLUMN/INDEX/PK/FK/CHECK prefixes, truncates 64 chars.
pub fn summary_entry(kind: LineKind, source: &str, target: &str) -> String {
    fn short(s: &str) -> String {
        let s = s.trim();
        let s = ["COLUMN ", "INDEX ", "CONSTRAINT ", "PK ", "FK ", "CHECK ", "DEFAULT "]
            .iter()
            .find_map(|p| s.strip_prefix(p))
            .unwrap_or(s);
        if s.chars().count() > 64 {
            format!("{}…", s.chars().take(63).collect::<String>())
        } else {
            s.to_string()
        }
    }
    match kind {
        LineKind::Removed => format!("- {}", short(source)),
        LineKind::Added => format!("+ {}", short(target)),
        LineKind::Modified => {
            let s = short(source);
            let t = short(target);
            // Drop the shared leading words from the target side ("age INT -> BIGINT").
            let sw = s.split_whitespace();
            let tw: Vec<&str> = t.split_whitespace().collect();
            let k = sw.zip(&tw).take_while(|(a, b)| a == *b).count();
            let t = if k < tw.len() { tw[k..].join(" ") } else { t };
            format!("~ {} -> {}", s, t)
        }
        LineKind::Unchanged => format!("  {}", short(source)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_produces_unchanged() {
        let rows = diff_lines("a\nb\n", "a\nb\n");
        assert!(rows.iter().all(|r| r.kind == LineKind::Unchanged));
        assert_eq!(rows.len(), 2);
    }

    #[test]
    fn single_line_change_is_modified() {
        let rows = diff_lines("age INT\n", "age BIGINT\n");
        assert_eq!(rows, vec![DiffLine{kind: LineKind::Modified,
            source: "age INT".into(), target: "age BIGINT".into()}]);
    }

    #[test]
    fn addition_and_removal_align() {
        let rows = diff_lines("a\nb\nc\n", "a\nc\nd\n");
        assert!(rows.iter().any(|r| r.kind == LineKind::Removed && r.source == "b"));
        assert!(rows.iter().any(|r| r.kind == LineKind::Added && r.target == "d"));
    }

    #[test]
    fn empty_source_all_added() {
        let rows = diff_lines("", "x\ny\n");
        assert_eq!(rows.iter().filter(|r| r.kind == LineKind::Added).count(), 2);
    }

    #[test]
    fn summary_strips_prefix() {
        assert_eq!(summary_entry(LineKind::Removed, "COLUMN name NVARCHAR(50)", ""),
                   "- name NVARCHAR(50)");
        assert_eq!(summary_entry(LineKind::Modified, "COLUMN age INT", "COLUMN age BIGINT"),
                   "~ age INT -> BIGINT");
    }
}

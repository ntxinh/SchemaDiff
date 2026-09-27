//! Best-effort export of a `CompareResult`: unified text, JSON, CSV and a
//! review-only SQL script. SQL is never applied — anything not expressible
//! as a plain statement becomes a `-- TODO: manual migration:` comment.
//!
//! `compare()` already orients `src`/`tgt` payloads to the migration
//! direction (target = desired state), so SQL generation is
//! direction-agnostic; `dir` is accepted for future per-direction tweaks.

use crate::compare::{CompareResult, ObjectDiff, SchemaPayload};
use crate::diff::{DiffLine, LineKind};
use crate::model::*;
use anyhow::Result;
use serde::Serialize;

/// git-style unified text for one object.
pub fn unified_text(o: &ObjectDiff) -> String {
    let mut s = format!("@@ {}.{} ({}) @@\n", o.schema, o.name, kind_name(o.kind));
    for l in &o.lines {
        match l.kind {
            LineKind::Added => s.push_str(&format!("+ {}\n", l.target)),
            LineKind::Removed => s.push_str(&format!("- {}\n", l.source)),
            LineKind::Modified => s.push_str(&format!("~ {} | {}\n", l.source, l.target)),
            LineKind::Unchanged => s.push_str(&format!("  {}\n", l.source)),
        }
    }
    s
}

/// Unified text for every object in every group.
pub fn unified_text_all(r: &CompareResult) -> String {
    r.groups
        .iter()
        .flat_map(|g| g.objects.iter())
        .map(unified_text)
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn to_json(r: &CompareResult) -> Result<String> {
    Ok(serde_json::to_string_pretty(r)?)
}

#[derive(Serialize)]
struct Row<'a> {
    kind: &'a str,
    schema: &'a str,
    object: &'a str,
    presence: &'a str,
    line_kind: &'a str,
    source: &'a str,
    target: &'a str,
}

/// One CSV row per diff line. Header: `kind,schema,object,presence,line_kind,source,target`.
pub fn to_csv(r: &CompareResult) -> Result<String> {
    let mut w = csv::Writer::from_writer(Vec::new());
    for g in &r.groups {
        for o in &g.objects {
            for l in &o.lines {
                w.serialize(Row {
                    kind: kind_name(o.kind),
                    schema: &o.schema,
                    object: &o.name,
                    presence: presence_name(o.presence),
                    line_kind: line_kind_name(l.kind),
                    source: &l.source,
                    target: &l.target,
                })?;
            }
        }
    }
    Ok(String::from_utf8(w.into_inner()?)?)
}

/// SQL statements to migrate one object toward the target state.
pub fn sql_for_object(o: &ObjectDiff, dir: Direction) -> String {
    let _ = dir; // payloads are already oriented by compare()
    let q = qname(&o.schema, &o.name);
    match (&o.src, &o.tgt) {
        (None, Some(tgt)) => create_sql(o, tgt, &q),
        (Some(_), None) => drop_sql(o, &q),
        (Some(SchemaPayload::Table(s)), Some(SchemaPayload::Table(t))) => {
            alter_table_sql(o, s, t, &q)
        }
        (Some(_), Some(SchemaPayload::Module(m))) => {
            format!("CREATE OR ALTER {}\n", after_create(&m.definition))
        }
        (Some(_), Some(SchemaPayload::Udt(u))) => udt_todo(u),
        _ => format!("-- TODO: manual migration: {}.{}\n", o.schema, o.name),
    }
}

/// SQL script for every changed object, separated by blank lines.
pub fn sql_for_all(r: &CompareResult) -> String {
    r.groups
        .iter()
        .flat_map(|g| g.objects.iter())
        .filter(|o| o.change_count > 0)
        .map(|o| sql_for_object(o, r.direction))
        .collect::<Vec<_>>()
        .join("\n")
}

fn create_sql(_o: &ObjectDiff, tgt: &SchemaPayload, q: &str) -> String {
    match tgt {
        SchemaPayload::Table(t) => {
            let cols = t
                .columns
                .iter()
                .map(|c| format!("    {}", col_def(c)))
                .collect::<Vec<_>>()
                .join(",\n");
            format!("CREATE TABLE {q} (\n{cols}\n);\n-- review: add constraints/indexes\n")
        }
        SchemaPayload::Module(m) => format!("CREATE OR ALTER {}\n", after_create(&m.definition)),
        SchemaPayload::Udt(u) => udt_todo(u),
    }
}

fn drop_sql(o: &ObjectDiff, q: &str) -> String {
    let kw = match o.kind {
        ObjKind::Table => "TABLE",
        ObjKind::View => "VIEW",
        ObjKind::Procedure => "PROCEDURE",
        ObjKind::Function => "FUNCTION",
        ObjKind::Trigger => "TRIGGER",
        ObjKind::Udt => {
            return format!("-- TODO: UDT {q} exists only in source; drop manually\n");
        }
    };
    format!("DROP {kw} {q};\n")
}

fn alter_table_sql(o: &ObjectDiff, s: &TableSchema, t: &TableSchema, q: &str) -> String {
    let mut out = String::new();
    for c in &t.columns {
        match s.columns.iter().find(|sc| sc.name == c.name) {
            None => out.push_str(&format!("ALTER TABLE {q} ADD {};\n", col_def(c))),
            Some(sc) if sc.data_type != c.data_type || sc.nullable != c.nullable => {
                out.push_str(&format!(
                    "ALTER TABLE {q} ALTER COLUMN [{}] {} {};\n",
                    c.name,
                    c.data_type,
                    if c.nullable { "NULL" } else { "NOT NULL" }
                ));
                if sc.identity != c.identity || sc.default != c.default {
                    out.push_str(&format!(
                        "-- TODO: manual migration: identity/default changed on [{}]\n",
                        c.name
                    ));
                }
            }
            Some(sc) if sc.identity != c.identity || sc.default != c.default => {
                out.push_str(&format!(
                    "-- TODO: manual migration: identity/default changed on [{}]\n",
                    c.name
                ));
            }
            _ => {}
        }
    }
    for c in &s.columns {
        if !t.columns.iter().any(|tc| tc.name == c.name) {
            out.push_str(&format!("ALTER TABLE {q} DROP COLUMN [{}];\n", c.name));
        }
    }
    // Constraint/index diffs: pair summary lines with diff lines; any changed
    // line not starting with COLUMN is a constraint or index change.
    for (l, sm) in changed_lines(o) {
        let text = match l.kind {
            LineKind::Added | LineKind::Modified => &l.target,
            _ => &l.source,
        };
        let is_column = match l.kind {
            LineKind::Modified => l.source.starts_with("COLUMN ") && l.target.starts_with("COLUMN "),
            _ => text.starts_with("COLUMN "),
        };
        if !is_column {
            out.push_str(&format!("-- TODO: manual migration: {sm}\n"));
        }
    }
    if out.is_empty() {
        out.push_str(&format!("-- no actionable diff for {q}\n"));
    }
    out
}

fn changed_lines(o: &ObjectDiff) -> impl Iterator<Item = (&DiffLine, &String)> {
    // summary has one entry per changed line; filter before zipping
    o.lines
        .iter()
        .filter(|l| l.kind != LineKind::Unchanged)
        .zip(o.summary.iter())
}

fn udt_todo(u: &UdtDef) -> String {
    format!(
        "-- TODO: UDT [{}].[{}] cannot be altered; drop/recreate required.\n",
        u.schema, u.name
    )
}

/// `[name] TYPE [NOT] NULL [IDENTITY] [DEFAULT <d>]`
fn col_def(c: &Column) -> String {
    format!(
        "[{}] {} {}{}{}",
        c.name,
        c.data_type,
        if c.nullable { "NULL" } else { "NOT NULL" },
        if c.identity { " IDENTITY" } else { "" },
        c.default
            .as_deref()
            .map(|d| format!(" DEFAULT {d}"))
            .unwrap_or_default()
    )
}

/// Definition text with a leading `CREATE` keyword stripped, for prefixing
/// with `CREATE OR ALTER`.
fn after_create(def: &str) -> &str {
    let t = def.trim_start();
    if t.len() >= 6 && t[..6].eq_ignore_ascii_case("create") {
        t[6..].trim_start()
    } else {
        t
    }
}

fn qname(schema: &str, name: &str) -> String {
    format!("[{}].[{}]", schema.replace(']', "]]"), name.replace(']', "]]"))
}

fn kind_name(k: ObjKind) -> &'static str {
    match k {
        ObjKind::Table => "Table",
        ObjKind::View => "View",
        ObjKind::Procedure => "Procedure",
        ObjKind::Function => "Function",
        ObjKind::Trigger => "Trigger",
        ObjKind::Udt => "Udt",
    }
}

fn presence_name(p: Presence) -> &'static str {
    match p {
        Presence::Both => "Both",
        Presence::OnlySource => "OnlySource",
        Presence::OnlyTarget => "OnlyTarget",
    }
}

fn line_kind_name(k: LineKind) -> &'static str {
    match k {
        LineKind::Added => "Added",
        LineKind::Removed => "Removed",
        LineKind::Modified => "Modified",
        LineKind::Unchanged => "Unchanged",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compare::compare;

    fn col(name: &str, ty: &str, nullable: bool) -> Column {
        Column {
            name: name.into(),
            data_type: ty.into(),
            nullable,
            identity: false,
            default: None,
        }
    }

    fn table(cols: Vec<Column>) -> TableSchema {
        TableSchema {
            schema: "dbo".into(),
            name: "Users".into(),
            columns: cols,
            constraints: vec![],
            indexes: vec![],
        }
    }

    fn table_diff(s: TableSchema, t: TableSchema) -> ObjectDiff {
        let src = Schema {
            tables: vec![s],
            ..Schema::empty()
        };
        let tgt = Schema {
            tables: vec![t],
            ..Schema::empty()
        };
        compare(&src, &tgt, Direction::SourceToTarget)
            .groups
            .into_iter()
            .find(|g| g.kind == ObjKind::Table)
            .unwrap()
            .objects
            .remove(0)
    }

    #[test]
    fn unified_has_header_and_prefixes() {
        let s = table(vec![col("id", "INT", false), col("name", "NVARCHAR(50)", false)]);
        let t = table(vec![
            col("id", "INT", false),
            col("name", "NVARCHAR(100)", false),
        ]);
        let o = table_diff(s, t);
        let text = unified_text(&o);
        assert!(text.starts_with("@@ dbo.Users (Table) @@\n"), "{text}");
        for line in text.lines().skip(1) {
            assert!(
                line.starts_with("- ")
                    || line.starts_with("+ ")
                    || line.starts_with("~ ")
                    || line.starts_with("  "),
                "bad prefix: {line}"
            );
        }
        assert!(text.contains("~ COLUMN name NVARCHAR(50) NOT NULL | COLUMN name NVARCHAR(100) NOT NULL"));
    }

    #[test]
    fn sql_alter_add_column() {
        let s = table(vec![col("id", "INT", false)]);
        let t = table(vec![col("id", "INT", false), col("email", "NVARCHAR(100)", true)]);
        let o = table_diff(s, t);
        let sql = sql_for_object(&o, Direction::SourceToTarget);
        assert!(
            sql.contains("ALTER TABLE [dbo].[Users] ADD [email] NVARCHAR(100) NULL"),
            "{sql}"
        );
    }

    #[test]
    fn sql_create_or_alter_proc() {
        let def = "CREATE PROCEDURE dbo.p\nAS\n    SELECT 42";
        let src = Schema::empty();
        let tgt = Schema {
            modules: vec![ModuleObject {
                schema: "dbo".into(),
                name: "p".into(),
                kind: ObjKind::Procedure,
                definition: def.into(),
            }],
            ..Schema::empty()
        };
        let r = compare(&src, &tgt, Direction::SourceToTarget);
        let o = &r.groups
            .iter()
            .find(|g| g.kind == ObjKind::Procedure)
            .unwrap()
            .objects[0];
        let sql = sql_for_object(o, Direction::SourceToTarget);
        assert!(sql.starts_with("CREATE OR ALTER PROCEDURE dbo.p"), "{sql}");
        assert!(sql.contains("SELECT 42"), "{sql}");
    }

    #[test]
    fn json_csv_roundtrip() {
        let s = table(vec![col("id", "INT", false)]);
        let t = table(vec![col("id", "INT", false), col("email", "NVARCHAR(100)", true)]);
        let src = Schema {
            tables: vec![s],
            ..Schema::empty()
        };
        let tgt = Schema {
            tables: vec![t],
            ..Schema::empty()
        };
        let r = compare(&src, &tgt, Direction::SourceToTarget);

        let json = to_json(&r).unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["compared"], serde_json::json!(r.compared));

        let csv = to_csv(&r).unwrap();
        let mut lines = csv.lines();
        assert_eq!(
            lines.next().unwrap(),
            "kind,schema,object,presence,line_kind,source,target"
        );
        let n = r
            .groups
            .iter()
            .flat_map(|g| g.objects.iter())
            .map(|o| o.lines.len())
            .sum::<usize>();
        assert_eq!(lines.count(), n);
    }
}

# MSSQL Schema Compare Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a Slint desktop app that diffs two MSSQL database schemas (git-diff style) with copy/export actions.

**Architecture:** Slint UI thread + dedicated tokio thread running tiberius queries. Commands flow UI→backend over `tokio::mpsc::unbounded`; results flow back over `std::sync::mpsc` polled by a repeating `slint::Timer`. Each schema object renders to canonical text; `similar` produces line diffs; hunks pair removed/added lines positionally into "modified" rows. SQL export is best-effort (CREATE OR ALTER for modules, ALTER COLUMN for table column diffs, `-- TODO` comments otherwise).

**Tech Stack:** Rust 2021, Slint 1.18, tiberius 0.13 (rustls), tokio 1, similar 3.2, arboard 3, rfd 0.17, serde/serde_json/csv, anyhow.

**Spec:** `docs/superpowers/specs/2026-09-27-mssql-schema-compare-design.md`

## Global Constraints

- Diff direction is always Source→Target (Source is the base; `Added` = present in Target). The UI **Swap** button physically exchanges Source/Target connection strings — that *is* the direction toggle. No separate toggle.
- Connection strings: `jdbc:` prefix → `Config::from_jdbc_string`, else `Config::from_ado_string`. Document `TrustServerCertificate=true` for dev containers. SQL auth only; Windows/NTLM/Azure AD out of scope.
- Rust edition `2021`, `mise.toml` pins `rust = "1.98"`.
- Compare must never block the UI thread; clipboard/file-dialog calls stay on the UI thread.
- Generated SQL scripts are review artifacts, never applied to a database.
- Commit after every task. Conventional commits (`feat:`, `test:`, `chore:`).
- Name collisions: Slint generates `DiffRow`/`TreeRow` structs — domain types use `DiffLine`/`TreeEntry`/`LineKind`.

## File Structure

| File | Responsibility |
|---|---|
| `src/main.rs` | channel creation, callback wiring, Timer polling, clipboard/dialog calls |
| `src/model.rs` | `Schema`, `TableSchema`, `Column`, `Constraint`, `IndexDef`, `ModuleObject`, `UdtDef`, `ObjKind`, `Presence`, `Direction` |
| `src/render.rs` | canonical text per object; `type_name` formatter |
| `src/diff.rs` | `diff_lines` (similar → `Vec<DiffLine>`), summary strings |
| `src/schema.rs` | tiberius `connect` + `fetch_schema` queries |
| `src/compare.rs` | `compare` (`Schema`×`Schema` → `CompareResult` with `ObjectDiff`/`ObjectGroup`) |
| `src/export.rs` | unified text, JSON, CSV, SQL generation |
| `src/tree.rs` | `flatten` (`CompareResult` + expanded set → `Vec<TreeEntry>`) |
| `src/backend.rs` | `Cmd`/`Reply` enums, `start()` command loop on tokio thread |
| `ui/app.slint` | full UI: `State` global, `AppWindow` |
| `build.rs` | `slint_build::compile("ui/app.slint")` — added in Task 8 |
| `dev/seed_src.sql`, `dev/seed_tgt.sql` | fixture databases for podman SQL Server |
| `tests/itest.rs` | env-gated (`SCHEMADIFF_ITEST=1`) live-DB integration test |
| `Makefile` | `run check test db-up db-seed db-down itest` |
| `compose.yml` | SQL Server 2022 service (podman-compatible) |

---

### Task 1: Project scaffold

**Files:**
- Create: `Cargo.toml`, `mise.toml`, `.gitignore`, `.editorconfig`, `src/main.rs`, `src/lib.rs`

**Interfaces:**
- Produces: `main()` that runs a placeholder Slint window via `slint::slint!` macro (no `ui/` yet, so no `include_modules!`). `lib.rs` empty (modules land in later tasks).

- [ ] **Step 1: Write all files**

`Cargo.toml`:
```toml
[package]
name = "schemadiff"
version = "0.1.0"
edition = "2021"

[dependencies]
slint = "1.18"
tiberius = { version = "0.13", default-features = false, features = ["rustls", "tokio-rustls", "tds73"] }
tokio = { version = "1", features = ["rt-multi-thread", "macros", "sync"] }
tokio-util = { version = "0.7", features = ["compat"] }
similar = "3.2"
arboard = "3.6"
rfd = "0.17"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
csv = "1"
anyhow = "1"

[build-dependencies]
slint-build = "1.18"
```

`mise.toml`:
```toml
[tools]
rust = "1.98"
```

`.gitignore`:
```
/target
.env
```

`.editorconfig`:
```ini
root = true
[*]
indent_style = space
indent_size = 4
end_of_line = lf
charset = utf-8
trim_trailing_whitespace = true
insert_final_newline = true
[*.{slint,toml,yml}]
indent_size = 4
```

`src/main.rs`:
```rust
slint::slint! {
    export component AppWindow inherits Window {
        title: "MSSQL Schema Compare";
        preferred-width: 1100px; preferred-height: 720px;
        Text { text: "SchemaDiff"; }
    }
}

fn main() -> Result<(), slint::PlatformError> {
    AppWindow::new()?.run()
}
```

`src/lib.rs`: empty file (placeholder for modules).

- [ ] **Step 2: Verify build**

Run: `cargo check`
Expected: compiles; first build downloads crates (tiberius pulls tokio-rustls, not openssl).

- [ ] **Step 3: Smoke run**

Run: `timeout 8 cargo run` (or `cargo run` and close the window)
Expected: a window titled "MSSQL Schema Compare" opens; no panic in stderr.

- [ ] **Step 4: Commit**

```bash
git add -A && git commit -m "chore: scaffold cargo project with slint window"
```

---

### Task 2: Diff engine (`src/diff.rs`)

**Files:**
- Create: `src/diff.rs`
- Modify: `src/lib.rs` (add `pub mod diff;`)

**Interfaces:**
- Produces:
  ```rust
  #[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize)]
  pub enum LineKind { Added, Removed, Modified, Unchanged }

  #[derive(Clone, Debug, serde::Serialize)]
  pub struct DiffLine { pub kind: LineKind, pub source: String, pub target: String }

  /// Line diff of two canonical texts; delete/insert runs are paired
  /// positionally into Modified rows.
  pub fn diff_lines(source: &str, target: &str) -> Vec<DiffLine>

  /// "- name NVARCHAR(50)" style entry; strips COLUMN/INDEX/PK/FK/CHECK prefixes, truncates 64 chars.
  pub fn summary_entry(kind: LineKind, source: &str, target: &str) -> String
  ```
- Consumed by: `compare.rs` (Task 5), `main.rs` mapping to Slint `DiffRow` (Task 9).

- [ ] **Step 1: Write failing tests**

`src/diff.rs` test module (or `tests/diff.rs` — put unit tests in-file):
```rust
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
```

- [ ] **Step 2: Run tests, confirm failure**

Run: `cargo test`
Expected: FAIL — `diff_lines`/`DiffLine` undefined.

- [ ] **Step 3: Implement**

```rust
use serde::Serialize;
use similar::{ChangeTag, TextDiff};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
pub enum LineKind { Added, Removed, Modified, Unchanged }

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct DiffLine { pub kind: LineKind, pub source: String, pub target: String }

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
    let n = dels.len().min(ins.len());
    for i in 0..n {
        out.push(DiffLine { kind: LineKind::Modified,
            source: dels[i].clone(), target: ins[i].clone() });
    }
    for s in dels.drain(..) { out.push(DiffLine{kind: LineKind::Removed, source: s, target: String::new()}); }
    for t in ins.drain(..) { out.push(DiffLine{kind: LineKind::Added, source: String::new(), target: t}); }
}

pub fn summary_entry(kind: LineKind, source: &str, target: &str) -> String {
    fn short(s: &str) -> String {
        let s = s.trim();
        let s = ["COLUMN ", "INDEX ", "CONSTRAINT ", "PK ", "FK ", "CHECK ", "DEFAULT "]
            .iter().find_map(|p| s.strip_prefix(p)).unwrap_or(s);
        if s.chars().count() > 64 { format!("{}…", s.chars().take(63).collect::<String>()) } else { s.to_string() }
    }
    match kind {
        LineKind::Removed => format!("- {}", short(source)),
        LineKind::Added => format!("+ {}", short(target)),
        LineKind::Modified => format!("~ {} -> {}", short(source), short(target)),
        LineKind::Unchanged => format!("  {}", short(source)),
    }
}
```

Note: `assert_eq!` on `Vec<DiffLine>` needs `PartialEq` — included above.

- [ ] **Step 4: Run tests**

Run: `cargo test`
Expected: PASS, 5 tests.

- [ ] **Step 5: Commit**

```bash
git add src/diff.rs src/lib.rs && git commit -m "feat: line diff engine with modified-line pairing"
```

---

### Task 3: Schema model + canonical text (`src/model.rs`, `src/render.rs`)

**Files:**
- Create: `src/model.rs`, `src/render.rs`
- Modify: `src/lib.rs` (add `pub mod model; pub mod render;`)

**Interfaces:**
- Produces (all `Clone, Debug, serde::Serialize`):
  ```rust
  // model.rs
  pub enum ObjKind { Table, View, Procedure, Function, Trigger, Udt }
  impl ObjKind { pub const ALL: [ObjKind; 6]; pub fn label(&self) -> &'static str } // "Tables","Views","Stored Procedures","Functions","Triggers","User-Defined Types"
  pub enum Presence { Both, OnlySource, OnlyTarget }
  pub enum Direction { SourceToTarget, TargetToSource }
  pub struct Column { pub name: String, pub data_type: String, pub nullable: bool, pub identity: bool, pub default: Option<String> }
  pub enum ConstraintKind { Pk, Uq, Fk, Check }
  pub struct Constraint { pub kind: ConstraintKind, pub name: String, pub text: String } // text = canonical body, e.g. "(id)" or "(user_id) -> dbo.Users (id)"
  pub struct IndexDef { pub name: String, pub unique: bool, pub columns: Vec<String>, pub included: Vec<String> }
  pub struct TableSchema { pub schema: String, pub name: String, pub columns: Vec<Column>, pub constraints: Vec<Constraint>, pub indexes: Vec<IndexDef> }
  pub struct ModuleObject { pub schema: String, pub name: String, pub kind: ObjKind, pub definition: String }
  pub struct UdtDef { pub schema: String, pub name: String, pub base_type: String, pub nullable: bool }
  pub struct Schema { pub tables: Vec<TableSchema>, pub modules: Vec<ModuleObject>, pub udts: Vec<UdtDef> }
  impl Schema { pub fn empty() -> Self }

  // render.rs
  pub fn table_text(t: &TableSchema) -> String     // deterministic canonical text
  pub fn module_text(m: &ModuleObject) -> String   // definition, \r\n→\n, trimmed
  pub fn udt_text(u: &UdtDef) -> String            // "TYPE name FROM BASE [NOT] NULL"
  pub fn type_name(base: &str, max_length: i16, precision: u8, scale: u8) -> String
  ```
- Consumed by: `schema.rs` builds them (Task 4); `compare.rs` calls `*_text` (Task 5); `export.rs` reads struct fields for SQL gen (Task 6).

`table_text` format (exact):
```
COLUMN <name> <data_type> NULL|NOT NULL [IDENTITY] [DEFAULT <expr>]
CONSTRAINT PK <name> (<cols>)          -- sorted by name
CONSTRAINT UQ <name> (<cols>)
CONSTRAINT FK <name> (<cols>) -> <schema>.<table> (<cols>)
CONSTRAINT CHECK <name> <expr>
INDEX <name> [UNIQUE] (<cols>) [INCLUDE (<cols>)]   -- sorted by name
```
Columns in `column_id` order (order stored in the vec). Constraints grouped kind-then-name sorted. Indexes sorted by name.

`type_name` rules: uppercase base; `nvarchar/nchar/nchar`/`n…` → `max_length/2` (or `MAX` when -1); `varchar/char/varbinary/binary` → `max_length` (or `MAX`); `decimal/numeric` → `(precision,scale)`; `datetime2/datetimeoffset/time` → `(scale)`; else bare name.

- [ ] **Step 1: Write failing tests**

```rust
#[test]
fn type_names() {
    assert_eq!(type_name("nvarchar", 100, 0, 0), "NVARCHAR(50)");
    assert_eq!(type_name("nvarchar", -1, 0, 0), "NVARCHAR(MAX)");
    assert_eq!(type_name("decimal", 0, 18, 2), "DECIMAL(18,2)");
    assert_eq!(type_name("int", 4, 10, 0), "INT");
    assert_eq!(type_name("datetime2", 8, 0, 7), "DATETIME2(7)");
}

#[test]
fn table_text_is_deterministic() {
    let t = TableSchema {
        schema: "dbo".into(), name: "Users".into(),
        columns: vec![
            Column{name:"id".into(), data_type:"INT".into(), nullable:false, identity:true, default:None},
            Column{name:"name".into(), data_type:"NVARCHAR(50)".into(), nullable:false, identity:false, default:None},
        ],
        constraints: vec![Constraint{kind:ConstraintKind::Pk, name:"PK_Users".into(), text:"(id)".into()}],
        indexes: vec![IndexDef{name:"IX_name".into(), unique:false, columns:vec!["name".into()], included:vec![]}],
    };
    let text = table_text(&t);
    assert!(text.contains("COLUMN id INT NOT NULL IDENTITY"));
    assert!(text.contains("COLUMN name NVARCHAR(50) NOT NULL"));
    assert!(text.contains("CONSTRAINT PK PK_Users (id)"));
    assert!(text.contains("INDEX IX_name (name)"));
}

#[test]
fn module_text_normalizes_crlf() {
    let m = ModuleObject{schema:"dbo".into(),name:"v".into(),kind:ObjKind::View,
        definition:"CREATE VIEW v\r\nAS SELECT 1\r\n".into()};
    assert_eq!(module_text(&m), "CREATE VIEW v\nAS SELECT 1");
}
```

- [ ] **Step 2: Run, confirm failure**

Run: `cargo test` → FAIL (unresolved `model`/`render`).

- [ ] **Step 3: Implement `model.rs` and `render.rs`**

`model.rs`: plain structs/enums per Interfaces + `ObjKind::ALL` + `label()` + `Schema::empty()`. All derive `Clone, Debug, Serialize` (`PartialEq` too — tests compare).

`render.rs`:
```rust
use crate::model::*;

pub fn type_name(base: &str, max_length: i16, precision: u8, scale: u8) -> String {
    let b = base.to_uppercase();
    match base {
        "nvarchar" | "nchar" => fmt_len(b, if max_length < 0 { -1 } else { max_length / 2 }),
        "varchar" | "char" | "varbinary" | "binary" => fmt_len(b, max_length),
        "decimal" | "numeric" => format!("{b}({precision},{scale})"),
        "datetime2" | "datetimeoffset" | "time" => format!("{b}({scale})"),
        _ => b,
    }
}
fn fmt_len(b: String, n: i16) -> String {
    if n < 0 { format!("{b}(MAX)") } else { format!("{b}({n})") }
}

pub fn table_text(t: &TableSchema) -> String {
    let mut s = String::new();
    for c in &t.columns {
        s.push_str(&format!("COLUMN {} {} {} NULL{}{}\n",
            c.name, c.data_type,
            if c.nullable { "" } else { "NOT " },   // careful: prints "NOT NULL"/"NULL"
            if c.identity { " IDENTITY" } else { "" },
            c.default.as_deref().map(|d| format!(" DEFAULT {d}")).unwrap_or_default()));
    }
    let mut cons = t.constraints.clone();
    cons.sort_by(|a, b| (con_kind_ord(&a.kind), &a.name).cmp(&(con_kind_ord(&b.kind), &b.name)));
    for c in &cons {
        let k = match c.kind { ConstraintKind::Pk=>"PK", ConstraintKind::Uq=>"UQ",
            ConstraintKind::Fk=>"FK", ConstraintKind::Check=>"CHECK" };
        s.push_str(&format!("CONSTRAINT {k} {} {}\n", c.name, c.text));
    }
    let mut idx = t.indexes.clone();
    idx.sort_by(|a,b| a.name.cmp(&b.name));
    for i in &idx {
        s.push_str(&format!("INDEX {}{} ({}){}\n", i.name,
            if i.unique { " UNIQUE" } else { "" }, i.columns.join(", "),
            if i.included.is_empty() { String::new() } else { format!(" INCLUDE ({})", i.included.join(", ")) }));
    }
    s.trim_end().to_string()
}
fn con_kind_ord(k:&ConstraintKind)->u8{match k{ConstraintKind::Pk=>0,ConstraintKind::Uq=>1,ConstraintKind::Fk=>2,ConstraintKind::Check=>3}}

pub fn module_text(m: &ModuleObject) -> String {
    m.definition.replace("\r\n", "\n").trim().to_string()
}
pub fn udt_text(u: &UdtDef) -> String {
    format!("TYPE {} FROM {} {}NULL", u.name, u.base_type, if u.nullable {""} else {"NOT "})
}
```
(Fix the `NULL`/`NOT NULL` formatting so it emits exactly `NOT NULL` or `NULL` — the test pins this.)

- [ ] **Step 4: Run tests** — `cargo test` → all PASS.

- [ ] **Step 5: Commit**

```bash
git add src/model.rs src/render.rs src/lib.rs && git commit -m "feat: schema model and canonical text rendering"
```

---

### Task 4: Schema extraction (`src/schema.rs`)

**Files:**
- Create: `src/schema.rs`
- Modify: `src/lib.rs` (add `pub mod schema;`)

**Interfaces:**
- Produces:
  ```rust
  pub type DbClient = tiberius::Client<tokio_util::compat::Compat<tokio::net::TcpStream>>;
  /// Detect "jdbc:" prefix → JDBC parse, else ADO; fall back to the other parser on error.
  pub fn parse_config(cs: &str) -> anyhow::Result<tiberius::Config>
  pub async fn connect(cs: &str) -> anyhow::Result<DbClient>
  pub async fn fetch_schema(client: &mut DbClient) -> anyhow::Result<crate::model::Schema>
  ```
- Consumed by: `backend.rs` (Task 9), `tests/itest.rs` (Task 10).

- [ ] **Step 1: Implement (no DB available for unit test — compile-check + deferred itest)**

Queries (run sequentially via `client.simple_query(q).await?.into_first_result().await?`; each returns `Vec<Row>` in index 0 — actually `into_first_result` returns `Vec<Row>` for the first result set):

```sql
-- Q1 columns
SELECT s.name AS sch, o.name AS tbl, c.column_id, c.name AS col,
       ty.name AS typ, c.max_length, c.precision, c.scale,
       c.is_nullable, c.is_identity, dc.definition AS dflt
FROM sys.objects o
JOIN sys.schemas s ON s.schema_id = o.schema_id
JOIN sys.columns c ON c.object_id = o.object_id
JOIN sys.types ty ON ty.user_type_id = c.user_type_id
LEFT JOIN sys.default_constraints dc ON dc.object_id = c.default_object_id
WHERE o.type = 'U' AND o.is_ms_shipped = 0
ORDER BY s.name, o.name, c.column_id;

-- Q2 modules
SELECT s.name AS sch, o.name, o.type, m.definition
FROM sys.objects o
JOIN sys.schemas s ON s.schema_id = o.schema_id
JOIN sys.sql_modules m ON m.object_id = o.object_id
WHERE o.type IN ('V','P','FN','IF','TF','TR') AND o.is_ms_shipped = 0
ORDER BY s.name, o.name;

-- Q3 indexes incl. PK/UQ
SELECT s.name AS sch, o.name AS tbl, i.name, i.is_unique, i.is_primary_key,
       i.is_unique_constraint, c.name AS col, ic.key_ordinal, ic.is_included_column,
       ic.is_descending_key
FROM sys.indexes i
JOIN sys.objects o ON o.object_id = i.object_id
JOIN sys.schemas s ON s.schema_id = o.schema_id
JOIN sys.index_columns ic ON ic.object_id = i.object_id AND ic.index_id = i.index_id
JOIN sys.columns c ON c.object_id = ic.object_id AND c.column_id = ic.column_id
WHERE o.type = 'U' AND o.is_ms_shipped = 0 AND i.name IS NOT NULL
ORDER BY s.name, o.name, i.name, ic.key_ordinal, ic.index_column_id;

-- Q4 check constraints
SELECT s.name AS sch, o.name AS tbl, cc.name, cc.definition
FROM sys.check_constraints cc
JOIN sys.objects o ON o.object_id = cc.parent_object_id
JOIN sys.schemas s ON s.schema_id = o.schema_id
WHERE o.is_ms_shipped = 0;

-- Q5 foreign keys
SELECT s.name AS sch, o.name AS tbl, fk.name, rs.name AS rsch, ro.name AS rtbl,
       pc.name AS col, rc.name AS rcol, fkc.constraint_column_id
FROM sys.foreign_keys fk
JOIN sys.foreign_key_columns fkc ON fkc.constraint_object_id = fk.object_id
JOIN sys.objects o ON o.object_id = fk.parent_object_id
JOIN sys.schemas s ON s.schema_id = o.schema_id
JOIN sys.objects ro ON ro.object_id = fk.referenced_object_id
JOIN sys.schemas rs ON rs.schema_id = ro.schema_id
JOIN sys.columns pc ON pc.object_id = fk.parent_object_id AND pc.column_id = fkc.parent_column_id
JOIN sys.columns rc ON rc.object_id = fk.referenced_object_id AND rc.column_id = fkc.referenced_column_id
ORDER BY s.name, o.name, fk.name, fkc.constraint_column_id;

-- Q6 UDTs
SELECT s.name AS sch, t.name, b.name AS base_typ, t.max_length, t.precision, t.scale, t.is_nullable
FROM sys.types t
JOIN sys.schemas s ON s.schema_id = t.schema_id
JOIN sys.types b ON b.user_type_id = t.system_type_id
WHERE t.is_user_defined = 1;
```

Assembly:
- `fetch_schema` runs Q1–Q6 in order, collecting rows.
- Q1 → build `BTreeMap<(String,String), TableSchema>`; `data_type` = `type_name(typ, max_length, precision, scale)`; `default` = `dflt` (Option<String>).
- Q2 → `ModuleObject { kind: map_type(o.type) }` where `V→View, P→Procedure, FN|IF|TF→Function, TR→Trigger`.
- Q3 → group rows by (sch,tbl,index name): `is_primary_key` or `is_unique_constraint` → `Constraint` (`text = "(col, col)"` ordered by `key_ordinal`); else `IndexDef` (key cols `key_ordinal>0` ordered, `is_included_column` → `included`). Descending key cols rendered `col DESC`.
- Q4 → `Constraint{kind:Check, text: definition}`.
- Q5 → group by fk name: `text = "(cols) -> rsch.rtbl (rcols)"`.
- Q6 → `UdtDef` with `base_type = type_name(base_typ, max_length, precision, scale)`.
- Row access: `row.get::<&str, _>("col")` / `row.get::<i16,_>("max_length")` etc. `max_length` is `i16`, `precision`/`scale`/`column_id` are `u8`/`i32` — use `try_get` with context on error (`anyhow::Context`).

```rust
use anyhow::{Context, Result};
use futures_util::TryStreamExt; // only if needed; prefer into_first_result
use tokio::net::TcpStream;
use tokio_util::compat::TokioAsyncWriteCompatExt;
use tiberius::{Client, Config};
use crate::model::*;
use crate::render::type_name;
use std::collections::BTreeMap;

pub type DbClient = Client<tokio_util::compat::Compat<TcpStream>>;

pub fn parse_config(cs: &str) -> Result<Config> {
    let trimmed = cs.trim();
    if trimmed.to_ascii_lowercase().starts_with("jdbc:") {
        Config::from_jdbc_string(trimmed)
            .or_else(|_| Config::from_ado_string(trimmed)).context("bad connection string")
    } else {
        Config::from_ado_string(trimmed)
            .or_else(|_| Config::from_jdbc_string(trimmed)).context("bad connection string")
    }
}

pub async fn connect(cs: &str) -> Result<DbClient> {
    let config = parse_config(cs)?;
    let tcp = TcpStream::connect(config.get_addr()).await.context("tcp connect")?;
    tcp.set_nodelay(true)?;
    Client::connect(config, tcp.compat_write()).await.context("tds login")
}
```
Then `fetch_schema` with the six queries + assembly helpers (`fn map_obj_kind(&str) -> ObjKind`, `fn rows_to_tables(rows) -> BTreeMap<…>`, etc.).

- [ ] **Step 2: Compile**

Run: `cargo check`
Expected: clean. (Live verification happens in Task 10's itest.)

- [ ] **Step 3: Commit**

```bash
git add src/schema.rs src/lib.rs && git commit -m "feat: mssql schema extraction via tiberius"
```

---

### Task 5: Schema compare (`src/compare.rs`)

**Files:**
- Create: `src/compare.rs`
- Modify: `src/lib.rs` (`pub mod compare;`)

**Interfaces:**
- Consumes: `model::*`, `render::{table_text,module_text,udt_text}`, `diff::{diff_lines,summary_entry,DiffLine,LineKind}`.
- Produces:
  ```rust
  pub enum SchemaPayload { Table(TableSchema), Module(ModuleObject), Udt(UdtDef) }
  pub struct ObjectDiff {
      pub kind: ObjKind, pub schema: String, pub name: String,
      pub presence: Presence,
      pub lines: Vec<DiffLine>, pub summary: Vec<String>, pub change_count: usize,
      pub src: Option<SchemaPayload>, pub tgt: Option<SchemaPayload>,
  }
  pub struct ObjectGroup { pub kind: ObjKind, pub objects: Vec<ObjectDiff> } // sorted by name
  pub struct CompareResult { pub direction: Direction, pub groups: Vec<ObjectGroup>,
                             pub compared: usize, pub different: usize }
  pub fn compare(src: &Schema, tgt: &Schema, direction: Direction) -> CompareResult
  ```
- Consumed by: `export.rs` (Task 6), `tree.rs` (Task 7), `main.rs` (Task 9).

- [ ] **Step 1: Write failing tests** — canned `Schema`s exercising: identical table (count 0), modified table column, source-only trigger (`Presence::OnlySource`, all lines Removed), target-only table (`OnlyTarget`, all Added), direction `TargetToSource` swaps which side is "added". Assert `compared`, `different`, `groups[i].objects[j].change_count`, summary non-empty.

```rust
#[test]
fn added_object_all_added_lines() {
    let src = Schema::empty();
    let mut tgt = Schema::empty();
    tgt.tables.push(sample_table()); // helper building a 1-column table
    let r = compare(&src, &tgt, Direction::SourceToTarget);
    assert_eq!(r.compared, 1);
    assert_eq!(r.different, 1);
    let o = &r.groups.iter().find(|g| g.kind == ObjKind::Table).unwrap().objects[0];
    assert_eq!(o.presence, Presence::OnlyTarget);
    assert!(o.lines.iter().all(|l| l.kind == LineKind::Added));
}

#[test]
fn direction_swap_flips_presence() {
    // same two schemas, TargetToSource → what was OnlyTarget becomes OnlySource
}
// plus: modified table yields Some(Modified) lines; identical objects → change_count 0, still present
```

- [ ] **Step 2: Run, confirm failure.**

- [ ] **Step 3: Implement**

Core pairing loop per kind:
```rust
fn objects_of<'a>(s: &'a Schema, kind: ObjKind)
    -> Vec<(String /*key "sch.name"*/, String /*text*/, SchemaPayload)> {
    // Table → tables.iter().map(|t| (key, table_text(t), Payload::Table(t.clone())))
    // View/Proc/Fn/Trigger → modules.iter().filter(kind)
    // Udt → udts
}

pub fn compare(src: &Schema, tgt: &Schema, direction: Direction) -> CompareResult {
    // per ObjKind::ALL: BTreeMap<key, (text, payload)> for each side;
    // union of keys → ObjectDiff:
    //   Both → diff_lines(src_text, tgt_text)
    //   OnlySource → src_text.lines() each Removed; OnlyTarget → each Added
    //   presence fields swapped when direction == TargetToSource (report
    //   from the swapped viewpoint: simply swap src/tgt args into this fn
    //   and keep direction recorded for labels/export)
    // summary = lines.iter().filter(!Unchanged).map(summary_entry)
    // change_count = non-Unchanged lines
}
```
Direction: implement `compare` by swapping `src`/`tgt` when `TargetToSource`, storing `direction` on the result for labeling/export. `compared` = total objects; `different` = objects with `change_count > 0`.

- [ ] **Step 4: Run tests** — `cargo test` PASS.

- [ ] **Step 5: Commit**

```bash
git add src/compare.rs src/lib.rs && git commit -m "feat: schema comparison producing grouped object diffs"
```

---

### Task 6: Export (`src/export.rs`)

**Files:**
- Create: `src/export.rs`
- Modify: `src/lib.rs` (`pub mod export;`)

**Interfaces:**
- Consumes: `compare::{CompareResult,ObjectDiff,SchemaPayload}`, `model::*`.
- Produces:
  ```rust
  pub fn unified_text(o: &ObjectDiff) -> String            // git-style for one object
  pub fn unified_text_all(r: &CompareResult) -> String
  pub fn to_json(r: &CompareResult) -> anyhow::Result<String>
  pub fn to_csv(r: &CompareResult) -> anyhow::Result<String>
  pub fn sql_for_object(o: &ObjectDiff, dir: Direction) -> String
  pub fn sql_for_all(r: &CompareResult) -> String
  ```
- Consumed by: `main.rs` copy/export callbacks (Task 9).

`unified_text` format:
```
@@ dbo.Users (Table) @@
  COLUMN id INT NOT NULL IDENTITY
- COLUMN name NVARCHAR(50) NOT NULL
+ COLUMN email NVARCHAR(100) NULL
~ COLUMN age INT NULL | COLUMN age BIGINT NULL      -- modified: "src | tgt"
```
CSV header: `kind,schema,object,presence,line_kind,source,target`.

SQL gen rules:
- `SchemaPayload::Module` in tgt → `CREATE OR ALTER` + `tgt.definition` (target text verbatim; works for V/P/FN/TR). Source-only module → `DROP <type> sch.name;`.
- `SchemaPayload::Table`: target-only → `-- TODO: CREATE TABLE for sch.name (see target)` + generated `CREATE TABLE sch.name (col TYPE [NOT] NULL, ...)` from `tgt` columns (columns only, not constraints — note `-- review: add constraints/indexes`). Column diffs → `ALTER TABLE sch.name ADD/DROP COLUMN col TYPE [NULL]`; `ALTER COLUMN col TYPE`. Constraint/index diffs → `-- TODO: manual migration: <summary line>`. Source-only table → `DROP TABLE sch.name;`.
- `SchemaPayload::Udt` → `-- TODO: UDT sch.name cannot be altered; drop/recreate required`.

- [ ] **Step 1: Write failing tests**

```rust
#[test]
fn unified_has_header_and_prefixes() { /* ObjectDiff with mixed lines → "@@ dbo.T @@", lines start - + ~ or space */ }

#[test]
fn sql_alter_add_column() {
    // src table missing 'email', tgt has it → contains "ALTER TABLE [dbo].[Users] ADD [email] NVARCHAR(100) NULL"
}

#[test]
fn sql_create_or_alter_proc() {
    // modified ModuleObject → output starts "CREATE OR ALTER" and contains target body
}

#[test]
fn json_csv_roundtrip() {
    // to_json parses back via serde_json::Value; to_csv has header + N rows
}
```

- [ ] **Step 2: Run, confirm failure.**

- [ ] **Step 3: Implement** — straightforward string building; serde `Serialize` already on model/diff types; `to_csv` writes via `csv::Writer::from_writer(vec![])` with `serialize` on a flat `#[derive(Serialize)] struct Row<'a>` borrowing fields.

- [ ] **Step 4: Run tests** — PASS.

- [ ] **Step 5: Commit**

```bash
git add src/export.rs src/lib.rs && git commit -m "feat: export diff as unified text, json, csv, best-effort sql"
```

---

### Task 7: Tree flattening (`src/tree.rs`)

**Files:**
- Create: `src/tree.rs`
- Modify: `src/lib.rs` (`pub mod tree;`)

**Interfaces:**
- Consumes: `compare::CompareResult`.
- Produces:
  ```rust
  #[derive(Clone, Copy, PartialEq, Eq, Debug)]
  pub enum TreeRowKind { Group, Object, Detail }
  #[derive(Clone, Debug)]
  pub struct TreeEntry {
      pub row_kind: TreeRowKind,
      pub group: usize,                 // index into result.groups
      pub object: Option<usize>,        // index into groups[group].objects
      pub depth: u8,
      pub expanded: bool,
      pub selected: bool,
      pub label: String,
      pub count: i32,                   // -1 when N/A
  }
  /// expanded keys: (group_idx, None) = group open; (group_idx, Some(obj_idx)) = object open
  pub fn flatten(result: &CompareResult,
                 expanded: &std::collections::BTreeSet<(usize, Option<usize>)>,
                 selected: Option<(usize, usize)>) -> Vec<TreeEntry>
  ```
- Consumed by: `main.rs` (Task 9) → maps to Slint `TreeRow`.

Labels: Group → `ObjKind::label()`; Object → `sch.name` (indent via `depth`); Detail → summary strings.

- [ ] **Step 1: Write failing tests**

```rust
#[test]
fn collapsed_shows_only_groups() {
    let r = sample_result(); // 2 groups
    let rows = flatten(&r, &BTreeSet::new(), None);
    assert!(rows.iter().all(|e| e.row_kind == TreeRowKind::Group));
}

#[test]
fn expanded_group_lists_objects_then_details() {
    let mut ex = BTreeSet::new();
    ex.insert((0, None)); ex.insert((0, Some(0)));
    let rows = flatten(&r, &ex, Some((0,0)));
    // sequence: Group, Object(selected), Detail*, Group
    assert_eq!(rows[1].row_kind, TreeRowKind::Object);
    assert!(rows[1].selected);
    assert_eq!(rows[2].row_kind, TreeRowKind::Detail);
}
```

- [ ] **Step 2: Run, confirm failure.**

- [ ] **Step 3: Implement** — nested loop: group row; if expanded, object rows (`count = change_count`); if object expanded, detail rows from `object.summary`.

- [ ] **Step 4: Run tests** — PASS.

- [ ] **Step 5: Commit**

```bash
git add src/tree.rs src/lib.rs && git commit -m "feat: flatten compare result to tree rows"
```

---

### Task 8: Slint UI (`ui/app.slint`, `build.rs`)

**Files:**
- Create: `ui/app.slint`, `build.rs`
- Modify: `src/main.rs` (switch `slint!{}` → `slint::include_modules!()`)

**Interfaces:**
- Produces Slint-side contract (Rust wiring in Task 9):
  ```slint
  export struct TreeRow { kind: string, depth: int, expanded: bool, selected: bool, label: string, count: int }
  export struct DiffRow { kind: string, source: string, target: string }
  export global State {
      in-out property <[TreeRow]> tree;
      in-out property <[DiffRow]> diff;
      in-out property <string> status;
      in-out property <bool> busy;
      in-out property <int> source-state;   // 0 unknown, 1 ok, 2 error
      in-out property <int> target-state;
      in-out property <string> diff-title;
      callback connect(bool, string);      // is-source, connstring
      callback swap();
      callback compare();
      callback tree-clicked(int);          // flat row index
      callback copy-object();
      callback copy-all();
      callback export-object();
      callback export-all(string);         // "json" | "csv" | "sql"
  }
  export component AppWindow inherits Window { ... }
  ```

- [ ] **Step 1: Write `ui/app.slint`**

```slint
import { Button, LineEdit, ListView, VerticalBox, HorizontalBox, ComboBox, ScrollView } from "std-widgets.slint";

export struct TreeRow { kind: string, depth: int, expanded: bool, selected: bool, label: string, count: int }
export struct DiffRow { kind: string, source: string, target: string }

export global State {
    in-out property <[TreeRow]> tree: [];
    in-out property <[DiffRow]> diff: [];
    in-out property <string> status: "Enter connection strings, then Compare Schemas.";
    in-out property <bool> busy: false;
    in-out property <int> source-state: 0;
    in-out property <int> target-state: 0;
    in-out property <string> diff-title: "";
    callback connect(bool, string);
    callback swap();
    callback compare();
    callback tree-clicked(int);
    callback copy-object();
    callback copy-all();
    callback export-object();
    callback export-all(string);
}

component ConnDot inherits Rectangle {
    in property <int> state;
    width: 12px; height: 12px; border-radius: 6px;
    background: state == 1 ? #2e7d32 : state == 2 ? #c62828 : #9e9e9e;
}

component DiffCell inherits Rectangle {
    in property <string> text;
    in property <string> kind;   // "added" | "removed" | "modified" | "unchanged"
    in property <bool> tinted;   // whether this half participates in the diff color
    background: !tinted ? #fafafa :
        kind == "added" ? #d4f7d4 : kind == "removed" ? #f7d4d4 :
        kind == "modified" ? #fdf3c4 : #ffffff;
    Text {
        text: parent.text; x: 6px; width: parent.width - 12px;
        font-family: "monospace"; font-size: 12px;
        color: !parent.tinted ? #888888 :
            parent.kind == "added" ? #1a5c1a : parent.kind == "removed" ? #8a1f1f :
            parent.kind == "modified" ? #7a5c00 : #1a1a1a;
        vertical-alignment: center; overflow: elide;
    }
}

export component AppWindow inherits Window {
    title: "MSSQL Schema Compare";
    preferred-width: 1200px; preferred-height: 760px;
    min-width: 800px; min-height: 500px;

    function row-bg(sel: bool) -> color { sel ? #bbdefb : transparent }

    VerticalBox {
        padding: 8px; spacing: 6px;

        // --- connection bar ---
        HorizontalBox {
            spacing: 6px;
            Text { text: "Source:"; vertical-alignment: center; }
            src := LineEdit { placeholder-text: "Server=host;Database=DB;User Id=sa;Password=…;TrustServerCertificate=true"; enabled: !State.busy; }
            ConnDot { state: State.source-state; }
            Button { text: "Connect"; enabled: !State.busy; clicked => { State.connect(true, src.text); } }
        }
        HorizontalBox {
            spacing: 6px;
            Text { text: "Target:"; vertical-alignment: center; }
            tgt := LineEdit { placeholder-text: "Server=host;Database=DB;User Id=sa;Password=…;TrustServerCertificate=true"; enabled: !State.busy; }
            ConnDot { state: State.target-state; }
            Button { text: "Connect"; enabled: !State.busy; clicked => { State.connect(false, tgt.text); } }
        }
        HorizontalBox {
            spacing: 8px;
            Button { text: "Swap"; enabled: !State.busy; clicked => { State.swap(); } }
            Button { text: "Compare Schemas"; primary: true; enabled: !State.busy; clicked => { State.compare(); } }
            Rectangle { horizontal-stretch: 1; }
            Text { text: "Export format:"; vertical-alignment: center; }
            fmt := ComboBox { model: ["sql", "json", "csv"]; current-index: 0; width: 90px; }
        }

        // --- main split ---
        HorizontalBox {
            spacing: 6px; vertical-stretch: 1;

            Rectangle {
                width: 34%; border-width: 1px; border-color: #cccccc; background: #ffffff;
                VerticalBox {
                    padding: 4px; spacing: 0;
                    Text { text: "Objects"; font-weight: 700; }
                    ListView {
                        for row[idx] in State.tree: Rectangle {
                            height: 22px;
                            background: root.row-bg(row.selected);
                            HorizontalLayout {
                                padding-left: row.depth * 14px; spacing: 4px;
                                Text {
                                    text: row.kind == "group" ? (row.expanded ? "▾" : "▸")
                                        : row.kind == "object" ? (row.expanded ? "▾" : "▸") : " ";
                                    width: 14px; vertical-alignment: center;
                                }
                                Text { text: row.label; vertical-alignment: center; overflow: elide; }
                                Text {
                                    text: row.count >= 0 ? "(" + row.count + ")" : "";
                                    vertical-alignment: center; color: #666;
                                }
                            }
                            TouchArea { clicked => { State.tree-clicked(idx); } }
                        }
                    }
                }
            }

            Rectangle {
                border-width: 1px; border-color: #cccccc; background: #ffffff;
                VerticalBox {
                    padding: 4px; spacing: 0;
                    Text { text: "Diff View:" + (State.diff-title == "" ? "" : " " + State.diff-title); font-weight: 700; }
                    ListView {
                        for row in State.diff: Rectangle {
                            height: 20px;
                            HorizontalLayout {
                                spacing: 0;
                                DiffCell {
                                    text: row.source; kind: row.kind;
                                    tinted: row.kind == "removed" || row.kind == "modified" || row.kind == "unchanged";
                                }
                                Rectangle { width: 1px; background: #e0e0e0; }
                                DiffCell {
                                    text: row.target; kind: row.kind;
                                    tinted: row.kind == "added" || row.kind == "modified" || row.kind == "unchanged";
                                }
                            }
                        }
                    }
                    HorizontalBox {
                        spacing: 6px;
                        Button { text: "Copy Changes"; enabled: State.diff.length > 0; clicked => { State.copy-object(); } }
                        Button { text: "Export SQL"; enabled: State.diff.length > 0; clicked => { State.export-object(); } }
                        Button { text: "Copy All"; enabled: State.tree.length > 0; clicked => { State.copy-all(); } }
                        Button { text: "Export All"; enabled: State.tree.length > 0; clicked => { State.export-all(fmt.current-value); } }
                    }
                }
            }
        }

        // --- status bar ---
        Rectangle {
            height: 24px; background: #f0f0f0; border-width: 1px; border-color: #d0d0d0;
            Text {
                x: 8px; text: (State.busy ? "⏳ " : "") + State.status;
                vertical-alignment: center;
                color: State.source-state == 2 || State.target-state == 2 ? #c62828 : #333333;
            }
        }
    }
}
```

`build.rs`:
```rust
fn main() {
    slint_build::compile("ui/app.slint").expect("slint compile");
}
```

`src/main.rs` (interim — full wiring in Task 9):
```rust
slint::include_modules!();

fn main() -> Result<(), slint::PlatformError> {
    AppWindow::new()?.run()
}
```

Note: with `include_modules!`, the file is `ui/app.slint` → module `app` → `AppWindow` resolves via `use` generated code — actually generated names land in `slint::include_modules!()` output at crate root of the *binary*, so `AppWindow` is directly in scope. If resolution complains, add `use schemadiff::`… no — for a binary crate, `include_modules!` in `main.rs` puts `AppWindow` in `main.rs` scope. `State` is accessed as `app.global::<State>()` where `State` is the generated global name (Slint generates `pub struct State` for `export global State` — accessed `ui.global::<State>()`).

- [ ] **Step 2: Compile**

Run: `cargo check`
Expected: clean. If Slint reports unknown import/struct fields, fix names (`ComboBox.current-value`, `Button.primary` exist in 1.x; `ListView` needs a `for` over a model — done).

- [ ] **Step 3: Smoke**

Run: `timeout 8 cargo run`
Expected: window opens with connection bar, empty panels, status bar.

- [ ] **Step 4: Commit**

```bash
git add ui/app.slint build.rs src/main.rs && git commit -m "feat: slint ui layout with state global"
```

---

### Task 9: Backend + wiring (`src/backend.rs`, `src/main.rs`)

**Files:**
- Create: `src/backend.rs`
- Modify: `src/main.rs`, `src/lib.rs` (`pub mod backend;`)

**Interfaces:**
- Produces:
  ```rust
  pub enum Which { Source, Target }
  pub enum Cmd {
      Test { which: Which, cs: String },
      Compare { src_cs: String, tgt_cs: String },
  }
  pub enum Reply {
      Connected { which_is_source: bool, ok: bool, msg: String },
      Result(Box<CompareResult>),
      Failed(String),
      Idle,   // busy finished (also implied by Result/Failed)
  }
  /// Spawn tokio thread + command loop; returns cmd sender.
  pub fn start(reply_tx: std::sync::mpsc::Sender<Reply>) -> tokio::sync::mpsc::UnboundedSender<Cmd>
  ```
- Consumes: `schema::{connect,fetch_schema}`, `compare::{compare,CompareResult}`, `model::Direction::SourceToTarget`.

`start` implementation:
```rust
pub fn start(reply_tx: Sender<Reply>) -> UnboundedSender<Cmd> {
    let (cmd_tx, mut cmd_rx) = unbounded_channel::<Cmd>();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("tokio");
        rt.block_on(async move {
            while let Some(cmd) = cmd_rx.recv().await {
                match cmd {
                    Cmd::Test { which, cs } => {
                        let r = connect(&cs).await.map(|_| ());
                        let _ = reply_tx.send(Reply::Connected {
                            which_is_source: matches!(which, Which::Source),
                            ok: r.is_ok(),
                            msg: r.err().map(|e| format!("{e:#}")).unwrap_or_default(),
                        });
                    }
                    Cmd::Compare { src_cs, tgt_cs } => {
                        let r = run_compare(&src_cs, &tgt_cs).await;
                        let _ = reply_tx.send(match r {
                            Ok(res) => Reply::Result(Box::new(res)),
                            Err(e) => Reply::Failed(format!("{e:#}")),
                        });
                    }
                }
            }
        });
    });
    cmd_tx
}
async fn run_compare(src_cs: &str, tgt_cs: &str) -> anyhow::Result<CompareResult> {
    let mut a = connect(src_cs).await.context("source connect")?;
    let mut b = connect(tgt_cs).await.context("target connect")?;
    let s = fetch_schema(&mut a).await.context("source schema fetch")?;
    let t = fetch_schema(&mut b).await.context("target schema fetch")?;
    Ok(compare(&s, &t, Direction::SourceToTarget))
}
```

`main.rs` wiring (replace file):
```rust
use schemadiff::{backend::{self, Cmd, Reply}, compare::CompareResult, export, tree};
use slint::{ModelRc, SharedString, VecModel};
use std::cell::RefCell;
use std::collections::BTreeSet;
use std::rc::Rc;
use std::sync::mpsc;

slint::include_modules!();

struct Store {
    result: Option<CompareResult>,
    expanded: BTreeSet<(usize, Option<usize>)>,
    selected: Option<(usize, usize)>,
}

fn main() -> Result<(), slint::PlatformError> {
    let app = AppWindow::new()?;
    let state = app.global::<State>();

    let (reply_tx, reply_rx) = mpsc::channel::<Reply>();
    let cmd_tx = backend::start(reply_tx);
    let store = Rc::new(RefCell::new(Store {
        result: None, expanded: BTreeSet::new(), selected: None,
    }));

    // Slint models
    let tree_model = Rc::new(VecModel::<TreeRow>::from(vec![]));
    let diff_model = Rc::new(VecModel::<DiffRow>::from(vec![]));
    state.set_tree(ModelRc::from(tree_model.clone()));
    state.set_diff(ModelRc::from(diff_model.clone()));

    // --- callbacks ---
    {
        let cmd_tx = cmd_tx.clone();
        state.on_connect(move |is_source, cs| {
            let _ = cmd_tx.send(Cmd::Test {
                which: if is_source { backend::Which::Source } else { backend::Which::Target },
                cs: cs.to_string(),
            });
        });
    }
    // swap: handled in Rust to also swap status dots + labels? The texts live in
    // Slint LineEdits — expose two in-out props? Simplest: do swap in Slint.
    // → move `swap` to Slint-side (see note below), no Rust callback needed.
    {
        let cmd_tx = cmd_tx.clone();
        state.on_compare(move || { /* need conn strings → see note */ });
    }
    // ... tree-clicked, copy/export callbacks ...
}
```

**Problem:** callbacks need the LineEdit texts. Fix inside `app.slint` (Task 8 file — executor edits it here): change signatures to carry data —
```slint
callback compare(string, string);        // src_cs, tgt_cs
callback swap() -> (string, string)?;    // NO — instead:
```
Simplest: keep `swap()` pure Slint in the button handler:
```slint
clicked => { let tmp = src.text; src.text = tgt.text; tgt.text = tmp; }
```
(remove `callback swap` from `State`). And `compare` passes texts:
```slint
clicked => { State.compare(src.text, tgt.text); }
```
So final `State` callbacks: `connect(bool,string)`, `compare(string,string)`, `tree-clicked(int)`, `copy-object()`, `copy-all()`, `export-object()`, `export-all(string)`. Update Task 8 file accordingly when implementing this task.

Remaining `main.rs` wiring:

```rust
// tree-clicked
{
    let store = store.clone(); let tree_model = tree_model.clone();
    let diff_model = diff_model.clone(); let state2 = app.global::<State>();
    state.on_tree_clicked(move |idx| {
        let mut st = store.borrow_mut();
        let Some(res) = &st.result else { return };
        let rows = tree::flatten(res, &st.expanded, st.selected);
        let Some(row) = rows.get(idx as usize) else { return };
        match row.row_kind {
            tree::TreeRowKind::Group => {
                let k = (row.group, None);
                if !st.expanded.remove(&k) { st.expanded.insert(k); }
            }
            tree::TreeRowKind::Object => {
                let k = (row.group, row.object.unwrap());
                if !st.expanded.remove(&k) { st.expanded.insert(k); }
                st.selected = Some(k);
                // populate diff view
                let o = &res.groups[k.0].objects[k.1];
                push_diff(&diff_model, o);
                state2.set_diff_title(format!("{}.{}", o.schema, o.name).into());
            }
            tree::TreeRowKind::Detail => {
                if let Some(oi) = row.object { st.selected = Some((row.group, oi)); }
            }
        }
        push_tree(&tree_model, res, &st.expanded, st.selected);
    });
}
```

Helpers in `main.rs`:
```rust
fn push_tree(model: &VecModel<TreeRow>, res: &CompareResult,
             expanded: &BTreeSet<(usize, Option<usize>)>, sel: Option<(usize,usize)>) {
    let entries = tree::flatten(res, expanded, sel);
    model.set_vec(entries.iter().map(|e| TreeRow {
        kind: match e.row_kind {
            tree::TreeRowKind::Group => "group",
            tree::TreeRowKind::Object => "object",
            tree::TreeRowKind::Detail => "detail",
        }.into(),
        depth: e.depth as i32, expanded: e.expanded, selected: e.selected,
        label: e.label.clone().into(), count: e.count,
    }).collect::<Vec<_>>());
}
fn push_diff(model: &VecModel<DiffRow>, o: &schemadiff::compare::ObjectDiff) {
    model.set_vec(o.lines.iter().map(|l| DiffRow {
        kind: match l.kind {
            schemadiff::diff::LineKind::Added => "added",
            schemadiff::diff::LineKind::Removed => "removed",
            schemadiff::diff::LineKind::Modified => "modified",
            schemadiff::diff::LineKind::Unchanged => "unchanged",
        }.into(),
        source: l.source.clone().into(), target: l.target.clone().into(),
    }).collect());
}
```

Reply polling timer:
```rust
let timer = slint::Timer::default();
{
    let store = store.clone(); let state = app.global::<State>();
    let tree_model = tree_model.clone();
    timer.start(slint::TimerMode::Repeated, std::time::Duration::from_millis(120),
        move || {
            while let Ok(r) = reply_rx.try_recv() {
                match r {
                    Reply::Connected{which_is_source, ok, msg} => {
                        if which_is_source { state.set_source_state(if ok {1} else {2}); }
                        else { state.set_target_state(if ok {1} else {2}); }
                        if !ok { state.set_status(format!("Connection failed: {msg}").into()); }
                        else { state.set_status("Connected.".into()); }
                    }
                    Reply::Result(res) => {
                        let mut st = store.borrow_mut();
                        st.expanded.clear(); st.selected = None;
                        state.set_status(format!("{} objects compared, {} differences found.",
                            res.compared, res.different).into());
                        push_tree(&tree_model, &res, &st.expanded, None);
                        st.result = Some(*res);
                        state.set_busy(false);
                    }
                    Reply::Failed(m) => {
                        state.set_status(format!("Compare failed: {m}").into());
                        state.set_busy(false);
                    }
                    Reply::Idle => {}
                }
            }
        });
}
```
(Note borrow ordering: `push_tree(&tree_model, &res, ...)` needs `res` before `st.result = Some(*res)` — read fields first or reborrow; executor handles.)

Copy/export callbacks (UI thread, `arboard` + `rfd`):
```rust
state.on_copy_object({ let store=store.clone(); move || {
    let st = store.borrow();
    if let (Some(res), Some((g,o))) = (&st.result, st.selected) {
        let text = export::unified_text(&res.groups[g].objects[o]);
        let _ = arboard::Clipboard::new().map(|mut c| c.set_text(text));
    }
}});
state.on_copy_all({ let store=store.clone(); move || { /* unified_text_all → clipboard */ }});
state.on_export_object({ let store=store.clone(); move || {
    // rfd::FileDialog::new().set_file_name("sch.name.sql").save_file()
    // → export::sql_for_object(obj, res.direction) → std::fs::write
}});
state.on_export_all({ let store=store.clone(); move |fmt: SharedString| {
    // match fmt.as_str(): "json"→to_json .json, "csv"→to_csv .csv, _→sql_for_all .sql
    // rfd save dialog with filter + extension, then write
}});
```

- [ ] **Step 1: Adjust `ui/app.slint`** — remove `callback swap`, change `compare` signature to `(string, string)`, move swap logic into the Swap button, Compare button passes `src.text, tgt.text`.

- [ ] **Step 2: Write `src/backend.rs`** per code above.

- [ ] **Step 3: Rewrite `src/main.rs`** per code above, all callbacks.

- [ ] **Step 4: Compile + unit tests**

Run: `cargo check && cargo test`
Expected: clean; existing tests still pass.

- [ ] **Step 5: Smoke**

Run: `timeout 8 cargo run` — window opens, buttons enabled, tree empty. (Live compare verified in Task 10.)

- [ ] **Step 6: Commit**

```bash
git add src/backend.rs src/main.rs src/lib.rs ui/app.slint && git commit -m "feat: backend command loop and ui wiring"
```

---

### Task 10: Dev DB, docs, integration test

**Files:**
- Create: `compose.yml`, `Makefile`, `dev/seed_src.sql`, `dev/seed_tgt.sql`, `tests/itest.rs`, `README.md`, `AGENTS.md`, `docs/development.md`, `.omp/lsp.json`

`compose.yml`:
```yaml
services:
  mssql:
    image: mcr.microsoft.com/mssql/server:2022-latest
    environment:
      ACCEPT_EULA: "Y"
      MSSQL_SA_PASSWORD: "SchemaDiff#dev1"
    ports: ["1433:1433"]
```

`Makefile`:
```makefile
CNAME := schemadiff-mssql
SA := SchemaDiff#dev1

run:      ; cargo run
check:    ; cargo check
test:     ; cargo test
db-up:
	podman run -d --name $(CNAME) --replace -e ACCEPT_EULA=Y -e 'MSSQL_SA_PASSWORD=$(SA)' -p 1433:1433 mcr.microsoft.com/mssql/server:2022-latest
	@echo "wait ~20s for SQL Server, then: make db-seed"
db-seed:
	podman exec -i $(CNAME) /opt/mssql-tools18/bin/sqlcmd -S localhost -U sa -P '$(SA)' -C < dev/seed_src.sql
	podman exec -i $(CNAME) /opt/mssql-tools18/bin/sqlcmd -S localhost -U sa -P '$(SA)' -C < dev/seed_tgt.sql
db-down:  ; podman rm -f $(CNAME)
itest:
	SCHEMADIFF_ITEST=1 cargo test --test itest -- --nocapture
```

`dev/seed_src.sql` (full content — creates `schemadiff_src`):
```sql
CREATE DATABASE schemadiff_src;
GO
USE schemadiff_src;
GO
CREATE TABLE dbo.Users (
    id INT IDENTITY(1,1) NOT NULL,
    name NVARCHAR(50) NOT NULL,
    age INT NULL,
    created_at DATETIME NULL DEFAULT (GETDATE()),
    CONSTRAINT PK_Users PRIMARY KEY (id)
);
GO
CREATE TABLE dbo.Orders (
    id INT IDENTITY(1,1) NOT NULL,
    user_id INT NOT NULL,
    total DECIMAL(18,2) NOT NULL,
    status NVARCHAR(20) NULL,
    CONSTRAINT PK_Orders PRIMARY KEY (id),
    CONSTRAINT FK_Orders_Users FOREIGN KEY (user_id) REFERENCES dbo.Users(id),
    CONSTRAINT CK_Orders_Total CHECK (total >= 0)
);
GO
CREATE TABLE dbo.Products (
    id INT NOT NULL,
    sku NVARCHAR(20) NOT NULL,
    price DECIMAL(10,2) NOT NULL,
    CONSTRAINT PK_Products PRIMARY KEY (id),
    CONSTRAINT UQ_Products_Sku UNIQUE (sku)
);
GO
CREATE INDEX IX_Users_Name ON dbo.Users(name);
GO
CREATE VIEW dbo.ActiveUsers AS SELECT id, name FROM dbo.Users;
GO
CREATE PROCEDURE dbo.GetUser @id INT AS SELECT * FROM dbo.Users WHERE id = @id;
GO
CREATE FUNCTION dbo.fn_FormatName(@n NVARCHAR(50)) RETURNS NVARCHAR(60) AS
BEGIN RETURN UPPER(@n) END;
GO
CREATE TRIGGER dbo.trg_Users_Audit ON dbo.Users AFTER INSERT AS SELECT 1;
GO
CREATE TYPE dbo.EmailType FROM NVARCHAR(100) NULL;
GO
```

`dev/seed_tgt.sql` (creates `schemadiff_tgt` — diffs: name widened, age dropped, email added, created_at→datetime2, Orders gains shipped_at + index, Products identical, AuditLog new, view/proc bodies changed, trigger absent, EmailType wider, NewProc added):
```sql
CREATE DATABASE schemadiff_tgt;
GO
USE schemadiff_tgt;
GO
CREATE TABLE dbo.Users (
    id INT IDENTITY(1,1) NOT NULL,
    name NVARCHAR(100) NOT NULL,
    email NVARCHAR(100) NULL,
    created_at DATETIME2 NULL DEFAULT (SYSDATETIME()),
    CONSTRAINT PK_Users PRIMARY KEY (id)
);
GO
CREATE TABLE dbo.Orders (
    id INT IDENTITY(1,1) NOT NULL,
    user_id INT NOT NULL,
    total DECIMAL(18,2) NOT NULL,
    status NVARCHAR(20) NULL,
    shipped_at DATETIME2 NULL,
    CONSTRAINT PK_Orders PRIMARY KEY (id),
    CONSTRAINT FK_Orders_Users FOREIGN KEY (user_id) REFERENCES dbo.Users(id),
    CONSTRAINT CK_Orders_Total CHECK (total >= 0)
);
GO
CREATE TABLE dbo.Products (
    id INT NOT NULL,
    sku NVARCHAR(20) NOT NULL,
    price DECIMAL(10,2) NOT NULL,
    CONSTRAINT PK_Products PRIMARY KEY (id),
    CONSTRAINT UQ_Products_Sku UNIQUE (sku)
);
GO
CREATE TABLE dbo.AuditLog (id INT NOT NULL, msg NVARCHAR(200) NULL,
    CONSTRAINT PK_AuditLog PRIMARY KEY (id));
GO
CREATE INDEX IX_Users_Name ON dbo.Users(name);
GO
CREATE INDEX IX_Orders_Shipped ON dbo.Orders(shipped_at);
GO
CREATE VIEW dbo.ActiveUsers AS SELECT id, name, email FROM dbo.Users WHERE email IS NOT NULL;
GO
CREATE PROCEDURE dbo.GetUser @id INT AS SELECT id, name, email FROM dbo.Users WHERE id = @id;
GO
CREATE FUNCTION dbo.fn_FormatName(@n NVARCHAR(50)) RETURNS NVARCHAR(60) AS
BEGIN RETURN UPPER(@n) END;
GO
CREATE PROCEDURE dbo.NewProc AS SELECT 1;
GO
CREATE TYPE dbo.EmailType FROM NVARCHAR(200) NULL;
GO
```

`tests/itest.rs`:
```rust
use schemadiff::{compare::{compare, Presence}, model::Direction, schema::{connect, fetch_schema}};

const CS: &str = "Server=localhost,1433;User Id=sa;Password=SchemaDiff#dev1;TrustServerCertificate=true;Database=";

#[tokio::test(flavor = "multi_thread")]
async fn live_schema_diff() {
    if std::env::var("SCHEMADIFF_ITEST").is_err() { return; }
    let mut a = connect(&format!("{CS}schemadiff_src")).await.unwrap();
    let mut b = connect(&format!("{CS}schemadiff_tgt")).await.unwrap();
    let src = fetch_schema(&mut a).await.unwrap();
    let tgt = fetch_schema(&mut b).await.unwrap();
    let res = compare(&src, &tgt, Direction::SourceToTarget);
    assert!(res.compared >= 9);
    assert!(res.different >= 6);
    let tables = res.groups.iter().find(|g| g.kind == schemadiff::model::ObjKind::Table).unwrap();
    let audit = tables.objects.iter().find(|o| o.name == "AuditLog").unwrap();
    assert_eq!(audit.presence, Presence::OnlyTarget);
    let users = tables.objects.iter().find(|o| o.name == "Users").unwrap();
    assert!(users.change_count >= 4); // name mod, age drop, email add, created_at mod
    let procs = res.groups.iter().find(|g| g.kind == schemadiff::model::ObjKind::Procedure).unwrap();
    assert_eq!(procs.objects.iter().find(|o| o.name == "NewProc").unwrap().presence, Presence::OnlyTarget);
    let triggers = res.groups.iter().find(|g| g.kind == schemadiff::model::ObjKind::Trigger).unwrap();
    assert_eq!(triggers.objects[0].presence, Presence::OnlySource);
}
```

`README.md` — build (`cargo run`), deps note (podman for dev DB), connection string examples (ADO + JDBC + `TrustServerCertificate=true`), feature list, Wayland note (`GTK_IM_MODULE=simple` hint per spec), screenshots placeholder-free.

`AGENTS.md` — project map (module table), commands (`cargo check/test/run`, `make db-up/db-seed/itest`), conventions (canonical text in render.rs is the single diff source; don't add a second diff path).

`docs/development.md` — podman setup, seed layout, how to add a new object type (model → schema query → render → compare match arm → slint group).

`.omp/lsp.json`:
```json
{
  "servers": {
    "rust-analyzer": {}
  }
}
```
(Empty override = explicitly pin rust-analyzer for this repo; auto-detection would work anyway via `Cargo.toml`.)

- [ ] **Step 1: Write all files above.**

- [ ] **Step 2: Bring up DB and seed**

Run: `make db-up` then wait ~20s, `make db-seed`
Expected: seed scripts run without errors (`sqlcmd` exits 0; `Msg` lines for objects are fine).

- [ ] **Step 3: Run integration test**

Run: `make itest`
Expected: `live_schema_diff` PASS. If tiberius rejects the conn string/TLS: try `Encrypt=false` instead of `TrustServerCertificate`, or `tds80` feature.

- [ ] **Step 4: Full app smoke**

Run: `cargo run`; enter `Server=localhost,1433;User Id=sa;Password=SchemaDiff#dev1;TrustServerCertificate=true;Database=schemadiff_src` / `_tgt`; Connect ×2 (dots green); Compare Schemas; expand Tables → Users; click it (diff view shows colored rows); Copy Changes (paste somewhere); Export All → sql to a file.
Expected: all objects listed; Users shows ≥4 changes; status bar shows counts.

- [ ] **Step 5: Commit**

```bash
git add -A && git commit -m "feat: dev database fixtures, integration test, docs"
```

---

## Self-Review Notes

- Spec coverage: connections ✓ (T8/T9), six object types ✓ (T3–T5), git-style diff w/ colors ✓ (T2/T8), copy/export all ✓ (T6/T9), status bar ✓ (T8/T9), swap ✓, Wireframe layout ✓, Wayland ✓ (Slint default backend; README note), mise.toml/Makefile/README/AGENTS/docs/.editorconfig/LSP config ✓ (T1/T10), podman verification ✓ (T10), best-effort SQL ✓ (T6).
- `fn_FormatName` body diff not exercised — identical in both fixtures deliberately (an identical module object proves 0-diff rows render). Diff coverage comes from Users/Orders/Views/Procs/Triggers/UDT.
- Type consistency: `DiffLine`/`LineKind` (T2) used by `ObjectDiff.lines` (T5) → Slint `DiffRow` map (T9); `TreeEntry` (T7) → Slint `TreeRow` (T9). Slint global named `State`, accessed `app.global::<State>()`.
- Risk: tiberius 0.13 `tds73` feature name / TLS flavor — fallback notes in T10 step 3. `Row::get` index types accept `&str` column names.
- Risk: Slint `ComboBox`/`Button.primary`/`ListView` API drift in 1.18 — `cargo check` in T8 catches.

//! Schema extraction from a live MSSQL database via tiberius.

use anyhow::{Context, Result};
use std::collections::BTreeMap;
use tiberius::{Client, Config, FromSql, Row};
use tokio::net::TcpStream;
use tokio_util::compat::TokioAsyncWriteCompatExt;

use crate::model::*;
use crate::render::type_name;

pub type DbClient = Client<tokio_util::compat::Compat<TcpStream>>;

/// Detect "jdbc:" prefix → JDBC parse, else ADO; fall back to the other parser on error.
pub fn parse_config(cs: &str) -> Result<Config> {
    let trimmed = cs.trim();
    if trimmed.to_ascii_lowercase().starts_with("jdbc:") {
        Config::from_jdbc_string(trimmed)
            .or_else(|_| Config::from_ado_string(trimmed))
            .context("bad connection string")
    } else {
        Config::from_ado_string(trimmed)
            .or_else(|_| Config::from_jdbc_string(trimmed))
            .context("bad connection string")
    }
}

pub async fn connect(cs: &str) -> Result<DbClient> {
    let config = parse_config(cs)?;
    let tcp = TcpStream::connect(config.get_addr()).await.context("tcp connect")?;
    tcp.set_nodelay(true).context("tcp set_nodelay")?;
    Client::connect(config, tcp.compat_write()).await.context("tds login")
}

const Q_COLUMNS: &str = r#"
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
"#;

const Q_MODULES: &str = r#"
SELECT s.name AS sch, o.name, o.type, m.definition
FROM sys.objects o
JOIN sys.schemas s ON s.schema_id = o.schema_id
JOIN sys.sql_modules m ON m.object_id = o.object_id
WHERE o.type IN ('V','P','FN','IF','TF','TR') AND o.is_ms_shipped = 0
ORDER BY s.name, o.name;
"#;

const Q_INDEXES: &str = r#"
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
"#;

const Q_CHECKS: &str = r#"
SELECT s.name AS sch, o.name AS tbl, cc.name, cc.definition
FROM sys.check_constraints cc
JOIN sys.objects o ON o.object_id = cc.parent_object_id
JOIN sys.schemas s ON s.schema_id = o.schema_id
WHERE o.is_ms_shipped = 0;
"#;

const Q_FKS: &str = r#"
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
"#;

const Q_UDTS: &str = r#"
SELECT s.name AS sch, t.name, b.name AS base_typ, t.max_length, t.precision, t.scale, t.is_nullable
FROM sys.types t
JOIN sys.schemas s ON s.schema_id = t.schema_id
JOIN sys.types b ON b.user_type_id = t.system_type_id
WHERE t.is_user_defined = 1;
"#;

/// Run all six catalog queries and assemble a `Schema`. A failed query lands
/// in the returned warnings instead of aborting the fetch; the schema holds
/// whatever succeeded.
pub async fn fetch_schema(client: &mut DbClient) -> (Schema, Vec<String>) {
    async fn run(client: &mut DbClient, label: &str, q: &str) -> Result<Vec<Row>> {
        client
            .simple_query(q)
            .await
            .with_context(|| format!("schema query: {label}"))?
            .into_first_result()
            .await
            .with_context(|| format!("schema query rows: {label}"))
    }

    let mut warnings: Vec<String> = Vec::new();
    let mut tables = TableMap::new();
    let mut modules = Vec::new();
    let mut udts = Vec::new();

    if let Err(e) = run(client, "columns", Q_COLUMNS)
        .await
        .and_then(|r| rows_to_tables(&r).map(|t| tables = t))
    {
        warnings.push(format!("columns: {e:#}"));
    }
    if let Err(e) = run(client, "modules", Q_MODULES)
        .await
        .and_then(|r| rows_to_modules(&r).map(|m| modules = m))
    {
        warnings.push(format!("modules: {e:#}"));
    }
    for (label, q, apply) in [
        ("indexes", Q_INDEXES, apply_index_rows as fn(&mut TableMap, &[Row]) -> Result<()>),
        ("checks", Q_CHECKS, apply_check_rows as _),
        ("fks", Q_FKS, apply_fk_rows as _),
    ] {
        if let Err(e) = run(client, label, q)
            .await
            .and_then(|r| apply(&mut tables, &r))
        {
            warnings.push(format!("{label}: {e:#}"));
        }
    }
    if let Err(e) = run(client, "udts", Q_UDTS)
        .await
        .and_then(|r| rows_to_udts(&r).map(|u| udts = u))
    {
        warnings.push(format!("udts: {e:#}"));
    }

    (
        Schema {
            tables: tables.into_values().collect(),
            modules,
            udts,
        },
        warnings,
    )
}

type TableKey = (String, String);
type TableMap = BTreeMap<TableKey, TableSchema>;

fn req<'a, T>(row: &'a Row, col: &'static str) -> Result<T>
where
    T: FromSql<'a>,
{
    row.try_get::<T, _>(col)
        .with_context(|| format!("decode `{col}`"))?
        .with_context(|| format!("unexpected NULL in `{col}`"))
}

fn opt<'a, T>(row: &'a Row, col: &'static str) -> Result<Option<T>>
where
    T: FromSql<'a>,
{
    row.try_get::<T, _>(col)
        .with_context(|| format!("decode `{col}`"))
}

fn map_obj_kind(t: &str) -> ObjKind {
    // sys.objects.type is char(2) and can arrive space-padded.
    match t.trim() {
        "V" => ObjKind::View,
        "P" => ObjKind::Procedure,
        "FN" | "IF" | "TF" => ObjKind::Function,
        "TR" => ObjKind::Trigger,
        _ => ObjKind::Table,
    }
}

fn paren_list(cols: &[String]) -> String {
    format!("({})", cols.join(", "))
}

/// Q1 → tables keyed (schema, table); columns arrive already column_id-sorted.
fn rows_to_tables(rows: &[Row]) -> Result<TableMap> {
    let mut map = TableMap::new();
    for row in rows {
        let sch = req::<&str>(row, "sch")?.to_string();
        let tbl = req::<&str>(row, "tbl")?.to_string();
        let t = map
            .entry((sch.clone(), tbl.clone()))
            .or_insert_with(|| TableSchema {
                schema: sch,
                name: tbl,
                columns: vec![],
                constraints: vec![],
                indexes: vec![],
            });
        t.columns.push(Column {
            name: req::<&str>(row, "col")?.to_string(),
            data_type: type_name(
                req::<&str>(row, "typ")?,
                req::<i16>(row, "max_length")?,
                req::<u8>(row, "precision")?,
                req::<u8>(row, "scale")?,
            ),
            nullable: req::<bool>(row, "is_nullable")?,
            identity: req::<bool>(row, "is_identity")?,
            default: opt::<&str>(row, "dflt")?.map(str::to_string),
        });
    }
    Ok(map)
}

/// A NULL definition means the object is encrypted or the login lacks VIEW
/// DEFINITION; keep the object with a placeholder so it shows as a content
/// diff rather than disappearing from the compare.
fn module_object(sch: &str, name: &str, ty: &str, definition: Option<&str>) -> ModuleObject {
    ModuleObject {
        schema: sch.to_string(),
        name: name.to_string(),
        kind: map_obj_kind(ty),
        definition: definition
            .unwrap_or(
                "-- definition unavailable (encrypted or insufficient VIEW DEFINITION permission)",
            )
            .to_string(),
    }
}

/// Q2 → module objects (views, procs, functions, triggers).
fn rows_to_modules(rows: &[Row]) -> Result<Vec<ModuleObject>> {
    rows.iter()
        .map(|row| {
            Ok(module_object(
                req::<&str>(row, "sch")?,
                req::<&str>(row, "name")?,
                req::<&str>(row, "type")?,
                opt::<&str>(row, "definition")?,
            ))
        })
        .collect()
}

struct IndexAcc {
    unique: bool,
    primary: bool,
    unique_constraint: bool,
    key_cols: Vec<String>,
    included: Vec<String>,
}

/// Q3 → Pk/Uq `Constraint`s (key cols only) or `IndexDef`s (key + included cols).
fn apply_index_rows(tables: &mut TableMap, rows: &[Row]) -> Result<()> {
    let mut groups: BTreeMap<(String, String, String), IndexAcc> = BTreeMap::new();
    for row in rows {
        let sch = req::<&str>(row, "sch")?.to_string();
        let tbl = req::<&str>(row, "tbl")?.to_string();
        let name = req::<&str>(row, "name")?.to_string();
        let g = groups
            .entry((sch, tbl, name))
            .or_insert_with(|| IndexAcc {
                unique: false,
                primary: false,
                unique_constraint: false,
                key_cols: vec![],
                included: vec![],
            });
        g.unique |= req::<bool>(row, "is_unique")?;
        g.primary |= req::<bool>(row, "is_primary_key")?;
        g.unique_constraint |= req::<bool>(row, "is_unique_constraint")?;

        let mut col = req::<&str>(row, "col")?.to_string();
        if req::<bool>(row, "is_descending_key")? {
            col.push_str(" DESC");
        }
        if req::<u8>(row, "key_ordinal")? > 0 {
            g.key_cols.push(col);
        } else if req::<bool>(row, "is_included_column")? {
            g.included.push(col);
        }
    }

    for ((sch, tbl, name), g) in groups {
        let Some(t) = tables.get_mut(&(sch, tbl)) else {
            continue;
        };
        if g.primary || g.unique_constraint {
            t.constraints.push(Constraint {
                kind: if g.primary {
                    ConstraintKind::Pk
                } else {
                    ConstraintKind::Uq
                },
                name,
                text: paren_list(&g.key_cols),
            });
        } else {
            t.indexes.push(IndexDef {
                name,
                unique: g.unique,
                columns: g.key_cols,
                included: g.included,
            });
        }
    }
    Ok(())
}

/// Q4 → Check constraints. `text` is the raw definition.
fn apply_check_rows(tables: &mut TableMap, rows: &[Row]) -> Result<()> {
    for row in rows {
        let sch = req::<&str>(row, "sch")?.to_string();
        let tbl = req::<&str>(row, "tbl")?.to_string();
        let name = req::<&str>(row, "name")?.to_string();
        // NULL definition → no VIEW DEFINITION; skip the constraint row.
        let Some(definition) = opt::<&str>(row, "definition")? else {
            continue;
        };
        let definition = definition.to_string();
        let Some(t) = tables.get_mut(&(sch, tbl)) else {
            continue;
        };
        t.constraints.push(Constraint {
            kind: ConstraintKind::Check,
            name,
            text: definition,
        });
    }
    Ok(())
}

struct FkAcc {
    ref_schema: String,
    ref_table: String,
    cols: Vec<String>,
    ref_cols: Vec<String>,
}

/// Q5 → Fk constraints, `text` = `(cols) -> rsch.rtbl (rcols)`.
fn apply_fk_rows(tables: &mut TableMap, rows: &[Row]) -> Result<()> {
    let mut groups: BTreeMap<(String, String, String), FkAcc> = BTreeMap::new();
    for row in rows {
        let sch = req::<&str>(row, "sch")?.to_string();
        let tbl = req::<&str>(row, "tbl")?.to_string();
        let name = req::<&str>(row, "name")?.to_string();
        let g = groups
            .entry((sch, tbl, name))
            .or_insert_with(|| FkAcc {
                ref_schema: String::new(),
                ref_table: String::new(),
                cols: vec![],
                ref_cols: vec![],
            });
        if g.cols.is_empty() {
            g.ref_schema = req::<&str>(row, "rsch")?.to_string();
            g.ref_table = req::<&str>(row, "rtbl")?.to_string();
        }
        g.cols.push(req::<&str>(row, "col")?.to_string());
        g.ref_cols.push(req::<&str>(row, "rcol")?.to_string());
    }

    for ((sch, tbl, name), g) in groups {
        let Some(t) = tables.get_mut(&(sch, tbl)) else {
            continue;
        };
        t.constraints.push(Constraint {
            kind: ConstraintKind::Fk,
            name,
            text: format!(
                "{} -> {}.{} {}",
                paren_list(&g.cols),
                g.ref_schema,
                g.ref_table,
                paren_list(&g.ref_cols)
            ),
        });
    }
    Ok(())
}

/// Q6 → user-defined types with canonical base type name.
fn rows_to_udts(rows: &[Row]) -> Result<Vec<UdtDef>> {
    rows.iter()
        .map(|row| {
            Ok(UdtDef {
                schema: req::<&str>(row, "sch")?.to_string(),
                name: req::<&str>(row, "name")?.to_string(),
                base_type: type_name(
                    req::<&str>(row, "base_typ")?,
                    req::<i16>(row, "max_length")?,
                    req::<u8>(row, "precision")?,
                    req::<u8>(row, "scale")?,
                ),
                nullable: req::<bool>(row, "is_nullable")?,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn obj_kind_maps_all_catalog_types() {
        assert_eq!(map_obj_kind("V"), ObjKind::View);
        assert_eq!(map_obj_kind("P"), ObjKind::Procedure);
        for t in ["FN", "IF", "TF"] {
            assert_eq!(map_obj_kind(t), ObjKind::Function);
        }
        assert_eq!(map_obj_kind("TR"), ObjKind::Trigger);
    }

    #[test]
    fn obj_kind_tolerates_char_padding() {
        // sys.objects.type is char(2): 'V' may arrive as 'V '.
        assert_eq!(map_obj_kind("V "), ObjKind::View);
        assert_eq!(map_obj_kind("TR "), ObjKind::Trigger);
    }

    #[test]
    fn module_object_uses_placeholder_when_definition_null() {
        let m = module_object("dbo", "v_secret", "V", None);
        assert_eq!(m.kind, ObjKind::View);
        assert_eq!(m.schema, "dbo");
        assert!(m.definition.starts_with("-- definition unavailable"));

        let m2 = module_object("dbo", "v1", "TR ", Some("CREATE TRIGGER v1"));
        assert_eq!(m2.kind, ObjKind::Trigger);
        assert_eq!(m2.definition, "CREATE TRIGGER v1");
    }

    #[test]
    fn paren_list_renders_csv_in_parens() {
        assert_eq!(paren_list(&["a".into(), "b".into()]), "(a, b)");
        assert_eq!(paren_list(&["x".into()]), "(x)");
    }
}

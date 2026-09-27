use crate::model::*;

/// Canonical T-SQL type name from sys catalog column metadata.
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
    if n < 0 {
        format!("{b}(MAX)")
    } else {
        format!("{b}({n})")
    }
}

/// Deterministic canonical text for a table: columns in stored order,
/// constraints sorted by kind then name, indexes sorted by name.
pub fn table_text(t: &TableSchema) -> String {
    let mut s = String::new();
    for c in &t.columns {
        s.push_str(&format!(
            "COLUMN {} {} {}{}{}\n",
            c.name,
            c.data_type,
            if c.nullable { "NULL" } else { "NOT NULL" },
            if c.identity { " IDENTITY" } else { "" },
            c.default
                .as_deref()
                .map(|d| format!(" DEFAULT {d}"))
                .unwrap_or_default()
        ));
    }
    let mut cons = t.constraints.clone();
    cons.sort_by(|a, b| (con_kind_ord(&a.kind), &a.name).cmp(&(con_kind_ord(&b.kind), &b.name)));
    for c in &cons {
        let k = match c.kind {
            ConstraintKind::Pk => "PK",
            ConstraintKind::Uq => "UQ",
            ConstraintKind::Fk => "FK",
            ConstraintKind::Check => "CHECK",
        };
        s.push_str(&format!("CONSTRAINT {k} {} {}\n", c.name, c.text));
    }
    let mut idx = t.indexes.clone();
    idx.sort_by(|a, b| a.name.cmp(&b.name));
    for i in &idx {
        s.push_str(&format!(
            "INDEX {}{} ({}){}\n",
            i.name,
            if i.unique { " UNIQUE" } else { "" },
            i.columns.join(", "),
            if i.included.is_empty() {
                String::new()
            } else {
                format!(" INCLUDE ({})", i.included.join(", "))
            }
        ));
    }
    s.trim_end().to_string()
}

fn con_kind_ord(k: &ConstraintKind) -> u8 {
    match k {
        ConstraintKind::Pk => 0,
        ConstraintKind::Uq => 1,
        ConstraintKind::Fk => 2,
        ConstraintKind::Check => 3,
    }
}

/// Canonical module text: definition with CRLF normalized to LF, trimmed.
pub fn module_text(m: &ModuleObject) -> String {
    m.definition.replace("\r\n", "\n").trim().to_string()
}

/// Canonical UDT text: `TYPE name FROM BASE [NOT] NULL`.
pub fn udt_text(u: &UdtDef) -> String {
    format!(
        "TYPE {} FROM {} {}",
        u.name,
        u.base_type,
        if u.nullable { "NULL" } else { "NOT NULL" }
    )
}

#[cfg(test)]
mod tests {
    use super::*;

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
            schema: "dbo".into(),
            name: "Users".into(),
            columns: vec![
                Column {
                    name: "id".into(),
                    data_type: "INT".into(),
                    nullable: false,
                    identity: true,
                    default: None,
                },
                Column {
                    name: "name".into(),
                    data_type: "NVARCHAR(50)".into(),
                    nullable: false,
                    identity: false,
                    default: None,
                },
            ],
            constraints: vec![Constraint {
                kind: ConstraintKind::Pk,
                name: "PK_Users".into(),
                text: "(id)".into(),
            }],
            indexes: vec![IndexDef {
                name: "IX_name".into(),
                unique: false,
                columns: vec!["name".into()],
                included: vec![],
            }],
        };
        let text = table_text(&t);
        assert!(text.contains("COLUMN id INT NOT NULL IDENTITY"));
        assert!(text.contains("COLUMN name NVARCHAR(50) NOT NULL"));
        assert!(text.contains("CONSTRAINT PK PK_Users (id)"));
        assert!(text.contains("INDEX IX_name (name)"));
    }

    #[test]
    fn module_text_normalizes_crlf() {
        let m = ModuleObject {
            schema: "dbo".into(),
            name: "v".into(),
            kind: ObjKind::View,
            definition: "CREATE VIEW v\r\nAS SELECT 1\r\n".into(),
        };
        assert_eq!(module_text(&m), "CREATE VIEW v\nAS SELECT 1");
    }
}

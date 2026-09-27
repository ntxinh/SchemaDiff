use std::collections::BTreeMap;

use crate::diff::{diff_lines, summary_entry, DiffLine, LineKind};
use crate::model::*;
use crate::render::{module_text, table_text, udt_text};
use serde::Serialize;

pub use crate::model::Presence;

#[derive(Clone, Debug, PartialEq, Serialize)]
pub enum SchemaPayload {
    Table(TableSchema),
    Module(ModuleObject),
    Udt(UdtDef),
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ObjectDiff {
    pub kind: ObjKind,
    pub schema: String,
    pub name: String,
    pub presence: Presence,
    pub lines: Vec<DiffLine>,
    pub summary: Vec<String>,
    pub change_count: usize,
    pub src: Option<SchemaPayload>,
    pub tgt: Option<SchemaPayload>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ObjectGroup {
    pub kind: ObjKind,
    pub objects: Vec<ObjectDiff>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct CompareResult {
    pub direction: Direction,
    pub groups: Vec<ObjectGroup>,
    pub compared: usize,
    pub different: usize,
    /// Partial-fetch warnings from the schema queries ("source fetch: …").
    pub warnings: Vec<String>,
}

type SideMap = BTreeMap<(String, String), (String, SchemaPayload)>;

fn objects_of(s: &Schema, kind: ObjKind) -> SideMap {
    let mut m = SideMap::new();
    match kind {
        ObjKind::Table => {
            for t in &s.tables {
                m.insert(
                    (t.schema.clone(), t.name.clone()),
                    (table_text(t), SchemaPayload::Table(t.clone())),
                );
            }
        }
        ObjKind::Udt => {
            for u in &s.udts {
                m.insert(
                    (u.schema.clone(), u.name.clone()),
                    (udt_text(u), SchemaPayload::Udt(u.clone())),
                );
            }
        }
        _ => {
            for md in s.modules.iter().filter(|md| md.kind == kind) {
                m.insert(
                    (md.schema.clone(), md.name.clone()),
                    (module_text(md), SchemaPayload::Module(md.clone())),
                );
            }
        }
    }
    m
}

/// Compare two schemas, grouped by object kind. For `TargetToSource` the
/// schemas are swapped internally so "added"/"removed" are reported from
/// the target's viewpoint; `direction` is kept on the result for labels.
pub fn compare(src: &Schema, tgt: &Schema, direction: Direction) -> CompareResult {
    let (src, tgt) = match direction {
        Direction::SourceToTarget => (src, tgt),
        Direction::TargetToSource => (tgt, src),
    };

    let mut groups = Vec::new();
    let mut compared = 0;
    let mut different = 0;

    for kind in ObjKind::ALL {
        let mut smap = objects_of(src, kind);
        let mut tmap = objects_of(tgt, kind);
        let keys: std::collections::BTreeSet<&(String, String)> =
            smap.keys().chain(tmap.keys()).collect();
        let keys: Vec<(String, String)> = keys.into_iter().cloned().collect();

        let mut objects = Vec::new();
        for key in keys {
            let sv = smap.remove(&key);
            let tv = tmap.remove(&key);
            let (presence, lines, sp, tp) = match (sv, tv) {
                (Some((st, sp)), Some((tt, tp))) => {
                    (Presence::Both, diff_lines(&st, &tt), Some(sp), Some(tp))
                }
                (Some((st, sp)), None) => (
                    Presence::OnlySource,
                    st.lines()
                        .map(|l| DiffLine {
                            kind: LineKind::Removed,
                            source: l.to_string(),
                            target: String::new(),
                        })
                        .collect(),
                    Some(sp),
                    None,
                ),
                (None, Some((tt, tp))) => (
                    Presence::OnlyTarget,
                    tt.lines()
                        .map(|l| DiffLine {
                            kind: LineKind::Added,
                            source: String::new(),
                            target: l.to_string(),
                        })
                        .collect(),
                    None,
                    Some(tp),
                ),
                (None, None) => unreachable!("key from union of both maps"),
            };
            let changed: Vec<&DiffLine> =
                lines.iter().filter(|l| l.kind != LineKind::Unchanged).collect();
            let summary = changed
                .iter()
                .map(|l| summary_entry(l.kind, &l.source, &l.target))
                .collect();
            let change_count = changed.len();

            let (schema, name) = key;
            compared += 1;
            if change_count > 0 {
                different += 1;
            }
            objects.push(ObjectDiff {
                kind,
                schema,
                name,
                presence,
                lines,
                summary,
                change_count,
                src: sp,
                tgt: tp,
            });
        }
        groups.push(ObjectGroup { kind, objects });
    }

    CompareResult {
        direction,
        groups,
        compared,
        different,
        warnings: vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn col(name: &str, ty: &str) -> Column {
        Column {
            name: name.to_string(),
            data_type: ty.to_string(),
            nullable: true,
            identity: false,
            default: None,
        }
    }

    fn sample_table() -> TableSchema {
        TableSchema {
            schema: "dbo".into(),
            name: "T".into(),
            columns: vec![col("id", "INT")],
            constraints: vec![],
            indexes: vec![],
        }
    }

    fn sample_trigger() -> ModuleObject {
        ModuleObject {
            schema: "dbo".into(),
            name: "trg".into(),
            kind: ObjKind::Trigger,
            definition: "CREATE TRIGGER dbo.trg ON dbo.T AFTER INSERT\nAS SELECT 1".into(),
        }
    }

    #[test]
    fn added_object_all_added_lines() {
        let src = Schema::empty();
        let mut tgt = Schema::empty();
        tgt.tables.push(sample_table());
        let r = compare(&src, &tgt, Direction::SourceToTarget);
        assert_eq!(r.compared, 1);
        assert_eq!(r.different, 1);
        let o = &r.groups.iter().find(|g| g.kind == ObjKind::Table).unwrap().objects[0];
        assert_eq!(o.presence, Presence::OnlyTarget);
        assert!(o.lines.iter().all(|l| l.kind == LineKind::Added));
        assert!(o.lines.len() > 0);
        assert!(!o.summary.is_empty());
    }

    #[test]
    fn direction_swap_flips_presence() {
        let src = Schema::empty();
        let mut tgt = Schema::empty();
        tgt.tables.push(sample_table());
        let r = compare(&src, &tgt, Direction::TargetToSource);
        assert_eq!(r.direction, Direction::TargetToSource);
        let o = &r.groups.iter().find(|g| g.kind == ObjKind::Table).unwrap().objects[0];
        assert_eq!(o.presence, Presence::OnlySource);
        assert!(o.lines.iter().all(|l| l.kind == LineKind::Removed));
    }

    #[test]
    fn removed_object_all_removed_lines() {
        let mut src = Schema::empty();
        src.modules.push(sample_trigger());
        let tgt = Schema::empty();
        let r = compare(&src, &tgt, Direction::SourceToTarget);
        let o = &r.groups.iter().find(|g| g.kind == ObjKind::Trigger).unwrap().objects[0];
        assert_eq!(o.presence, Presence::OnlySource);
        assert!(o.lines.iter().all(|l| l.kind == LineKind::Removed));
        assert_eq!(o.change_count, o.lines.len());
    }

    #[test]
    fn identical_objects_count_zero_still_present() {
        let mut src = Schema::empty();
        src.tables.push(sample_table());
        let tgt = src.clone();
        let r = compare(&src, &tgt, Direction::SourceToTarget);
        assert_eq!(r.compared, 1);
        assert_eq!(r.different, 0);
        let o = &r.groups.iter().find(|g| g.kind == ObjKind::Table).unwrap().objects[0];
        assert_eq!(o.presence, Presence::Both);
        assert_eq!(o.change_count, 0);
        assert!(o.lines.iter().all(|l| l.kind == LineKind::Unchanged));
        assert!(o.summary.is_empty());
    }

    #[test]
    fn modified_table_column_yields_modified_line() {
        let mut src = Schema::empty();
        src.tables.push(sample_table());
        let mut tgt = Schema::empty();
        let mut t = sample_table();
        t.columns[0].data_type = "BIGINT".into();
        tgt.tables.push(t);
        let r = compare(&src, &tgt, Direction::SourceToTarget);
        assert_eq!(r.different, 1);
        let o = &r.groups.iter().find(|g| g.kind == ObjKind::Table).unwrap().objects[0];
        assert_eq!(o.presence, Presence::Both);
        assert!(o.lines.iter().any(|l| l.kind == LineKind::Modified));
        assert_eq!(o.change_count, 1);
        assert_eq!(o.summary.len(), 1);
    }

    #[test]
    fn groups_ordered_and_objects_sorted_by_name() {
        let mut src = Schema::empty();
        src.tables.push(TableSchema { name: "Z".into(), ..sample_table() });
        src.tables.push(TableSchema { name: "A".into(), ..sample_table() });
        src.modules.push(sample_trigger());
        src.udts.push(UdtDef {
            schema: "dbo".into(),
            name: "u1".into(),
            base_type: "INT".into(),
            nullable: false,
        });
        let tgt = Schema::empty();
        let r = compare(&src, &tgt, Direction::SourceToTarget);
        assert_eq!(
            r.groups.iter().map(|g| g.kind).collect::<Vec<_>>(),
            ObjKind::ALL
        );
        let tables = &r.groups[0].objects;
        assert_eq!(
            tables.iter().map(|o| o.name.as_str()).collect::<Vec<_>>(),
            vec!["A", "Z"]
        );
        assert_eq!(r.compared, 4);
        assert_eq!(r.different, 4);
    }

    #[test]
    fn dotted_schema_names_do_not_collide() {
        // ("a.b","T") and ("a","b.c") must produce two distinct objects
        let mut src = Schema::empty();
        src.tables.push(TableSchema { schema: "a.b".into(), ..sample_table() });
        src.tables.push(TableSchema {
            schema: "a".into(),
            name: "b.c".into(),
            ..sample_table()
        });
        let tgt = Schema::empty();
        let r = compare(&src, &tgt, Direction::SourceToTarget);
        let tables = &r.groups[0].objects;
        assert_eq!(tables.len(), 2);
        assert_eq!(r.compared, 2);
        assert_eq!(tables[0].schema, "a");
        assert_eq!(tables[0].name, "b.c");
        assert_eq!(tables[1].schema, "a.b");
        assert_eq!(tables[1].name, "T");
    }
}

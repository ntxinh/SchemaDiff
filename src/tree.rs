use crate::compare::CompareResult;
use std::collections::BTreeSet;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TreeRowKind {
    Group,
    Object,
    Detail,
}

#[derive(Clone, Debug)]
pub struct TreeEntry {
    pub row_kind: TreeRowKind,
    pub group: usize,
    pub object: Option<usize>,
    pub depth: u8,
    pub expanded: bool,
    pub selected: bool,
    pub label: String,
    pub count: i32,
}

/// Flatten a compare result into row entries for the tree view.
/// expanded keys: (group_idx, None) = group open; (group_idx, Some(obj_idx)) = object open.
pub fn flatten(
    result: &CompareResult,
    expanded: &BTreeSet<(usize, Option<usize>)>,
    selected: Option<(usize, usize)>,
) -> Vec<TreeEntry> {
    let mut rows = Vec::new();
    for (g, group) in result.groups.iter().enumerate() {
        let group_open = expanded.contains(&(g, None));
        rows.push(TreeEntry {
            row_kind: TreeRowKind::Group,
            group: g,
            object: None,
            depth: 0,
            expanded: group_open,
            selected: false,
            label: group.kind.label().to_string(),
            count: -1,
        });
        if !group_open {
            continue;
        }
        for (i, obj) in group.objects.iter().enumerate() {
            let obj_open = expanded.contains(&(g, Some(i)));
            rows.push(TreeEntry {
                row_kind: TreeRowKind::Object,
                group: g,
                object: Some(i),
                depth: 1,
                expanded: obj_open,
                selected: selected == Some((g, i)),
                label: format!("{}.{}", obj.schema, obj.name),
                count: obj.change_count as i32,
            });
            if obj_open {
                for s in &obj.summary {
                    rows.push(TreeEntry {
                        row_kind: TreeRowKind::Detail,
                        group: g,
                        object: Some(i),
                        depth: 2,
                        expanded: false,
                        selected: false,
                        label: s.clone(),
                        count: -1,
                    });
                }
            }
        }
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compare::{CompareResult, ObjectDiff, ObjectGroup};
    use crate::model::{Direction, ObjKind, Presence};

    fn obj(schema: &str, name: &str, count: usize, summary: Vec<String>) -> ObjectDiff {
        ObjectDiff {
            kind: ObjKind::Table,
            schema: schema.into(),
            name: name.into(),
            presence: Presence::Both,
            lines: vec![],
            summary,
            change_count: count,
            src: None,
            tgt: None,
        }
    }

    fn sample_result() -> CompareResult {
        CompareResult {
            direction: Direction::SourceToTarget,
            groups: vec![
                ObjectGroup {
                    kind: ObjKind::Table,
                    objects: vec![
                        obj("dbo", "t1", 3, vec!["a".into(), "b".into()]),
                        obj("dbo", "t2", 0, vec![]),
                    ],
                },
                ObjectGroup {
                    kind: ObjKind::View,
                    objects: vec![obj("dbo", "v1", 1, vec!["c".into()])],
                },
            ],
            compared: 3,
            different: 2,
        }
    }

    #[test]
    fn collapsed_shows_only_groups() {
        let r = sample_result();
        let rows = flatten(&r, &BTreeSet::new(), None);
        assert_eq!(rows.len(), 2);
        assert!(rows.iter().all(|e| e.row_kind == TreeRowKind::Group));
        assert_eq!(rows[0].label, "Tables");
        assert_eq!(rows[0].depth, 0);
        assert_eq!(rows[0].count, -1);
        assert_eq!(rows[1].label, "Views");
    }

    #[test]
    fn expanded_group_lists_objects() {
        let r = sample_result();
        let mut ex = BTreeSet::new();
        ex.insert((0, None));
        let rows = flatten(&r, &ex, None);
        // Group, Object, Object, Group
        assert_eq!(rows.len(), 4);
        assert_eq!(rows[1].row_kind, TreeRowKind::Object);
        assert_eq!(rows[1].depth, 1);
        assert_eq!(rows[1].label, "dbo.t1");
        assert_eq!(rows[1].count, 3);
        assert_eq!(rows[2].label, "dbo.t2");
        assert_eq!(rows[2].count, 0);
        assert_eq!(rows[3].row_kind, TreeRowKind::Group);
    }

    #[test]
    fn expanded_group_lists_objects_then_details() {
        let r = sample_result();
        let mut ex = BTreeSet::new();
        ex.insert((0, None));
        ex.insert((0, Some(0)));
        let rows = flatten(&r, &ex, Some((0, 0)));
        // Group, Object(selected), Detail, Detail, Object, Group
        assert_eq!(rows[1].row_kind, TreeRowKind::Object);
        assert!(rows[1].selected);
        assert!(rows[1].expanded);
        assert_eq!(rows[2].row_kind, TreeRowKind::Detail);
        assert_eq!(rows[2].depth, 2);
        assert_eq!(rows[2].label, "a");
        assert_eq!(rows[3].label, "b");
        assert_eq!(rows[4].row_kind, TreeRowKind::Object);
        assert!(!rows[4].selected);
        assert!(!rows[4].expanded);
        assert_eq!(rows[5].row_kind, TreeRowKind::Group);
    }

    #[test]
    fn selected_marks_only_matching_row() {
        let r = sample_result();
        let mut ex = BTreeSet::new();
        ex.insert((0, None));
        ex.insert((1, None));
        let rows = flatten(&r, &ex, Some((1, 0)));
        let sel: Vec<_> = rows.iter().filter(|e| e.selected).collect();
        assert_eq!(sel.len(), 1);
        assert_eq!(sel[0].label, "dbo.v1");
        assert_eq!(sel[0].group, 1);
        assert_eq!(sel[0].object, Some(0));
    }

    #[test]
    fn empty_summary_object_has_no_details() {
        let r = sample_result();
        let mut ex = BTreeSet::new();
        ex.insert((0, None));
        ex.insert((0, Some(1))); // dbo.t2 has empty summary
        let rows = flatten(&r, &ex, None);
        // Group, Object, Object(expanded, no details), Group
        assert_eq!(rows.len(), 4);
        assert!(rows[2].expanded);
        assert!(!rows.iter().any(|e| e.row_kind == TreeRowKind::Detail));
    }
}

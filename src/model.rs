use serde::Serialize;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
pub enum ObjKind {
    Table,
    View,
    Procedure,
    Function,
    Trigger,
    Udt,
}

impl ObjKind {
    pub const ALL: [ObjKind; 6] = [
        ObjKind::Table,
        ObjKind::View,
        ObjKind::Procedure,
        ObjKind::Function,
        ObjKind::Trigger,
        ObjKind::Udt,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            ObjKind::Table => "Tables",
            ObjKind::View => "Views",
            ObjKind::Procedure => "Stored Procedures",
            ObjKind::Function => "Functions",
            ObjKind::Trigger => "Triggers",
            ObjKind::Udt => "User-Defined Types",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
pub enum Presence {
    Both,
    OnlySource,
    OnlyTarget,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
pub enum Direction {
    SourceToTarget,
    TargetToSource,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Column {
    pub name: String,
    pub data_type: String,
    pub nullable: bool,
    pub identity: bool,
    pub default: Option<String>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
pub enum ConstraintKind {
    Pk,
    Uq,
    Fk,
    Check,
}

/// `text` is the canonical constraint body, e.g. `(id)` or `(user_id) -> dbo.Users (id)`.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Constraint {
    pub kind: ConstraintKind,
    pub name: String,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct IndexDef {
    pub name: String,
    pub unique: bool,
    pub columns: Vec<String>,
    pub included: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct TableSchema {
    pub schema: String,
    pub name: String,
    pub columns: Vec<Column>,
    pub constraints: Vec<Constraint>,
    pub indexes: Vec<IndexDef>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ModuleObject {
    pub schema: String,
    pub name: String,
    pub kind: ObjKind,
    pub definition: String,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct UdtDef {
    pub schema: String,
    pub name: String,
    pub base_type: String,
    pub nullable: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct Schema {
    pub tables: Vec<TableSchema>,
    pub modules: Vec<ModuleObject>,
    pub udts: Vec<UdtDef>,
}

impl Schema {
    pub fn empty() -> Self {
        Self::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn obj_kind_labels() {
        assert_eq!(ObjKind::ALL.len(), 6);
        assert_eq!(ObjKind::Procedure.label(), "Stored Procedures");
        assert_eq!(ObjKind::Udt.label(), "User-Defined Types");
    }
}

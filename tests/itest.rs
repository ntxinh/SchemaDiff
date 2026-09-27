use schemadiff::{compare::{compare, Presence}, model::Direction, schema::{connect, fetch_schema}};

const CS: &str = "Server=localhost,1433;User Id=sa;Password=SchemaDiff#dev1;TrustServerCertificate=true;Database=";

#[tokio::test(flavor = "multi_thread")]
async fn live_schema_diff() {
    if std::env::var("SCHEMADIFF_ITEST").is_err() { return; }
    let mut a = connect(&format!("{CS}schemadiff_src")).await.unwrap();
    let mut b = connect(&format!("{CS}schemadiff_tgt")).await.unwrap();
    let (src, _w) = fetch_schema(&mut a).await;
    let (tgt, _w) = fetch_schema(&mut b).await;
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

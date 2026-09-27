# AGENTS.md

MSSQL schema-compare desktop app (Rust + Slint). Read-only against databases.

## Project map

| File | Role |
|------|------|
| `src/model.rs` | Schema model types: `Schema`, `TableSchema`, `ModuleObject`, `UdtDef`, `ObjKind`, `Presence`, `Direction` |
| `src/schema.rs` | tiberius connection + `fetch_schema` metadata queries (sys.* catalog views) |
| `src/render.rs` | Canonical text rendering of each object — the single source of truth for what gets diffed |
| `src/diff.rs` | Line diff (`similar`) → `DiffLine`/`LineKind`, summary entries |
| `src/compare.rs` | `compare(&Schema,&Schema,Direction) -> CompareResult` grouped `ObjectDiff`s; re-exports `Presence` |
| `src/tree.rs` | Flattens `CompareResult` + expand/select state → `TreeEntry` rows for the UI |
| `src/export.rs` | Unified text / JSON / CSV / best-effort-SQL export of diffs |
| `src/backend.rs` | Async command loop (`Cmd`/`Reply`) so DB work stays off the UI thread |
| `src/main.rs` | Slint app wiring: callbacks → backend → `VecModel`s (`TreeRow`, `DiffRow`) |
| `ui/app.slint` | Layout + `State` global (accessed via `app.global::<State>()`) |
| `dev/seed_*.sql` | Fixture databases exercising every object type |
| `tests/itest.rs` | Live integration test, gated on `SCHEMADIFF_ITEST` |

## Commands

```bash
cargo check / cargo test / cargo run   # or: make check / test / run
make db-up db-seed itest db-down       # podman dev DB + integration test
```

## Conventions

- `render.rs` canonical text is the ONLY diff source. Do not add a second
  diff path — new/changed object semantics go through render → diff → compare.
- `ObjKind` ordering drives group order in the tree; keep the enum order
  stable.
- Compare maps are keyed by `(schema, name)` tuples — never bare names.
- All DDL identifiers are escaped in render output; keep it that way.
- Edition 2021; conventional commits; no new deps without need.

# MSSQL Schema Compare — Design

Date: 2026-09-27
Status: Approved (design phase)

## Goal

Desktop app (Rust + Slint) that compares the schemas of two MSSQL databases,
shows a git-diff-style visualization, and supports copy/export of differences.
Primary target: Linux Wayland; cross-platform where free.

## Tech stack (fixed)

- Rust edition 2021, pinned via `mise.toml`
- Slint 1.x (latest stable) — `slint` + `slint-build`
- `tiberius` with `rustls` feature (no OpenSSL dep) — async TDS driver
- `tokio` `rt-multi-thread` + `macros`
- `similar` — Myers/Patience line diff
- `arboard` — clipboard (works on Wayland)
- `serde`, `serde_json`, `csv` — exports
- `anyhow` — error propagation
- `rfd` — native file save dialogs (Slint has none)

## Architecture

```
UI thread (Slint)                    tokio thread
  callbacks ──mpsc──> BackendCmd ──> query/fetch/diff
  models <──invoke_from_event_loop── CompareResult
```

- `main.rs` creates the tokio `Runtime` on a dedicated thread, spawns the
  backend command loop, builds the Slint window, wires callbacks.
- UI callbacks push `BackendCmd::{TestConnection{which,cs}, Compare{src,tgt}}`
  into an `mpsc`. Results return through `slint::invoke_from_event_loop`,
  which updates `VecModel`s and the status string.
- UI thread never blocks; no global mutable state beyond Slint model handles
  and the `mpsc::Sender`.

## Module layout

| File | Purpose |
|---|---|
| `src/main.rs` | wiring, runtime spawn, callback registration |
| `src/backend.rs` | command loop; connection test; fetch+diff orchestration |
| `src/schema.rs` | tiberius queries → `Schema` model |
| `src/render.rs` | `Schema` → canonical text per object; shared by diff + export |
| `src/diff.rs` | `similar` ops → `Vec<DiffRow>` + summaries + counts |
| `src/export.rs` | clipboard text, JSON/CSV, ALTER/CREATE-OR-ALTER generation |
| `ui/app.slint` | all UI |
| `build.rs` | `slint-build` |

## Data model

```rust
struct Schema {
    tables: Vec<TableSchema>, views: Vec<ModuleObject>,
    procedures: Vec<ModuleObject>, functions: Vec<ModuleObject>,
    triggers: Vec<ModuleObject>, udts: Vec<UdtSchema>,
}
struct TableSchema {
    schema: String, name: String,
    columns: Vec<Column>, // ordered by column_id
    constraints: Vec<Constraint>, // pk, fk, check, default
    indexes: Vec<Index>,
}
struct ModuleObject { schema: String, name: String, kind: ObjKind, definition: String }
struct DiffRow { kind: DiffKind, source: String, target: String }
enum DiffKind { Added, Removed, Modified, Unchanged }
struct CompareResult {
    groups: Vec<(ObjKind, Vec<ObjectDiff>)>, // ObjectDiff{name, rows, summary, direction-aware}
    stats: Stats, // objects compared, differences found
}
```

## Schema extraction (tiberius `simple_query`, one query per concern)

- Identity/type: `sys.objects` ⨝ `sys.schemas`; exclude `sys`, `INFORMATION_SCHEMA`.
- Table columns: `sys.columns` ⨝ `sys.types` ⨝ `sys.default_constraints` —
  name, type incl. `(max)|(p,s)` params, nullable, identity, default expr.
- Keys/indexes: `sys.indexes` ⨝ `sys.index_columns` ⨝ `sys.key_constraints`;
  FKs from `sys.foreign_keys` ⨝ `sys.foreign_key_columns`.
- Module bodies: `sys.sql_modules.definition` for V/P/FN/IF/TF/TR.
- UDTs: `sys.types` where `is_user_defined = 1`.
- Connection strings: try `Config::from_ado_string`, fall back to
  `from_jdbc_string`. SQL auth; document `TrustServerCertificate=true` for dev
  containers. Windows/NTLM auth out of scope on Linux.

## Diff approach (approved: canonical text)

- Every object renders to deterministic text:
  - Tables: `COLUMN <name> <type> [NOT] NULL [DEFAULT <expr>]` per column
    (ordered by `column_id`), then `PK`/`FK`/`CHECK`/`INDEX` lines sorted by
    name.
  - Modules (view/proc/function/trigger): `definition` verbatim, trimmed.
  - UDTs: `TYPE <name> FROM <base-type> [NOT] NULL` + length/precision params.
- `similar::TextDiff::from_lines(source_text, target_text)` → grouped
  `DiffOp`s → `DiffRow`s. Within each change hunk, delete/insert lines are
  paired positionally → `Modified`; leftovers → `Removed`/`Added`.
- Summary entries (`- col name`, `+ col name`, `~ line`) derive from the same
  hunk lines — no second diff engine.
- All objects appear in the left tree, including `(0)`-diff ones.

**Direction:** Source is the base; `Added` = present in Target, `Removed` =
missing from Target. A UI toggle reverses the pair before diffing (diff is
symmetric, so the toggle just swaps arguments). SQL export always describes
"make the first DB look like the second" in the selected direction.

## UI (`ui/app.slint`)

- Top: two `LineEdit`s (Source, Target) each with a Connect button + status
  dot (grey/green/red); Swap and Compare Schemas buttons; direction toggle.
- Body split (`HorizontalBox`):
  - Left `ListView` over flat `VecModel<TreeRow{depth, kind, name, count,
    expanded, obj_id}>`; click toggles expand → Rust rebuilds the flat model.
    No recursive components.
  - Right `ListView` over `VecModel<DiffRow>`; each row = two cells (Source /
    Target) colored by `DiffKind`:
    Added `bg #d4f7d4 / fg #1a5c1a`-ish, Removed `#f7d4d4/#8a1f1f`,
    Modified `#fdf3c4/#7a5c00`, Unchanged `white/#1a1a1a`.
- Buttons: Copy Changes · Export SQL · Copy All · Export All (+format combo
  JSON|CSV|SQL).
- Status bar: `"N objects compared, M differences found"` or error text.

## Export semantics

- **Copy object / Copy all:** git-style unified text —
  `@@ <schema>.<name> @@` headers, `-`/`+`/`~`/` ` line prefixes.
- **Export SQL** (best-effort, Source→Target in selected direction):
  - Modules: `CREATE OR ALTER <kind> <name> AS <target definition>`.
    Missing-in-target → `DROP`; missing-in-source → `CREATE` (target body).
  - Tables: `ALTER TABLE <t> ADD/DROP/ALTER COLUMN` for column diffs;
    expressible constraint/index diffs → `ALTER`/`DROP`/`CREATE`; anything not
    expressible → `-- TODO: manual migration for <constraint>` comment.
  - Scripts are review artifacts, not auto-applied migrations.
- **JSON/CSV:** serialized `DiffRow`s + object metadata (kind, schema, name).

## Error handling

`anyhow` in backend; errors land in the status bar (red) and the per-field
status dot for connect failures. Partial fetch failures name the object type
that failed; successful types still display.

## Testing / verification

- `compose.yml` (podman-compatible): `mcr.microsoft.com/mssql/server:2022-latest`,
  `ACCEPT_EULA=Y`, `MSSQL_SA_PASSWORD`, port 1433.
- `dev/seed_src.sql`, `dev/seed_tgt.sql`: two DBs with differences covering
  all six object types (added/removed/modified columns, PK/FK/index diffs,
  body diffs in procs/views, a UDT diff).
- `Makefile`: `db-up`, `db-seed`, `db-down`, `itest` (env-gated integration
  test hitting `localhost:1433`), `run`, `check`, `test`.
- Unit tests on `diff.rs` and `render.rs` with canned schema structs.
- Manual smoke: run app on Wayland, connect to both fixture DBs, exercise
  expand/collapse, copy, export.

## Deliverables

`Cargo.toml`, `build.rs`, `src/*.rs`, `ui/app.slint`, `mise.toml`,
`Makefile`, `compose.yml`, `.editorconfig`, `.gitignore`, `README.md`,
`AGENTS.md`, `docs/` (usage, dev notes), `dev/seed_*.sql`, omp agent config
(`.omlsp.toml`).

## Non-goals

- Windows/NTLM/Azure AD auth.
- Applying migrations to a database.
- Data (row) comparison.
- Full SQL Compare-grade dependency-ordered migration engine.

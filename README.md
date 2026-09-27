# schemadiff

MSSQL schema comparison tool. Connects to two SQL Server databases, extracts
their schema objects, and shows a git-style side-by-side diff — with
copy-to-clipboard and export (text / JSON / CSV / best-effort SQL).

## Features

- Compare two live MSSQL databases (source → target, or swapped)
- Object types: tables (columns, constraints, indexes), views, stored
  procedures, functions, triggers, user-defined types
- Grouped tree of objects with presence markers (source-only / target-only /
  different) and per-object change counts
- Side-by-side colored diff (add / remove / context rows)
- Copy changes for one object or all; export all diffs to a file
- Read-only: only schema metadata is queried, nothing is written

## Build

Rust 1.98+ (see `mise.toml`). Slint needs fontconfig; on systems without it
system-wide, point pkg-config at a local install before building:

```bash
export PKG_CONFIG_PATH=$HOME/.local/share/fc-devel/pc   # if fontconfig-devel is missing
cargo run
```

## Usage

1. Enter a connection string for Source and Target.
2. Click **Connect** on each — the status dot turns green on success.
3. Click **Compare Schemas**.
4. Expand groups in the tree, click an object to see its diff.
5. **Copy Changes** / **Copy All** put diffs on the clipboard;
   **Export All** writes text, JSON, CSV, or best-effort SQL to a file.

### Connection strings

ADO-style (`;`-separated). `TrustServerCertificate=true` is needed for local
dev containers with self-signed certs:

```
Server=localhost,1433;User Id=sa;Password=SchemaDiff#dev1;TrustServerCertificate=true;Database=mydb
```

Equivalent JDBC form (for reference):

```
jdbc:sqlserver://localhost:1433;databaseName=mydb;user=sa;password=SchemaDiff#dev1;trustServerCertificate=true
```

## Dev database

A podman-run SQL Server 2022 container with seeded fixture DBs
(`schemadiff_src` / `schemadiff_tgt`) is provided — see
[docs/development.md](docs/development.md). Requires podman.

```bash
make db-up      # start container (wait ~20s)
make db-seed    # create + seed both databases
make itest      # run the live integration test
make db-down    # remove container
```

`compose.yml` is included as well if you prefer `podman-compose`.

## Wayland

Slint picks a working backend automatically. If IME/focus behaves oddly under
Wayland, launch with `GTK_IM_MODULE=simple`.

## Tests

```bash
cargo test        # unit tests (30)
make itest        # live test against the seeded dev DB (needs db-up + db-seed)
```

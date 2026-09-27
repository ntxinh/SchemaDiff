# Development

## Dev database (podman)

SQL Server 2022 runs in a podman container (no podman-compose needed; a
`compose.yml` is included for convenience):

```bash
make db-up      # podman run -d --name schemadiff-mssql ... -p 1433:1433
                # wait ~20s for SQL Server to accept logins
make db-seed    # sqlcmd < dev/seed_src.sql, dev/seed_tgt.sql
make db-down    # podman rm -f schemadiff-mssql
```

SA password: `SchemaDiff#dev1` (dev only, also in `compose.yml`).

To poll readiness instead of sleeping:

```bash
until podman exec schemadiff-mssql /opt/mssql-tools18/bin/sqlcmd \
    -S localhost -U sa -P 'SchemaDiff#dev1' -C -l 5 -Q 'SELECT 1' \
    >/dev/null 2>&1; do sleep 2; done
```

### Seed layout

- `dev/seed_src.sql` → database `schemadiff_src`
- `dev/seed_tgt.sql` → database `schemadiff_tgt`

The pair is designed to exercise every object type and every diff shape:

| Object | src | tgt | expected |
|--------|-----|-----|----------|
| `Users` | name NVARCHAR(50), age, created_at DATETIME | name NVARCHAR(100), email, created_at DATETIME2 | ≥4 column changes |
| `Orders` | — | + shipped_at, + IX_Orders_Shipped | column + index adds |
| `Products` | identical | identical | no diff rows |
| `AuditLog` | absent | present | target-only |
| `ActiveUsers` (view) | 2 cols | 3 cols + WHERE | body changed |
| `GetUser` (proc) | `SELECT *` | explicit cols | body changed |
| `NewProc` | absent | present | target-only |
| `fn_FormatName` | identical | identical | proves 0-diff modules render |
| `trg_Users_Audit` (trigger) | present | absent | source-only |
| `EmailType` (UDT) | NVARCHAR(100) | NVARCHAR(200) | type widened |

Keep the seeds verbatim — fixture deltas are the test's assertions.

## Integration test

`tests/itest.rs` connects to both fixture DBs and asserts the real compare
result. It no-ops unless `SCHEMADIFF_ITEST` is set, so `cargo test` stays
hermetic:

```bash
make db-up && sleep 25 && make db-seed && make itest
```

Conn string used: `Server=localhost,1433;User Id=sa;Password=SchemaDiff#dev1;TrustServerCertificate=true;Database=<db>`.

## Adding a new object type

One pipeline, five touch points:

1. **model.rs** — add the `ObjKind` variant + payload struct; add the field to `Schema`.
2. **schema.rs** — add the `sys.*` catalog query in `fetch_schema` (label errors so a NULL definition names its object).
3. **render.rs** — add the canonical-text function for the payload.
4. **compare.rs** — add the match arm so the kind diffs like the others (`SchemaPayload` variant + grouping).
5. **ui/app.slint / main.rs** — extend the Slint group display if the kind needs distinct presentation.

No second diff path: everything funnels through render → diff_lines → compare.

CNAME := schemadiff-mssql
SA := SchemaDiff\#dev1

run:      ; cargo run
check:    ; cargo check
test:     ; cargo test
db-up:
	podman run -d --name $(CNAME) --replace -e ACCEPT_EULA=Y -e 'MSSQL_SA_PASSWORD=$(SA)' -p 1433:1433 mcr.microsoft.com/mssql/server:2022-latest
	@echo "wait ~20s for SQL Server, then: make db-seed"
db-seed:
	podman exec -i $(CNAME) /opt/mssql-tools18/bin/sqlcmd -S localhost -U sa -P '$(SA)' -C < dev/seed_src.sql
	podman exec -i $(CNAME) /opt/mssql-tools18/bin/sqlcmd -S localhost -U sa -P '$(SA)' -C < dev/seed_tgt.sql
db-down:  ; podman rm -f $(CNAME)
itest:
	SCHEMADIFF_ITEST=1 cargo test --test itest -- --nocapture

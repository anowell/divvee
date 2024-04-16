set dotenv-load
set positional-arguments

cli *args:
    cargo run -p dv -- "$@"

db-setup:
    sqlx database create
    sqlite3 --bail ${DATABASE_PATH} < core/schema.sql

db-reset: && db-setup
    sqlx database drop

db-shell:
    sqlite3 --bail --column ${DATABASE_PATH}

db-exec sql:
    sqlite3 --bail --column ${DATABASE_PATH} <<< '{{sql}}'

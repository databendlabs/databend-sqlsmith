# databend-sqlsmith

`databend-sqlsmith` is SQL fuzzing tool for generating and executing random SQL against a Databend-compatible HTTP endpoint. It is intended to find crashes, panics, connection failures, timeouts, and unexpected errors across query planning, expression evaluation, functions, DDL, and DML.

## Features

- Creates a test database, base tables, and views automatically.
- Generates random DDL, DML, and SELECT queries.
- Generates scalar expressions, aggregate functions, window functions, lambda functions, nested types, and complex query shapes.
- Loads scalar function signatures from `function_list.txt` instead of hard-coding them in Rust.
- Supports mutation fuzzing for existing SQL files or directories.
- Filters known expected errors such as invalid arguments, unsupported type combinations, and normal cast failures.
- Highlights unknown errors, connection failures, server crashes, and query timeouts.
- Attempts to reduce some failing queries to smaller reproductions.

## Usage

Start a Databend-compatible service, then run:

```bash
cargo run --bin databend-sqlsmith -- \
  --host localhost \
  --port 8000 \
  --user root \
  --pass "" \
  --db sqlsmith_test \
  --count 500 \
  --timeout 5 \
  --log-path .databend/sqlsmithlog
```

Run mutation fuzzing over existing SQL files:

```bash
cargo run --bin databend-sqlsmith -- \
  --fuzz-path path/to/sql_or_directory \
  --log-path .databend/sqlsmithlog
```

## Options

- `--host`: HTTP host, default `localhost`.
- `--port`: HTTP port, default `8000`.
- `--db`: test database name, default `sqlsmith_test`.
- `--user`: username, default `root`.
- `--pass`: password, default empty string.
- `--count`: number of random queries to generate, default `500`.
- `--timeout`: timeout per SQL statement in seconds, default `5`.
- `--fuzz-path`: SQL file or directory for mutation fuzzing. If empty, random generation mode is used.
- `--log-path`: directory for SQL logs, default `.databend/sqlsmithlog`.

## Output

Each run writes two log files under `--log-path`:

- `databend-sqlsmith.<time>.sql`: all executed SQL statements.
- `databend-sqlsmith.<time>.error.sql`: SQL statements that triggered errors or timeouts.

These files are the main inputs for debugging failures and creating regression tests.

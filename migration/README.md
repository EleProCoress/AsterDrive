# Running Migrator CLI

## Baseline rebase policy

The current migration set is rebased into `m20260512_000001_baseline_schema`.

- Fresh installs run the new baseline directly.
- Existing deployments must first run the full pre-rc.1 migration set:
  `m20260502_000001_baseline_schema`,
  `m20260508_000001_split_file_folder_owner_provenance`,
  and `m20260511_000001_add_background_task_failure_can_retry`.
- When a complete pre-rc.1 migration history is detected, AsterDrive validates key schema sentinels and rewrites only `seaql_migrations` to the new baseline stamp.
- Incomplete pre-rebase histories are rejected with an instruction to upgrade to the last pre-rc.1 build first.

Do not truncate application tables for this rebase. Only migration metadata is rewritten.

- Generate a new migration file
    ```sh
    cargo run -p migration --features cli -- generate MIGRATION_NAME
    ```
- Apply all pending migrations
    ```sh
    cargo run -p migration --features cli
    ```
    ```sh
    cargo run -p migration --features cli -- up
    ```
- Apply first 10 pending migrations
    ```sh
    cargo run -p migration --features cli -- up -n 10
    ```
- Rollback last applied migrations
    ```sh
    cargo run -p migration --features cli -- down
    ```
- Rollback last 10 applied migrations
    ```sh
    cargo run -p migration --features cli -- down -n 10
    ```
- Drop all tables from the database, then reapply all migrations
    ```sh
    cargo run -p migration --features cli -- fresh
    ```
- Rollback all applied migrations, then reapply all migrations
    ```sh
    cargo run -p migration --features cli -- refresh
    ```
- Rollback all applied migrations
    ```sh
    cargo run -p migration --features cli -- reset
    ```
- Check the status of all migrations
    ```sh
    cargo run -p migration --features cli -- status
    ```

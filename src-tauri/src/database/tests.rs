use super::*;
use rusqlite::Connection;

#[test]
fn schema_migration_sets_user_version_when_missing() {
    let db = Database::memory().expect("create memory db");
    let conn = db.conn.lock().expect("lock connection");
    assert_eq!(
        Database::get_user_version(&conn).expect("read version"),
        SCHEMA_VERSION
    );
}

#[test]
fn phase_one_migration_drops_routing_tables_and_preserves_catalog_rows() {
    let conn = Connection::open_in_memory().expect("open memory db");
    conn.execute_batch(
        "
        CREATE TABLE mcp_servers (id TEXT PRIMARY KEY, name TEXT NOT NULL, server_config TEXT NOT NULL);
        CREATE TABLE skills (id TEXT PRIMARY KEY, name TEXT NOT NULL, directory TEXT NOT NULL);
        CREATE TABLE prompts (id TEXT NOT NULL, app_type TEXT NOT NULL, name TEXT NOT NULL, content TEXT NOT NULL, PRIMARY KEY (id, app_type));
        CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT);
        CREATE TABLE providers (id TEXT NOT NULL, app_type TEXT NOT NULL, PRIMARY KEY (id, app_type));
        CREATE TABLE provider_endpoints (id INTEGER PRIMARY KEY, provider_id TEXT NOT NULL, app_type TEXT NOT NULL, url TEXT NOT NULL);
        CREATE TABLE proxy_config (app_type TEXT PRIMARY KEY);
        CREATE TABLE provider_health (provider_id TEXT PRIMARY KEY);
        CREATE TABLE proxy_request_logs (request_id TEXT PRIMARY KEY);
        CREATE TABLE model_pricing (model_id TEXT PRIMARY KEY);
        CREATE TABLE stream_check_logs (id INTEGER PRIMARY KEY);
        CREATE TABLE proxy_live_backup (app_type TEXT PRIMARY KEY);
        CREATE TABLE proxy_failover_live_snapshots (app_type TEXT NOT NULL, provider_id TEXT NOT NULL, config_json TEXT NOT NULL, generated_at TEXT NOT NULL, PRIMARY KEY (app_type, provider_id));
        CREATE TABLE usage_daily_rollups (date TEXT PRIMARY KEY);
        CREATE TABLE session_log_sync (file_path TEXT PRIMARY KEY);
        CREATE TABLE session_usage_dedup (request_id TEXT PRIMARY KEY);
        CREATE TABLE failover_queue (provider_id TEXT PRIMARY KEY);
        CREATE TABLE profiles (id TEXT PRIMARY KEY);
        INSERT INTO mcp_servers VALUES ('mcp-1', 'MCP', '{}');
        INSERT INTO skills VALUES ('skill-1', 'Skill', 'skill');
        INSERT INTO prompts VALUES ('prompt-1', 'claude', 'Prompt', 'content');
        INSERT INTO settings VALUES ('keep', 'yes');
        ",
    )
    .expect("seed pre-phase-one database");
    Database::set_user_version(&conn, SCHEMA_VERSION - 1).expect("set old schema version");
    Database::apply_schema_migrations_on_conn(&conn).expect("apply phase-one migration");

    for table in [
        "providers",
        "provider_endpoints",
        "proxy_config",
        "provider_health",
        "proxy_request_logs",
        "model_pricing",
        "stream_check_logs",
        "proxy_live_backup",
        "proxy_failover_live_snapshots",
        "proxy_failover_live_snapshots",
        "usage_daily_rollups",
        "session_log_sync",
        "session_usage_dedup",
        "failover_queue",
        "profiles",
    ] {
        assert!(!Database::table_exists(&conn, table).expect("check dropped table"));
    }
    for (table, query) in [
        (
            "mcp_servers",
            "SELECT COUNT(*) FROM mcp_servers WHERE id = 'mcp-1'",
        ),
        ("skills", "SELECT COUNT(*) FROM skills WHERE id = 'skill-1'"),
        (
            "prompts",
            "SELECT COUNT(*) FROM prompts WHERE id = 'prompt-1'",
        ),
        (
            "settings",
            "SELECT COUNT(*) FROM settings WHERE key = 'keep'",
        ),
    ] {
        let count: i64 = conn
            .query_row(query, [], |row| row.get(0))
            .expect("count preserved row");
        assert_eq!(count, 1, "{table} row should survive");
    }
}

#[test]
fn legacy_database_is_copied_only_when_desk_database_is_missing() {
    let temp = tempfile::tempdir().expect("tempdir");
    let legacy = temp.path().join("legacy/cc-switch.db");
    let desk = temp.path().join("desk/cc-switch.db");
    std::fs::create_dir_all(legacy.parent().expect("legacy parent")).expect("create legacy dir");
    std::fs::write(&legacy, b"legacy database").expect("write legacy database");

    Database::copy_legacy_database_if_needed(&desk, &legacy).expect("copy legacy database");
    assert_eq!(
        std::fs::read(&desk).expect("read copied database"),
        b"legacy database"
    );

    std::fs::write(&desk, b"desk database").expect("overwrite desk database");
    Database::copy_legacy_database_if_needed(&desk, &legacy).expect("skip existing desk database");
    assert_eq!(
        std::fs::read(&desk).expect("read existing database"),
        b"desk database"
    );
}

#[test]
#[ignore = "run with CC_SWITCH_PHASE1_MIGRATION_DB pointing at a copied legacy database"]
fn phase_one_migration_on_real_database_copy() {
    let path = std::env::var_os("CC_SWITCH_PHASE1_MIGRATION_DB")
        .expect("CC_SWITCH_PHASE1_MIGRATION_DB must point at the test copy");
    let conn = Connection::open(path).expect("open copied legacy database");
    let before = ["mcp_servers", "skills", "prompts", "settings"]
        .into_iter()
        .map(|table| {
            let count: i64 = conn
                .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                    row.get(0)
                })
                .expect("count retained table before migration");
            (table, count)
        })
        .collect::<Vec<_>>();

    Database::create_tables_on_conn(&conn).expect("create retained columns");
    Database::apply_schema_migrations_on_conn(&conn).expect("migrate copied legacy database");

    for table in [
        "provider_health",
        "provider_endpoints",
        "proxy_request_logs",
        "proxy_live_backup",
        "usage_daily_rollups",
        "session_usage_dedup",
        "session_log_sync",
        "failover_queue",
        "model_pricing",
        "stream_check_logs",
        "proxy_config",
        "circuit_breaker_config",
        "profiles",
        "custom_icons",
        "providers",
    ] {
        assert!(!Database::table_exists(&conn, table).expect("check retired table"));
    }
    for (table, expected) in before {
        let actual: i64 = conn
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .expect("count retained table after migration");
        assert_eq!(actual, expected, "{table} rows should survive");
    }
}

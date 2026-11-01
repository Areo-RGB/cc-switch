//! Retained database schema and the Phase 1 cleanup migration.

use super::{lock_conn, Database, SCHEMA_VERSION};
use crate::error::AppError;
use rusqlite::Connection;

const RETIRED_TABLES: &[&str] = &[
    "provider_health",
    "provider_endpoints",
    "proxy_request_logs",
    "proxy_live_backup",
    "proxy_failover_live_snapshots",
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
    // Temporary names used by older table rebuild migrations.
    "proxy_config_new",
    "proxy_config_v14",
    "usage_daily_rollups_v10",
    "skills_old",
];

impl Database {
    /// Create the tables owned by Desk. Routing-related tables are deliberately
    /// absent; old copies are removed by the migration below.
    pub(crate) fn create_tables(&self) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        Self::create_tables_on_conn(&conn)
    }

    pub(crate) fn create_tables_on_conn(conn: &Connection) -> Result<(), AppError> {
        conn.execute_batch(
            "\
            CREATE TABLE IF NOT EXISTS mcp_servers (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                server_config TEXT NOT NULL,
                description TEXT,
                homepage TEXT,
                docs TEXT,
                tags TEXT NOT NULL DEFAULT '[]',
                enabled_claude BOOLEAN NOT NULL DEFAULT 0,
                enabled_codex BOOLEAN NOT NULL DEFAULT 0,
                enabled_gemini BOOLEAN NOT NULL DEFAULT 0,
                enabled_grokbuild BOOLEAN NOT NULL DEFAULT 0,
                enabled_opencode BOOLEAN NOT NULL DEFAULT 0,
                enabled_hermes BOOLEAN NOT NULL DEFAULT 0,
                enabled_mcode BOOLEAN NOT NULL DEFAULT 0,
                enabled_pi BOOLEAN NOT NULL DEFAULT 0,
                enabled_agy BOOLEAN NOT NULL DEFAULT 0,
                enabled_kimi BOOLEAN NOT NULL DEFAULT 0,
                enabled_omp BOOLEAN NOT NULL DEFAULT 0,
                enabled_dsh BOOLEAN NOT NULL DEFAULT 0
            );
            CREATE TABLE IF NOT EXISTS prompts (
                id TEXT NOT NULL,
                app_type TEXT NOT NULL,
                name TEXT NOT NULL,
                content TEXT NOT NULL,
                description TEXT,
                enabled BOOLEAN NOT NULL DEFAULT 1,
                created_at INTEGER,
                updated_at INTEGER,
                PRIMARY KEY (id, app_type)
            );
            CREATE TABLE IF NOT EXISTS skills (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                directory TEXT NOT NULL,
                repo_owner TEXT,
                repo_name TEXT,
                repo_branch TEXT DEFAULT 'main',
                readme_url TEXT,
                enabled_claude BOOLEAN NOT NULL DEFAULT 0,
                enabled_codex BOOLEAN NOT NULL DEFAULT 0,
                enabled_gemini BOOLEAN NOT NULL DEFAULT 0,
                enabled_grokbuild BOOLEAN NOT NULL DEFAULT 0,
                enabled_opencode BOOLEAN NOT NULL DEFAULT 0,
                enabled_hermes BOOLEAN NOT NULL DEFAULT 0,
                enabled_mcode BOOLEAN NOT NULL DEFAULT 0,
                enabled_agy BOOLEAN NOT NULL DEFAULT 0,
                enabled_kimi BOOLEAN NOT NULL DEFAULT 0,
                enabled_dsh BOOLEAN NOT NULL DEFAULT 0,
                enabled_pi BOOLEAN NOT NULL DEFAULT 0,
                enabled_omp BOOLEAN NOT NULL DEFAULT 0,
                installed_at INTEGER NOT NULL DEFAULT 0,
                content_hash TEXT,
                updated_at INTEGER NOT NULL DEFAULT 0
            );
            CREATE TABLE IF NOT EXISTS skill_repos (
                owner TEXT NOT NULL,
                name TEXT NOT NULL,
                branch TEXT NOT NULL DEFAULT 'main',
                enabled BOOLEAN NOT NULL DEFAULT 1,
                PRIMARY KEY (owner, name)
            );
            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT
            );",
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        // Older databases may contain a retained table with an earlier column
        // set. Add only retained columns before the old routing tables go away.
        let mcp_columns = [
            ("description", "TEXT"),
            ("homepage", "TEXT"),
            ("docs", "TEXT"),
            ("tags", "TEXT NOT NULL DEFAULT '[]'"),
            ("enabled_claude", "BOOLEAN NOT NULL DEFAULT 0"),
            ("enabled_codex", "BOOLEAN NOT NULL DEFAULT 0"),
            ("enabled_gemini", "BOOLEAN NOT NULL DEFAULT 0"),
            ("enabled_grokbuild", "BOOLEAN NOT NULL DEFAULT 0"),
            ("enabled_opencode", "BOOLEAN NOT NULL DEFAULT 0"),
            ("enabled_hermes", "BOOLEAN NOT NULL DEFAULT 0"),
            ("enabled_mcode", "BOOLEAN NOT NULL DEFAULT 0"),
            ("enabled_pi", "BOOLEAN NOT NULL DEFAULT 0"),
            ("enabled_agy", "BOOLEAN NOT NULL DEFAULT 0"),
            ("enabled_kimi", "BOOLEAN NOT NULL DEFAULT 0"),
            ("enabled_omp", "BOOLEAN NOT NULL DEFAULT 0"),
            ("enabled_dsh", "BOOLEAN NOT NULL DEFAULT 0"),
        ];
        for (column, definition) in mcp_columns {
            Self::add_column_if_missing(conn, "mcp_servers", column, definition)?;
        }

        for (column, definition) in [
            ("description", "TEXT"),
            ("enabled", "BOOLEAN NOT NULL DEFAULT 1"),
            ("created_at", "INTEGER"),
            ("updated_at", "INTEGER"),
        ] {
            Self::add_column_if_missing(conn, "prompts", column, definition)?;
        }

        for (column, definition) in [
            ("description", "TEXT"),
            ("repo_owner", "TEXT"),
            ("repo_name", "TEXT"),
            ("repo_branch", "TEXT DEFAULT 'main'"),
            ("readme_url", "TEXT"),
            ("enabled_claude", "BOOLEAN NOT NULL DEFAULT 0"),
            ("enabled_codex", "BOOLEAN NOT NULL DEFAULT 0"),
            ("enabled_gemini", "BOOLEAN NOT NULL DEFAULT 0"),
            ("enabled_grokbuild", "BOOLEAN NOT NULL DEFAULT 0"),
            ("enabled_opencode", "BOOLEAN NOT NULL DEFAULT 0"),
            ("enabled_hermes", "BOOLEAN NOT NULL DEFAULT 0"),
            ("enabled_mcode", "BOOLEAN NOT NULL DEFAULT 0"),
            ("enabled_agy", "BOOLEAN NOT NULL DEFAULT 0"),
            ("enabled_kimi", "BOOLEAN NOT NULL DEFAULT 0"),
            ("enabled_dsh", "BOOLEAN NOT NULL DEFAULT 0"),
            ("enabled_pi", "BOOLEAN NOT NULL DEFAULT 0"),
            ("enabled_omp", "BOOLEAN NOT NULL DEFAULT 0"),
            ("installed_at", "INTEGER NOT NULL DEFAULT 0"),
            ("content_hash", "TEXT"),
            ("updated_at", "INTEGER NOT NULL DEFAULT 0"),
        ] {
            Self::add_column_if_missing(conn, "skills", column, definition)?;
        }
        Self::add_column_if_missing(conn, "skill_repos", "enabled", "BOOLEAN NOT NULL DEFAULT 1")?;

        Ok(())
    }

    /// Migrate any legacy database to the retained Desk schema in one atomic
    /// step. The copy made by `Database::init` is the only file this touches.
    pub(crate) fn apply_schema_migrations(&self) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        Self::apply_schema_migrations_on_conn(&conn)
    }

    pub(crate) fn apply_schema_migrations_on_conn(conn: &Connection) -> Result<(), AppError> {
        let version = Self::get_user_version(conn)?;
        if version > SCHEMA_VERSION {
            return Err(AppError::Database(format!(
                "数据库版本过新（{version}），当前应用仅支持 {SCHEMA_VERSION}，请升级应用后再尝试。"
            )));
        }

        conn.execute("SAVEPOINT desk_schema_migration", [])
            .map_err(|e| AppError::Database(format!("开启迁移 savepoint 失败: {e}")))?;

        let result = (|| {
            for table in RETIRED_TABLES {
                conn.execute(&format!("DROP TABLE IF EXISTS {table}"), [])
                    .map_err(|e| AppError::Database(format!("删除旧表 {table} 失败: {e}")))?;
            }
            Self::set_user_version(conn, SCHEMA_VERSION)
        })();

        match result {
            Ok(()) => {
                conn.execute("RELEASE desk_schema_migration", [])
                    .map_err(|e| AppError::Database(format!("提交迁移失败: {e}")))?;
                Ok(())
            }
            Err(error) => {
                let _ = conn.execute("ROLLBACK TO desk_schema_migration", []);
                let _ = conn.execute("RELEASE desk_schema_migration", []);
                Err(error)
            }
        }
    }

    pub(crate) fn get_user_version(conn: &Connection) -> Result<i32, AppError> {
        conn.query_row("PRAGMA user_version", [], |row| row.get(0))
            .map_err(|e| AppError::Database(format!("读取数据库版本失败: {e}")))
    }

    pub(crate) fn set_user_version(conn: &Connection, version: i32) -> Result<(), AppError> {
        conn.execute_batch(&format!("PRAGMA user_version = {version}"))
            .map_err(|e| AppError::Database(format!("设置数据库版本失败: {e}")))
    }

    pub(crate) fn table_exists(conn: &Connection, table: &str) -> Result<bool, AppError> {
        conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1)",
            [table],
            |row| row.get(0),
        )
        .map_err(|e| AppError::Database(format!("检查表 {table} 失败: {e}")))
    }

    fn has_column(conn: &Connection, table: &str, column: &str) -> Result<bool, AppError> {
        conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM pragma_table_info(?1) WHERE name = ?2)",
            (table, column),
            |row| row.get(0),
        )
        .map_err(|e| AppError::Database(format!("检查列 {table}.{column} 失败: {e}")))
    }

    fn add_column_if_missing(
        conn: &Connection,
        table: &str,
        column: &str,
        definition: &str,
    ) -> Result<(), AppError> {
        if !Self::table_exists(conn, table)? || Self::has_column(conn, table, column)? {
            return Ok(());
        }

        conn.execute(
            &format!("ALTER TABLE {table} ADD COLUMN {column} {definition}"),
            [],
        )
        .map_err(|e| AppError::Database(format!("添加列 {table}.{column} 失败: {e}")))?;
        Ok(())
    }
}

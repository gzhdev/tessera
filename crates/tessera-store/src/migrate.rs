//! 迁移器：编译期嵌入、按序应用、只前滚（§11.3、design.md D3/D4）。
//!
//! 启动流程（[`run`]）：
//! 1. 读 `schema_migrations` 最大版本（表不存在视为全新数据库，版本 0）
//! 2. 数据库版本高于代码支持 → [`StoreError::SchemaNewerThanCode`]，拒绝启动且不做任何修改
//! 3. 存在未应用迁移且数据库文件已存在 → 先 `VACUUM INTO` 备份为 `{db}.bak-{迁移前版本}`
//! 4. 逐条应用，每条在单个事务内完成（脚本 + 版本记录一起提交），失败整条回滚

use std::path::{Path, PathBuf};

use rusqlite::Connection;

use crate::StoreError;
use crate::pool::DbPool;

/// 单条迁移：来自 `migrations/{NNN}_{name}.sql`，`include_str!` 嵌入二进制。
#[derive(Debug, Clone)]
pub struct Migration {
    pub version: i64,
    pub name: &'static str,
    pub sql: &'static str,
}

/// 编译期嵌入的全部迁移，按版本升序。**只允许追加**，已发布的条目永不修改。
pub const MIGRATIONS: &[Migration] = &[Migration {
    version: 1,
    name: "init",
    sql: include_str!("../migrations/001_init.sql"),
}];

/// 当前代码支持的最高结构版本。
pub fn latest_version() -> i64 {
    MIGRATIONS
        .last()
        .expect("迁移列表非空（001_init.sql 永远在内）")
        .version
}

/// 一次迁移执行的结果，供测试断言与启动日志。
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Outcome {
    /// 本次实际应用的迁移版本，升序；空 = 数据库已是最新。
    pub applied: Vec<i64>,
    /// 迁移前自动备份的文件路径；全新建库（无文件可备份）或无未应用迁移时为 `None`。
    pub backup: Option<PathBuf>,
}

/// 生产路径：应用未应用的迁移。
pub fn run(path: &Path, pool: &DbPool) -> Result<Outcome, StoreError> {
    run_with(path, pool, MIGRATIONS)
}

/// 与 [`run`] 相同，但迁移列表可注入——供测试构造「中途失败的迁移」等场景；
/// 生产代码请用 [`run`]。
pub fn run_with(
    path: &Path,
    pool: &DbPool,
    migrations: &[Migration],
) -> Result<Outcome, StoreError> {
    let versions_sorted = migrations.windows(2).all(|w| w[0].version < w[1].version);
    assert!(versions_sorted, "迁移列表必须严格递增");

    let mut conn = pool.get()?;
    let current = current_version(&conn)?;
    let code_latest = migrations.last().expect("迁移列表非空").version;

    if current > code_latest {
        tracing::error!(
            db_version = current,
            code_version = code_latest,
            "拒绝启动：数据库结构版本高于代码支持（降级保护，§11.3）"
        );
        return Err(StoreError::SchemaNewerThanCode {
            db: current,
            code: code_latest,
        });
    }

    let mut outcome = Outcome::default();
    let pending: Vec<&Migration> = migrations.iter().filter(|m| m.version > current).collect();
    if pending.is_empty() {
        return Ok(outcome);
    }

    // 迁移前自动备份（§11.3）。全新数据库（尚无任何用户表）没有可备份的内容；
    // 已有数据的库——含 schema_migrations 缺失但存在其他表的异常库——都先备份。
    // 不能用「文件是否存在」判断：连接一打开 SQLite 就会创建文件。
    if !is_fresh_database(&conn)? {
        let backup = backup_path(path, current);
        if backup.exists() {
            std::fs::remove_file(&backup).map_err(|e| {
                StoreError::db("清理旧备份文件失败", format!("{}: {e}", backup.display()))
            })?;
        }
        conn.execute(
            "VACUUM INTO ?1",
            rusqlite::params![backup.to_string_lossy()],
        )?;
        tracing::info!(backup = %backup.display(), from_version = current, "迁移前已备份数据库");
        outcome.backup = Some(backup);
    }

    for m in pending {
        apply_one(&mut conn, m)?;
        tracing::info!(version = m.version, name = m.name, "已应用迁移");
        outcome.applied.push(m.version);
    }
    Ok(outcome)
}

/// 应用单条迁移：脚本与版本记录在同一事务内提交，任一步失败整条回滚（D3）。
fn apply_one(conn: &mut Connection, m: &Migration) -> Result<(), StoreError> {
    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    tx.execute_batch(m.sql)?;
    tx.execute(
        "INSERT INTO schema_migrations (version, applied_at) VALUES (?1, ?2)",
        rusqlite::params![m.version, crate::now_utc()],
    )?;
    tx.commit()?;
    Ok(())
}

/// 是否全新数据库：除 SQLite 内部表外无任何用户表（没有可备份的内容）。
fn is_fresh_database(conn: &Connection) -> Result<bool, StoreError> {
    let user_tables: i64 = conn.query_row(
        "SELECT count(*) FROM sqlite_master
         WHERE type = 'table' AND name NOT LIKE 'sqlite_%'",
        rusqlite::params![],
        |r| r.get(0),
    )?;
    Ok(user_tables == 0)
}

/// 当前数据库结构版本：`schema_migrations` 不存在（全新库）或为空时为 0。
fn current_version(conn: &Connection) -> Result<i64, StoreError> {
    let table_exists: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'schema_migrations')",
        rusqlite::params![],
        |r| r.get(0),
    )?;
    if !table_exists {
        return Ok(0);
    }
    Ok(conn.query_row(
        "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
        rusqlite::params![],
        |r| r.get(0),
    )?)
}

/// 备份文件路径：`{db路径}.bak-{迁移前版本}`（§11.3）。
fn backup_path(db: &Path, version: i64) -> PathBuf {
    let mut name = db.as_os_str().to_os_string();
    name.push(format!(".bak-{version}"));
    PathBuf::from(name)
}

/// 开发模式专用：删除数据库文件（含 WAL/SHM 旁文件）并从零应用全部迁移（design.md D4）。
///
/// **显式且刺眼**：正常启动路径 [`crate::open`] / [`run`] 永远不会删除任何文件，
/// 只有显式调用本函数才会。它存在的意义是让开发期「推倒重来」有正路可走，
/// 从而没人去改已冻结的 `001_init.sql`——一旦数据库来自真实用户，这里删掉的就是用户数据。
pub fn rebuild(path: &Path) -> Result<(DbPool, Outcome), StoreError> {
    for suffix in ["", "-wal", "-shm"] {
        let p = PathBuf::from(format!("{}{suffix}", path.display()));
        match std::fs::remove_file(&p) {
            Ok(()) => tracing::warn!(path = %p.display(), "重建数据库：已删除文件"),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => {
                return Err(StoreError::db(
                    "重建数据库：删除旧文件失败",
                    format!("{}: {e}", p.display()),
                ));
            }
        }
    }
    let pool = crate::pool::open_pool(path, crate::pool::DEFAULT_MAX_POOL_SIZE)?;
    let outcome = run(path, &pool)?;
    Ok((pool, outcome))
}

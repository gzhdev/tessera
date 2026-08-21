//! `plugins` 表访问层：登记、查询、启用停用、更新激活时间、卸载（§11.2）。

use rusqlite::OptionalExtension;

use crate::StoreError;
use crate::pool::DbPool;

/// 登记新插件所需的最少信息（计数列与时间戳由本层生成）。
pub struct NewPlugin {
    pub plugin_id: String,
    pub version: String,
    pub install_path: String,
    /// 原始清单 JSON，供审计与升级 diff。
    pub manifest_json: String,
}

/// `plugins` 表一行。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginRecord {
    pub plugin_id: String,
    pub version: String,
    pub install_path: String,
    pub manifest_json: String,
    pub enabled: bool,
    pub installed_at: String,
    pub last_activated_at: Option<String>,
    /// 存储用量计数（字节）。由 [`crate::storage`] 与数据写入同事务维护。
    pub storage_bytes: i64,
    /// 存储用量计数（键数）。
    pub storage_keys: i64,
}

fn row_to_record(r: &rusqlite::Row<'_>) -> rusqlite::Result<PluginRecord> {
    Ok(PluginRecord {
        plugin_id: r.get(0)?,
        version: r.get(1)?,
        install_path: r.get(2)?,
        manifest_json: r.get(3)?,
        enabled: r.get::<_, i64>(4)? != 0,
        installed_at: r.get(5)?,
        last_activated_at: r.get(6)?,
        storage_bytes: r.get(7)?,
        storage_keys: r.get(8)?,
    })
}

const COLS: &str = "plugin_id, version, install_path, manifest_json, enabled, installed_at, last_activated_at, storage_bytes, storage_keys";

/// 登记插件。重复登记返回 [`StoreError::InvalidArgument`]。
/// `storage_bytes` / `storage_keys` 初始为 0（design.md D5）。
pub fn register(pool: &DbPool, p: &NewPlugin) -> Result<(), StoreError> {
    let conn = pool.get()?;
    match conn.execute(
        "INSERT INTO plugins (plugin_id, version, install_path, manifest_json, enabled, installed_at, last_activated_at, storage_bytes, storage_keys)
         VALUES (?1, ?2, ?3, ?4, 1, ?5, NULL, 0, 0)",
        rusqlite::params![p.plugin_id, p.version, p.install_path, p.manifest_json, crate::now_utc()],
    ) {
        Ok(_) => Ok(()),
        Err(rusqlite::Error::SqliteFailure(e, _))
            if e.code == rusqlite::ErrorCode::ConstraintViolation =>
        {
            Err(StoreError::InvalidArgument(format!(
                "插件 {} 已登记，不可重复登记",
                p.plugin_id
            )))
        }
        Err(e) => Err(e.into()),
    }
}

/// 查询单个插件；不存在返回 `None`。
pub fn get(pool: &DbPool, plugin_id: &str) -> Result<Option<PluginRecord>, StoreError> {
    let conn = pool.get()?;
    conn.query_row(
        &format!("SELECT {COLS} FROM plugins WHERE plugin_id = ?1"),
        rusqlite::params![plugin_id],
        row_to_record,
    )
    .optional()
    .map_err(StoreError::from)
}

/// 全部已登记插件。
pub fn list(pool: &DbPool) -> Result<Vec<PluginRecord>, StoreError> {
    let conn = pool.get()?;
    let mut stmt = conn.prepare(&format!("SELECT {COLS} FROM plugins ORDER BY plugin_id"))?;
    let rows = stmt
        .query_map(rusqlite::params![], row_to_record)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

/// 启用 / 停用插件。插件不存在返回 [`StoreError::NotFound`]。
pub fn set_enabled(pool: &DbPool, plugin_id: &str, enabled: bool) -> Result<(), StoreError> {
    let conn = pool.get()?;
    let n = conn.execute(
        "UPDATE plugins SET enabled = ?2 WHERE plugin_id = ?1",
        rusqlite::params![plugin_id, enabled as i64],
    )?;
    if n == 0 {
        return Err(StoreError::NotFound(format!("插件 {plugin_id} 未登记")));
    }
    Ok(())
}

/// 更新最后激活时间为当前时刻。插件不存在返回 [`StoreError::NotFound`]。
pub fn touch_last_activated(pool: &DbPool, plugin_id: &str) -> Result<(), StoreError> {
    let conn = pool.get()?;
    let n = conn.execute(
        "UPDATE plugins SET last_activated_at = ?2 WHERE plugin_id = ?1",
        rusqlite::params![plugin_id, crate::now_utc()],
    )?;
    if n == 0 {
        return Err(StoreError::NotFound(format!("插件 {plugin_id} 未登记")));
    }
    Ok(())
}

/// 卸载插件：删除登记记录。`plugin_permissions` / `plugin_config` / `plugin_storage`
/// 的从属行由外键 `ON DELETE CASCADE` 自动清除，不留孤儿行（§11.2）。
/// 卸载后配置值是否保留是上层语义（§11.4），存储层按 schema 级联全删。
pub fn uninstall(pool: &DbPool, plugin_id: &str) -> Result<(), StoreError> {
    let conn = pool.get()?;
    let n = conn.execute(
        "DELETE FROM plugins WHERE plugin_id = ?1",
        rusqlite::params![plugin_id],
    )?;
    if n == 0 {
        return Err(StoreError::NotFound(format!("插件 {plugin_id} 未登记")));
    }
    Ok(())
}

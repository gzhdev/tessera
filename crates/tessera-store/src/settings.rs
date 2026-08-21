//! `host_settings` 表访问层：宿主自身设置（窗口布局、面板状态、快捷键覆盖，§11.2）。

use rusqlite::OptionalExtension;

use crate::StoreError;
use crate::pool::DbPool;

/// 写入（覆盖）一个设置项。
pub fn set(pool: &DbPool, key: &str, value_json: &str) -> Result<(), StoreError> {
    let conn = pool.get()?;
    conn.execute(
        "INSERT INTO host_settings (key, value_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT (key) DO UPDATE SET value_json = excluded.value_json, updated_at = excluded.updated_at",
        rusqlite::params![key, value_json, crate::now_utc()],
    )?;
    Ok(())
}

/// 读取一个设置项；不存在返回 `None`。
pub fn get(pool: &DbPool, key: &str) -> Result<Option<String>, StoreError> {
    let conn = pool.get()?;
    conn.query_row(
        "SELECT value_json FROM host_settings WHERE key = ?1",
        rusqlite::params![key],
        |r| r.get(0),
    )
    .optional()
    .map_err(StoreError::from)
}

/// 删除一个设置项。返回是否确实删除了一条。
pub fn delete(pool: &DbPool, key: &str) -> Result<bool, StoreError> {
    let conn = pool.get()?;
    let n = conn.execute(
        "DELETE FROM host_settings WHERE key = ?1",
        rusqlite::params![key],
    )?;
    Ok(n > 0)
}

/// 全部设置项，按 key 排序。
pub fn list(pool: &DbPool) -> Result<Vec<(String, String)>, StoreError> {
    let conn = pool.get()?;
    let mut stmt = conn.prepare("SELECT key, value_json FROM host_settings ORDER BY key")?;
    let rows = stmt
        .query_map(rusqlite::params![], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

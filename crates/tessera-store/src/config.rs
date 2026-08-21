//! `plugin_config` 表访问层——仅存取（§11.2 / §11.4）。
//!
//! 配置值的 schema 校验、前缀强制与 `core/config-changed` 事件属 `core-host-config`，
//! 这里只保证值能读写且与私有数据分表存放。

use rusqlite::OptionalExtension;

use crate::StoreError;
use crate::pool::DbPool;

/// 写入（覆盖）某插件一个配置项的值。插件未登记返回 [`StoreError::InvalidArgument`]。
pub fn set_value(
    pool: &DbPool,
    plugin_id: &str,
    key: &str,
    value_json: &str,
) -> Result<(), StoreError> {
    let conn = pool.get()?;
    match conn.execute(
        "INSERT INTO plugin_config (plugin_id, key, value_json, updated_at)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT (plugin_id, key) DO UPDATE SET value_json = excluded.value_json, updated_at = excluded.updated_at",
        rusqlite::params![plugin_id, key, value_json, crate::now_utc()],
    ) {
        Ok(_) => Ok(()),
        Err(rusqlite::Error::SqliteFailure(e, _))
            if e.extended_code == rusqlite::ffi::SQLITE_CONSTRAINT_FOREIGNKEY =>
        {
            Err(StoreError::InvalidArgument(format!(
                "插件 {plugin_id} 未登记，无法写配置值"
            )))
        }
        Err(e) => Err(e.into()),
    }
}

/// 读取一个配置项的值；未设置返回 `None`（schema 默认值的回退属 `core-host-config`）。
pub fn get_value(pool: &DbPool, plugin_id: &str, key: &str) -> Result<Option<String>, StoreError> {
    let conn = pool.get()?;
    conn.query_row(
        "SELECT value_json FROM plugin_config WHERE plugin_id = ?1 AND key = ?2",
        rusqlite::params![plugin_id, key],
        |r| r.get(0),
    )
    .optional()
    .map_err(StoreError::from)
}

/// 删除一个配置项的值。返回是否确实删除了一条。
pub fn delete_value(pool: &DbPool, plugin_id: &str, key: &str) -> Result<bool, StoreError> {
    let conn = pool.get()?;
    let n = conn.execute(
        "DELETE FROM plugin_config WHERE plugin_id = ?1 AND key = ?2",
        rusqlite::params![plugin_id, key],
    )?;
    Ok(n > 0)
}

/// 某插件的全部配置值，按 key 排序。
pub fn list_for_plugin(
    pool: &DbPool,
    plugin_id: &str,
) -> Result<Vec<(String, String)>, StoreError> {
    let conn = pool.get()?;
    let mut stmt = conn
        .prepare("SELECT key, value_json FROM plugin_config WHERE plugin_id = ?1 ORDER BY key")?;
    let rows = stmt
        .query_map(rusqlite::params![plugin_id], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

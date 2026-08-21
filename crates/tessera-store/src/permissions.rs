//! `plugin_permissions` 表访问层（§11.2）。

use crate::StoreError;
use crate::pool::DbPool;

/// 一条权限授予记录。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PermissionRecord {
    pub plugin_id: String,
    /// `fs.read` / `net.http` / …
    pub permission_type: String,
    /// 规范化后的 scope JSON 字符串。
    pub scope_json: String,
    pub granted: bool,
    pub decided_at: String,
}

/// 授予权限（UPSERT：重复授予幂等，刷新 `decided_at`）。
pub fn grant(
    pool: &DbPool,
    plugin_id: &str,
    permission_type: &str,
    scope_json: &str,
) -> Result<(), StoreError> {
    let conn = pool.get()?;
    conn.execute(
        "INSERT INTO plugin_permissions (plugin_id, permission_type, scope_json, granted, decided_at)
         VALUES (?1, ?2, ?3, 1, ?4)
         ON CONFLICT (plugin_id, permission_type, scope_json)
         DO UPDATE SET granted = 1, decided_at = excluded.decided_at",
        rusqlite::params![plugin_id, permission_type, scope_json, crate::now_utc()],
    )?;
    Ok(())
}

/// 撤销权限：保留记录（审计轨迹），仅翻转 `granted`。记录不存在返回 [`StoreError::NotFound`]。
pub fn revoke(
    pool: &DbPool,
    plugin_id: &str,
    permission_type: &str,
    scope_json: &str,
) -> Result<(), StoreError> {
    let conn = pool.get()?;
    let n = conn.execute(
        "UPDATE plugin_permissions SET granted = 0, decided_at = ?4
         WHERE plugin_id = ?1 AND permission_type = ?2 AND scope_json = ?3",
        rusqlite::params![plugin_id, permission_type, scope_json, crate::now_utc()],
    )?;
    if n == 0 {
        return Err(StoreError::NotFound(format!(
            "插件 {plugin_id} 无 {permission_type} 授权记录"
        )));
    }
    Ok(())
}

/// 某插件的全部授予记录。
pub fn list_for_plugin(
    pool: &DbPool,
    plugin_id: &str,
) -> Result<Vec<PermissionRecord>, StoreError> {
    let conn = pool.get()?;
    let mut stmt = conn.prepare(
        "SELECT plugin_id, permission_type, scope_json, granted, decided_at
         FROM plugin_permissions WHERE plugin_id = ?1 ORDER BY permission_type, scope_json",
    )?;
    let rows = stmt
        .query_map(rusqlite::params![plugin_id], |r| {
            Ok(PermissionRecord {
                plugin_id: r.get(0)?,
                permission_type: r.get(1)?,
                scope_json: r.get(2)?,
                granted: r.get::<_, i64>(3)? != 0,
                decided_at: r.get(4)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

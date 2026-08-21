//! `virtual_dirs` 表访问层，含内置别名删除保护（§7.2 / §11.2、design.md D7）。

use rusqlite::OptionalExtension;

use crate::StoreError;
use crate::pool::DbPool;

/// 一条别名映射。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VirtualDir {
    pub alias: String,
    /// 未映射时为空串——别名已注册但未映射是合法状态（design.md D7），
    /// 与 `E_PERM_UNKNOWN_VDIR`（别名根本不存在）是两类失败。
    pub real_path: String,
    pub writable: bool,
    pub builtin: bool,
}

/// 新增或更新用户自建别名（`builtin` 恒为 0，不提供写入内置别名的入口）。
pub fn upsert(
    pool: &DbPool,
    alias: &str,
    real_path: &str,
    writable: bool,
) -> Result<(), StoreError> {
    let conn = pool.get()?;
    conn.execute(
        "INSERT INTO virtual_dirs (alias, real_path, writable, builtin)
         VALUES (?1, ?2, ?3, 0)
         ON CONFLICT (alias) DO UPDATE SET real_path = excluded.real_path, writable = excluded.writable",
        rusqlite::params![alias, real_path, writable as i64],
    )?;
    Ok(())
}

/// 更新某别名的真实路径（workspace 等待用户选定、壳层启动时填充内置别名模板）。
/// 别名不存在返回 [`StoreError::NotFound`]。
pub fn set_real_path(pool: &DbPool, alias: &str, real_path: &str) -> Result<(), StoreError> {
    let conn = pool.get()?;
    let n = conn.execute(
        "UPDATE virtual_dirs SET real_path = ?2 WHERE alias = ?1",
        rusqlite::params![alias, real_path],
    )?;
    if n == 0 {
        return Err(StoreError::NotFound(format!("别名 {alias} 不存在")));
    }
    Ok(())
}

/// 查询单个别名；不存在返回 `None`。
pub fn get(pool: &DbPool, alias: &str) -> Result<Option<VirtualDir>, StoreError> {
    let conn = pool.get()?;
    conn.query_row(
        "SELECT alias, real_path, writable, builtin FROM virtual_dirs WHERE alias = ?1",
        rusqlite::params![alias],
        |r| {
            Ok(VirtualDir {
                alias: r.get(0)?,
                real_path: r.get(1)?,
                writable: r.get::<_, i64>(2)? != 0,
                builtin: r.get::<_, i64>(3)? != 0,
            })
        },
    )
    .optional()
    .map_err(StoreError::from)
}

/// 全部别名。
pub fn list(pool: &DbPool) -> Result<Vec<VirtualDir>, StoreError> {
    let conn = pool.get()?;
    let mut stmt = conn
        .prepare("SELECT alias, real_path, writable, builtin FROM virtual_dirs ORDER BY alias")?;
    let rows = stmt
        .query_map(rusqlite::params![], |r| {
            Ok(VirtualDir {
                alias: r.get(0)?,
                real_path: r.get(1)?,
                writable: r.get::<_, i64>(2)? != 0,
                builtin: r.get::<_, i64>(3)? != 0,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

/// 删除别名。内置别名（`builtin = 1`）被拒绝（§11.2 数据约束，D7）；
/// 用户自建别名可自由删除。
pub fn delete(pool: &DbPool, alias: &str) -> Result<(), StoreError> {
    match get(pool, alias)? {
        None => Err(StoreError::NotFound(format!("别名 {alias} 不存在"))),
        Some(dir) if dir.builtin => Err(StoreError::Rejected(format!("内置别名 {alias} 不可删除"))),
        Some(_) => {
            let conn = pool.get()?;
            conn.execute(
                "DELETE FROM virtual_dirs WHERE alias = ?1",
                rusqlite::params![alias],
            )?;
            Ok(())
        }
    }
}

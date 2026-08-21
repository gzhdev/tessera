//! `plugin_storage` 访问层：三重配额、`__` 保留命名空间、`__picked/*` 交付条目
//! （§11.4 / §11.5、design.md D5/D6）。
//!
//! 接口刻意分成两组（D6）：
//! - **插件侧** [`set_plugin_key`]：拒绝 `__` 前缀，做三重配额检查，
//!   数据写入与计数器更新在同一事务内（D5）
//! - **宿主侧** [`set_reserved_key`]：只接受 `__` 前缀，不更新配额计数器
//!
//! 两个函数让「宿主误调插件接口 / 插件获得保留写入」在编译期就不可能发生。

use rusqlite::OptionalExtension;

use crate::StoreError;
use crate::pool::DbPool;

/// 每插件总字节数上限：16 MB（§11.5）。
pub const QUOTA_TOTAL_BYTES: i64 = 16 * 1024 * 1024;
/// 每插件键数上限：10000（§11.5）。
pub const QUOTA_MAX_KEYS: i64 = 10_000;
/// 单值字节数上限：1 MB（§11.5）。
pub const QUOTA_VALUE_BYTES: usize = 1024 * 1024;
/// 宿主保留命名空间前缀（§11.4）。
pub const RESERVED_PREFIX: &str = "__";
/// 文件交付条目前缀（§9.2 / §11.5）。
pub const PICKED_PREFIX: &str = "__picked/";
/// 单次文件读取上限，同时约束单条交付内容：64 MB（§7.2 / §11.5）。
pub const MAX_FILE_READ_BYTES: usize = 64 * 1024 * 1024;
/// 同时存活的交付条目上限，超出淘汰最旧（§11.5）。
pub const MAX_PICKED_ENTRIES: i64 = 4;

/// 某插件的存储用量（不含保留命名空间）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StorageUsage {
    pub bytes: i64,
    pub keys: i64,
}

/// 插件侧写入一个键。
///
/// - 键以 `__` 开头 → [`StoreError::InvalidArgument`]（保留命名空间，§11.4；
///   WASM 边界调用方映射为 `invalid-argument`）
/// - 单值 > 1 MB、写入后总量 > 16 MB 或键数 > 10000 → [`StoreError::quota`]，
///   且已有数据不受影响（事务未提交即返回）
/// - 数据 UPSERT 与 `plugins.storage_bytes` / `storage_keys` 计数更新在同一事务内（D5）；
///   覆盖写入按差值计入
pub fn set_plugin_key(
    pool: &DbPool,
    plugin_id: &str,
    key: &str,
    value: &[u8],
) -> Result<(), StoreError> {
    if key.starts_with(RESERVED_PREFIX) {
        return Err(StoreError::InvalidArgument(format!(
            "键 `{key}` 以 `__` 开头，该前缀保留给宿主"
        )));
    }
    if value.len() > QUOTA_VALUE_BYTES {
        return Err(StoreError::quota(
            "单个值超过存储上限（1 MB）",
            format!("plugin={plugin_id} key={key} size={}", value.len()),
        ));
    }

    let mut conn = pool.get()?;
    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;

    let (cur_bytes, cur_keys) = tx
        .query_row(
            "SELECT storage_bytes, storage_keys FROM plugins WHERE plugin_id = ?1",
            rusqlite::params![plugin_id],
            |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?)),
        )
        .optional()?
        .ok_or_else(|| StoreError::NotFound(format!("插件 {plugin_id} 未登记")))?;

    let old_len: Option<i64> = tx
        .query_row(
            "SELECT length(value) FROM plugin_storage WHERE plugin_id = ?1 AND key = ?2",
            rusqlite::params![plugin_id, key],
            |r| r.get(0),
        )
        .optional()?;

    let new_bytes = cur_bytes - old_len.unwrap_or(0) + value.len() as i64;
    let new_keys = cur_keys + u64::from(old_len.is_none()) as i64;

    if new_bytes > QUOTA_TOTAL_BYTES {
        return Err(StoreError::quota(
            "插件存储空间已满（上限 16 MB）",
            format!("plugin={plugin_id} key={key} projected_bytes={new_bytes}"),
        ));
    }
    if new_keys > QUOTA_MAX_KEYS {
        return Err(StoreError::quota(
            "插件存储键数已达上限（10000）",
            format!("plugin={plugin_id} key={key} projected_keys={new_keys}"),
        ));
    }

    tx.execute(
        "INSERT INTO plugin_storage (plugin_id, key, value, updated_at) VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT (plugin_id, key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
        rusqlite::params![plugin_id, key, value, crate::now_utc()],
    )?;
    tx.execute(
        "UPDATE plugins SET storage_bytes = ?1, storage_keys = ?2 WHERE plugin_id = ?3",
        rusqlite::params![new_bytes, new_keys, plugin_id],
    )?;
    tx.commit()?;
    Ok(())
}

/// 读取一个键，返回原始字节。插件可以读到自己命名空间下的保留键
/// （含宿主写入的 `__picked/*`，spec storage-quota Scenario「插件可读保留键」）。
pub fn get_plugin_key(
    pool: &DbPool,
    plugin_id: &str,
    key: &str,
) -> Result<Option<Vec<u8>>, StoreError> {
    let conn = pool.get()?;
    conn.query_row(
        "SELECT value FROM plugin_storage WHERE plugin_id = ?1 AND key = ?2",
        rusqlite::params![plugin_id, key],
        |r| r.get(0),
    )
    .optional()
    .map_err(StoreError::from)
}

/// 插件侧删除一个键（`__` 前缀同写入一样被拒）。计数器在同一事务内扣减。
/// 返回是否确实删除了一条。
pub fn delete_plugin_key(pool: &DbPool, plugin_id: &str, key: &str) -> Result<bool, StoreError> {
    if key.starts_with(RESERVED_PREFIX) {
        return Err(StoreError::InvalidArgument(format!(
            "键 `{key}` 以 `__` 开头，该前缀保留给宿主"
        )));
    }
    let mut conn = pool.get()?;
    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    let old_len: Option<i64> = tx
        .query_row(
            "SELECT length(value) FROM plugin_storage WHERE plugin_id = ?1 AND key = ?2",
            rusqlite::params![plugin_id, key],
            |r| r.get(0),
        )
        .optional()?;
    let Some(len) = old_len else {
        return Ok(false);
    };
    tx.execute(
        "DELETE FROM plugin_storage WHERE plugin_id = ?1 AND key = ?2",
        rusqlite::params![plugin_id, key],
    )?;
    tx.execute(
        "UPDATE plugins SET storage_bytes = storage_bytes - ?1, storage_keys = storage_keys - 1
         WHERE plugin_id = ?2",
        rusqlite::params![len, plugin_id],
    )?;
    tx.commit()?;
    Ok(true)
}

/// 宿主侧写入保留键（D6）。只接受 `__` 前缀；**不更新配额计数器**——
/// 保留命名空间的数据不计入插件配额（§11.5），否则一次大文件交付就撑爆插件自身存储。
///
/// `__picked/*` 交付条目的独立上限（§11.5）：
/// - 单条 ≤ [`MAX_FILE_READ_BYTES`]（64 MB），超限拒绝
/// - 新增条目使存活数 > [`MAX_PICKED_ENTRIES`]（4）时淘汰最旧（按写入顺序）；
///   覆盖已有条目不触发淘汰
///
/// 其余 `__` 前缀保留键（将来的宿主投递通道）直接 UPSERT，暂无额外规则。
pub fn set_reserved_key(
    pool: &DbPool,
    plugin_id: &str,
    key: &str,
    value: &[u8],
) -> Result<(), StoreError> {
    if !key.starts_with(RESERVED_PREFIX) {
        return Err(StoreError::InvalidArgument(format!(
            "宿主侧写入接口只接受 `__` 前缀的保留键，收到 `{key}`"
        )));
    }
    if key.starts_with(PICKED_PREFIX) && value.len() > MAX_FILE_READ_BYTES {
        return Err(StoreError::InvalidArgument(format!(
            "单条交付内容超过单次文件读取上限（64 MB），实际 {} 字节",
            value.len()
        )));
    }

    let mut conn = pool.get()?;
    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;

    let exists_row: bool = tx
        .query_row(
            "SELECT 1 FROM plugins WHERE plugin_id = ?1",
            rusqlite::params![plugin_id],
            |_| Ok(true),
        )
        .optional()?
        .unwrap_or(false);
    if !exists_row {
        return Err(StoreError::NotFound(format!("插件 {plugin_id} 未登记")));
    }

    if key.starts_with(PICKED_PREFIX) {
        let key_exists: bool = tx
            .query_row(
                "SELECT 1 FROM plugin_storage WHERE plugin_id = ?1 AND key = ?2",
                rusqlite::params![plugin_id, key],
                |_| Ok(true),
            )
            .optional()?
            .unwrap_or(false);
        if !key_exists {
            let alive: i64 = tx.query_row(
                "SELECT count(*) FROM plugin_storage WHERE plugin_id = ?1 AND key GLOB '__picked/*'",
                rusqlite::params![plugin_id],
                |r| r.get(0),
            )?;
            if alive >= MAX_PICKED_ENTRIES {
                // 按 rowid（写入顺序）淘汰最旧的，给新条目腾位
                tx.execute(
                    "DELETE FROM plugin_storage WHERE rowid IN (
                         SELECT rowid FROM plugin_storage
                         WHERE plugin_id = ?1 AND key GLOB '__picked/*'
                         ORDER BY rowid LIMIT ?2
                     )",
                    rusqlite::params![plugin_id, alive + 1 - MAX_PICKED_ENTRIES],
                )?;
            }
        }
    }

    tx.execute(
        "INSERT INTO plugin_storage (plugin_id, key, value, updated_at) VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT (plugin_id, key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
        rusqlite::params![plugin_id, key, value, crate::now_utc()],
    )?;
    tx.commit()?;
    Ok(())
}

/// 插件停用时清除其全部 `__picked/*` 交付条目（§11.5）。
/// 普通键值数据保留；交付条目不按时间过期——只要插件未停用且未被淘汰，
/// 无论多久之后读取都应成功。返回清除的条数。
pub fn clear_picked_entries(pool: &DbPool, plugin_id: &str) -> Result<u64, StoreError> {
    let conn = pool.get()?;
    let n = conn.execute(
        "DELETE FROM plugin_storage WHERE plugin_id = ?1 AND key GLOB '__picked/*'",
        rusqlite::params![plugin_id],
    )?;
    Ok(n as u64)
}

/// 查询某插件的存储用量。计数列只被插件侧写入维护（保留键从不计入，D5/D6），
/// 因此结果天然不含保留命名空间的占用（spec storage-quota「用量可见」）。
pub fn usage(pool: &DbPool, plugin_id: &str) -> Result<StorageUsage, StoreError> {
    let conn = pool.get()?;
    let usage = conn
        .query_row(
            "SELECT storage_bytes, storage_keys FROM plugins WHERE plugin_id = ?1",
            rusqlite::params![plugin_id],
            |r| {
                Ok(StorageUsage {
                    bytes: r.get(0)?,
                    keys: r.get(1)?,
                })
            },
        )
        .optional()?
        .ok_or_else(|| StoreError::NotFound(format!("插件 {plugin_id} 未登记")))?;
    Ok(usage)
}

/// 以实际数据重算计数器（排除保留命名空间），返回重算后的用量。
/// 修复外部直接改库造成的计数漂移（D5 缓解措施）。
pub fn recalc_usage(pool: &DbPool, plugin_id: &str) -> Result<StorageUsage, StoreError> {
    let mut conn = pool.get()?;
    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    let usage = recalc_on(&tx, plugin_id)?;
    tx.commit()?;
    Ok(usage)
}

/// 用户手动清理某插件的私有存储：删除其全部普通键（保留命名空间归宿主管辖，
/// 不随用户清理动作消失），并顺带重算计数器（spec storage-quota「手动清理释放配额」）。
/// 删除与重算在同一事务内——中途崩溃不会留下「数据已删、计数虚高」的配额虚耗（D5）。
pub fn clear_plugin_storage(pool: &DbPool, plugin_id: &str) -> Result<(), StoreError> {
    let mut conn = pool.get()?;
    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    tx.execute(
        "DELETE FROM plugin_storage WHERE plugin_id = ?1 AND key NOT GLOB '__*'",
        rusqlite::params![plugin_id],
    )?;
    recalc_on(&tx, plugin_id)?;
    tx.commit()?;
    Ok(())
}

fn recalc_on(conn: &rusqlite::Connection, plugin_id: &str) -> Result<StorageUsage, StoreError> {
    let exists: bool = conn
        .query_row(
            "SELECT 1 FROM plugins WHERE plugin_id = ?1",
            rusqlite::params![plugin_id],
            |_| Ok(true),
        )
        .optional()?
        .unwrap_or(false);
    if !exists {
        return Err(StoreError::NotFound(format!("插件 {plugin_id} 未登记")));
    }
    conn.execute(
        "UPDATE plugins SET
             storage_bytes = (SELECT COALESCE(SUM(length(value)), 0) FROM plugin_storage
                              WHERE plugin_id = ?1 AND key NOT GLOB '__*'),
             storage_keys  = (SELECT COUNT(*) FROM plugin_storage
                              WHERE plugin_id = ?1 AND key NOT GLOB '__*')
         WHERE plugin_id = ?1",
        rusqlite::params![plugin_id],
    )?;
    let u = conn.query_row(
        "SELECT storage_bytes, storage_keys FROM plugins WHERE plugin_id = ?1",
        rusqlite::params![plugin_id],
        |r| {
            Ok(StorageUsage {
                bytes: r.get(0)?,
                keys: r.get(1)?,
            })
        },
    )?;
    Ok(u)
}

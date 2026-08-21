//! 连接池与连接级 PRAGMA（§11.2 连接管理、design.md D2）。

use std::path::Path;

use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;

use crate::StoreError;

/// 连接池类型。多个模块的访问函数都以 `&DbPool` 为第一参数。
pub type DbPool = Pool<SqliteConnectionManager>;

/// 默认池大小。保守初值（design.md Risks）；真实压力要等 `core-plugin-lifecycle`
/// 之后才能观察，届时再暴露为配置。
pub const DEFAULT_MAX_POOL_SIZE: u32 = 8;

/// 打开连接池。每个连接建立时统一执行：
///
/// - `journal_mode = WAL`——并发读不阻塞写（数据库级持久属性，重复执行无害）
/// - `foreign_keys = ON`——连接级属性，必须每个连接都设，级联删除才生效
/// - `synchronous = NORMAL`——WAL 下的推荐档位
pub fn open_pool(path: &Path, max_size: u32) -> Result<DbPool, StoreError> {
    let manager = SqliteConnectionManager::file(path).with_init(|conn| {
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA foreign_keys = ON;
             PRAGMA synchronous = NORMAL;",
        )
    });
    Ok(Pool::builder().max_size(max_size).build(manager)?)
}

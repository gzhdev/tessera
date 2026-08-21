//! 集成测试共用助手（不同测试二进制各取所需，允许部分未用）。
#![allow(dead_code)]

use std::path::{Path, PathBuf};

use tempfile::TempDir;
use tessera_store::pool::DbPool;

/// 建临时目录与库文件路径（文件尚不存在）。
pub fn temp_db() -> (TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("创建临时目录");
    let path = dir.path().join("tessera.db");
    (dir, path)
}

/// 生产入口打开（建池 + 迁移）。
pub fn opened(path: &Path) -> DbPool {
    tessera_store::open(path).expect("打开数据库")
}

/// 登记一个最小插件。
pub fn register_plugin(pool: &DbPool, id: &str) {
    tessera_store::plugins::register(
        pool,
        &tessera_store::plugins::NewPlugin {
            plugin_id: id.to_string(),
            version: "0.1.0".to_string(),
            install_path: format!("/plugins/{id}"),
            manifest_json: "{}".to_string(),
        },
    )
    .expect("登记插件");
}

/// 直连执行标量 SQL（断言用，绕过访问层）。
pub fn scalar_i64(pool: &DbPool, sql: &str, plugin_id: &str) -> i64 {
    pool.get()
        .expect("取连接")
        .query_row(sql, rusqlite::params![plugin_id], |r| r.get(0))
        .expect("标量查询")
}

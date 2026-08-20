//! `E_HOST_*`：宿主内部错误（视为宿主 bug，非插件问题）。
//!
//! 构造请走 [`crate::TesseraError::host`] 或本模块的便捷函数，
//! 保证 `is_host_bug()` 标记自动打上。

use crate::TesseraError;

crate::codes::reason_enum!(HostCode {
    Panic => "PANIC",
    Db => "DB",
});

/// 宿主代码 panic（`E_HOST_PANIC`）。
pub fn panic(user_message: impl Into<String>, developer_detail: impl Into<String>) -> TesseraError {
    TesseraError::host(HostCode::Panic, user_message, developer_detail)
}

/// 数据库访问失败（`E_HOST_DB`）——由 `tessera-store` 层使用。
pub fn db(user_message: impl Into<String>, developer_detail: impl Into<String>) -> TesseraError {
    TesseraError::host(HostCode::Db, user_message, developer_detail)
}

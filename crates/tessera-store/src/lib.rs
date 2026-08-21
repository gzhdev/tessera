//! SQLite 存储层：连接池、迁移与六张表的 typed 访问（设计书 §11）。
//!
//! - 生产入口 [`open`]：建池 + 应用未应用的迁移。数据库结构版本高于代码支持时
//!   返回 [`StoreError::SchemaNewerThanCode`] 并拒绝启动（降级保护，§11.3）。
//! - 开发模式的「删库重建」走 [`migrate::rebuild`]——与生产路径分立、显式且刺眼
//!   （design.md D4），[`open`] 永远不会删除任何文件。
//! - 各表 SQL 封装在同名模块内，上层不写裸查询。
//!
//! 错误约定：配额超限携带 `E_QUOTA_STORAGE`，数据库故障携带 `E_HOST_DB`；
//! 参数非法（如插件写 `__` 保留前缀）由 WASM 边界的调用方映射为
//! `invalid-argument`（§5.2 / §11.4），存储层用 [`StoreError::InvalidArgument`] 表达。

pub mod config;
pub mod migrate;
pub mod permissions;
pub mod plugins;
pub mod pool;
pub mod settings;
pub mod storage;
pub mod vdirs;

use std::path::Path;

use tessera_error::TesseraError;

/// 打开数据库（生产入口）：建立连接池并应用未应用的迁移。
pub fn open(path: &Path) -> Result<pool::DbPool, StoreError> {
    let pool = pool::open_pool(path, pool::DEFAULT_MAX_POOL_SIZE)?;
    migrate::run(path, &pool)?;
    Ok(pool)
}

/// 存储层错误。配额与数据库故障携带 [`TesseraError`]（三段式），
/// 其余变体由调用方按类别处置。
#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    /// 参数非法——例如插件写入 `__` 保留前缀的键（§11.4）。
    #[error("参数非法：{0}")]
    InvalidArgument(String),

    /// 操作被存储层拒绝——例如删除内置虚拟目录别名。
    #[error("操作被拒绝：{0}")]
    Rejected(String),

    /// 目标记录不存在。
    #[error("记录不存在：{0}")]
    NotFound(String),

    /// 配额超限（错误码 `E_QUOTA_STORAGE`，§11.5）。
    #[error("{0}")]
    Quota(TesseraError),

    /// 数据库访问失败（错误码 `E_HOST_DB`）。
    #[error("{0}")]
    Host(TesseraError),

    /// 数据库结构版本高于当前代码支持的最高版本——拒绝启动（§11.3 降级保护）。
    /// 返回此错误前数据库未被做过任何修改。
    #[error(
        "数据库结构版本（v{db}）高于当前应用支持的最高版本（v{code}）。\
         请先升级应用再启动，以免旧代码破坏新结构；数据库内容未被修改。"
    )]
    SchemaNewerThanCode { db: i64, code: i64 },
}

impl StoreError {
    /// 是否配额超限（程序判定用，不解析文案）。
    pub fn is_quota(&self) -> bool {
        matches!(self, StoreError::Quota(_))
    }

    /// 构造配额超限错误（`E_QUOTA_STORAGE`）。
    pub fn quota(user_message: impl Into<String>, developer_detail: impl Into<String>) -> Self {
        StoreError::Quota(TesseraError::with_detail(
            tessera_error::ErrorCode::Quota(tessera_error::codes::QuotaCode::Storage),
            user_message,
            developer_detail,
        ))
    }

    /// 构造数据库故障错误（`E_HOST_DB`）。
    fn db(user_message: impl Into<String>, developer_detail: impl Into<String>) -> Self {
        StoreError::Host(tessera_error::codes::host::db(
            user_message,
            developer_detail,
        ))
    }
}

impl From<rusqlite::Error> for StoreError {
    fn from(e: rusqlite::Error) -> Self {
        StoreError::db("数据库访问失败", format!("rusqlite: {e}"))
    }
}

impl From<r2d2::Error> for StoreError {
    fn from(e: r2d2::Error) -> Self {
        StoreError::db("获取数据库连接失败", format!("r2d2: {e}"))
    }
}

/// 当前时刻的 UTC 时间字符串，秒精度（`2026-08-21T12:34:56Z`）。
/// 时间戳列统一用此格式（§11.2 各表 `*_at` 列）。
pub(crate) fn now_utc() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("系统时钟早于 Unix 纪元")
        .as_secs();
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    let (year, month, day) = civil_from_days(days);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        rem / 3600,
        rem / 60 % 60,
        rem % 60
    )
}

/// Unix 天数 → 公历年月日（Howard Hinnant 的 civil_from_days 算法）。
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let year = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if month <= 2 { year + 1 } else { year }, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 时间戳格式与已知锚点核对（1970-01-01 与一个闰年后日期）。
    #[test]
    fn now_utc_format_and_civil_anchors() {
        assert_eq!(now_utc().len(), 20);
        assert!(now_utc().ends_with('Z'));

        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(19_723), (2024, 1, 1)); // 闰年前
        assert_eq!(civil_from_days(20_686), (2026, 8, 21));
        assert_eq!(civil_from_days(11_017), (2000, 3, 1)); // 闰日次日
    }
}

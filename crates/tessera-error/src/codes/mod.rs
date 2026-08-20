//! 错误码域成员，按域分模块组织（设计书 §12.1 的 12 个域）。
//!
//! 每个模块定义该域的原因枚举（变体名 ↔ `E_{DOMAIN}_{REASON}` 的 REASON 段）。
//! 新增错误码必须落在已有域内；新增域是显式契约变更（先改 `Domain` 与本目录）。

/// 为一个域生成原因枚举：变体列表、`ALL`、`as_str`、`parse`。
macro_rules! reason_enum {
    ($name:ident { $($variant:ident => $reason:literal),+ $(,)? }) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum $name {
            $($variant),+
        }

        impl $name {
            pub const ALL: &'static [Self] = &[$(Self::$variant),+];

            pub const fn as_str(&self) -> &'static str {
                match self {
                    $(Self::$variant => $reason),+
                }
            }

            pub fn parse(s: &str) -> Option<Self> {
                Self::ALL.iter().copied().find(|r| r.as_str() == s)
            }
        }
    };
}

pub(crate) use reason_enum;

pub mod activate;
pub mod call;
pub mod compat;
pub mod conflict;
pub mod deps;
pub mod host;
pub mod load;
pub mod manifest;
pub mod perm;
pub mod quota;
pub mod timeout;
pub mod trap;

pub use activate::ActivateCode;
pub use call::CallCode;
pub use compat::CompatCode;
pub use conflict::ConflictCode;
pub use deps::DepsCode;
pub use host::HostCode;
pub use load::LoadCode;
pub use manifest::ManifestCode;
pub use perm::PermCode;
pub use quota::QuotaCode;
pub use timeout::TimeoutCode;
pub use trap::TrapCode;

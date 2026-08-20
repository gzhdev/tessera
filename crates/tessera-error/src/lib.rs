//! Tessera 统一错误模型（设计书 §12.1，`specs/error-model`）。
//!
//! - [`ErrorCode`]：全局唯一错误码枚举，字符串形式 `E_{DOMAIN}_{REASON}`，
//!   12 个域为封闭集合（新增域 = 一次显式的契约变更），域成员按模块组织在 [`codes`]。
//! - [`TesseraError`]：三段式结构——错误码（程序判定用）、面向用户的一句话（进 UI）、
//!   可选的开发者细节（只进日志，不进 UI）。
//! - `E_HOST_*` 域有专用构造入口 [`TesseraError::host`]（及 [`codes::host`] 的便捷函数），
//!   经该入口构造的错误 [`TesseraError::is_host_bug`] 恒为真——宿主缺陷不会被误报为插件问题。

pub mod codes;

use std::fmt;
use std::str::FromStr;

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

pub use codes::host::HostCode;

/// 错误域（封闭集合）。新增域必须作为显式契约变更处理（spec error-model Requirement 1）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Domain {
    Manifest,
    Compat,
    Deps,
    Conflict,
    Load,
    Activate,
    Perm,
    Quota,
    Timeout,
    Trap,
    Call,
    Host,
}

impl Domain {
    pub const ALL: &'static [Domain] = &[
        Domain::Manifest,
        Domain::Compat,
        Domain::Deps,
        Domain::Conflict,
        Domain::Load,
        Domain::Activate,
        Domain::Perm,
        Domain::Quota,
        Domain::Timeout,
        Domain::Trap,
        Domain::Call,
        Domain::Host,
    ];

    pub const fn as_str(&self) -> &'static str {
        match self {
            Domain::Manifest => "MANIFEST",
            Domain::Compat => "COMPAT",
            Domain::Deps => "DEPS",
            Domain::Conflict => "CONFLICT",
            Domain::Load => "LOAD",
            Domain::Activate => "ACTIVATE",
            Domain::Perm => "PERM",
            Domain::Quota => "QUOTA",
            Domain::Timeout => "TIMEOUT",
            Domain::Trap => "TRAP",
            Domain::Call => "CALL",
            Domain::Host => "HOST",
        }
    }

    fn parse(s: &str) -> Option<Domain> {
        Domain::ALL.iter().copied().find(|d| d.as_str() == s)
    }
}

impl fmt::Display for Domain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// 全局错误码。单一类型、可与常量做相等比较；序列化为 `E_{DOMAIN}_{REASON}` 字符串跨界传递。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorCode {
    Manifest(codes::ManifestCode),
    Compat(codes::CompatCode),
    Deps(codes::DepsCode),
    Conflict(codes::ConflictCode),
    Load(codes::LoadCode),
    Activate(codes::ActivateCode),
    Perm(codes::PermCode),
    Quota(codes::QuotaCode),
    Timeout(codes::TimeoutCode),
    Trap(codes::TrapCode),
    Call(codes::CallCode),
    Host(codes::HostCode),
}

impl ErrorCode {
    /// 全部已知错误码（测试与全量校验用）。
    pub fn all() -> Vec<ErrorCode> {
        use codes::*;
        let mut all = Vec::new();
        for r in ManifestCode::ALL {
            all.push(ErrorCode::Manifest(*r));
        }
        for r in CompatCode::ALL {
            all.push(ErrorCode::Compat(*r));
        }
        for r in DepsCode::ALL {
            all.push(ErrorCode::Deps(*r));
        }
        for r in ConflictCode::ALL {
            all.push(ErrorCode::Conflict(*r));
        }
        for r in LoadCode::ALL {
            all.push(ErrorCode::Load(*r));
        }
        for r in ActivateCode::ALL {
            all.push(ErrorCode::Activate(*r));
        }
        for r in PermCode::ALL {
            all.push(ErrorCode::Perm(*r));
        }
        for r in QuotaCode::ALL {
            all.push(ErrorCode::Quota(*r));
        }
        for r in TimeoutCode::ALL {
            all.push(ErrorCode::Timeout(*r));
        }
        for r in TrapCode::ALL {
            all.push(ErrorCode::Trap(*r));
        }
        for r in CallCode::ALL {
            all.push(ErrorCode::Call(*r));
        }
        for r in HostCode::ALL {
            all.push(ErrorCode::Host(*r));
        }
        all
    }

    /// 该错误码所属的域。
    pub fn domain(&self) -> Domain {
        match self {
            ErrorCode::Manifest(_) => Domain::Manifest,
            ErrorCode::Compat(_) => Domain::Compat,
            ErrorCode::Deps(_) => Domain::Deps,
            ErrorCode::Conflict(_) => Domain::Conflict,
            ErrorCode::Load(_) => Domain::Load,
            ErrorCode::Activate(_) => Domain::Activate,
            ErrorCode::Perm(_) => Domain::Perm,
            ErrorCode::Quota(_) => Domain::Quota,
            ErrorCode::Timeout(_) => Domain::Timeout,
            ErrorCode::Trap(_) => Domain::Trap,
            ErrorCode::Call(_) => Domain::Call,
            ErrorCode::Host(_) => Domain::Host,
        }
    }

    /// 域内原因段（不含域前缀），如 `"PARSE"`。
    pub fn reason(&self) -> &'static str {
        match self {
            ErrorCode::Manifest(r) => r.as_str(),
            ErrorCode::Compat(r) => r.as_str(),
            ErrorCode::Deps(r) => r.as_str(),
            ErrorCode::Conflict(r) => r.as_str(),
            ErrorCode::Load(r) => r.as_str(),
            ErrorCode::Activate(r) => r.as_str(),
            ErrorCode::Perm(r) => r.as_str(),
            ErrorCode::Quota(r) => r.as_str(),
            ErrorCode::Timeout(r) => r.as_str(),
            ErrorCode::Trap(r) => r.as_str(),
            ErrorCode::Call(r) => r.as_str(),
            ErrorCode::Host(r) => r.as_str(),
        }
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "E_{}_{}", self.domain(), self.reason())
    }
}

impl FromStr for ErrorCode {
    type Err = ParseErrorCodeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let rest = s
            .strip_prefix("E_")
            .ok_or_else(|| ParseErrorCodeError(s.into()))?;
        let (domain, reason) = rest
            .split_once('_')
            .ok_or_else(|| ParseErrorCodeError(s.into()))?;
        let code = match Domain::parse(domain) {
            Some(Domain::Manifest) => codes::ManifestCode::parse(reason).map(ErrorCode::Manifest),
            Some(Domain::Compat) => codes::CompatCode::parse(reason).map(ErrorCode::Compat),
            Some(Domain::Deps) => codes::DepsCode::parse(reason).map(ErrorCode::Deps),
            Some(Domain::Conflict) => codes::ConflictCode::parse(reason).map(ErrorCode::Conflict),
            Some(Domain::Load) => codes::LoadCode::parse(reason).map(ErrorCode::Load),
            Some(Domain::Activate) => codes::ActivateCode::parse(reason).map(ErrorCode::Activate),
            Some(Domain::Perm) => codes::PermCode::parse(reason).map(ErrorCode::Perm),
            Some(Domain::Quota) => codes::QuotaCode::parse(reason).map(ErrorCode::Quota),
            Some(Domain::Timeout) => codes::TimeoutCode::parse(reason).map(ErrorCode::Timeout),
            Some(Domain::Trap) => codes::TrapCode::parse(reason).map(ErrorCode::Trap),
            Some(Domain::Call) => codes::CallCode::parse(reason).map(ErrorCode::Call),
            Some(Domain::Host) => codes::HostCode::parse(reason).map(ErrorCode::Host),
            None => None,
        };
        code.ok_or_else(|| ParseErrorCodeError(s.into()))
    }
}

/// 错误码字符串无法解析。
#[derive(Debug, thiserror::Error)]
#[error("无法解析为错误码：{0}")]
pub struct ParseErrorCodeError(String);

// 错误码序列化为字符串跨界传递，反序列化结果与原值相等（spec error-model Scenario 2）。
impl Serialize for ErrorCode {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for ErrorCode {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        ErrorCode::from_str(&s).map_err(D::Error::custom)
    }
}

/// 三段式错误结构：错误码 / 面向用户的一句话 / 可选的开发者细节。
///
/// 前两段进 UI；`developer_detail`（完整路径、回溯、相关 id）只进日志，不进 UI。
#[derive(Debug, Clone, thiserror::Error, Serialize, Deserialize)]
#[error("{code}: {user_message}")]
pub struct TesseraError {
    pub code: ErrorCode,
    /// 面向用户的一句话：说明发生了什么，不含技术细节。
    pub user_message: String,
    /// 面向开发者的细节：完整路径、回溯、相关 id；只进日志，不进 UI。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub developer_detail: Option<String>,
}

impl TesseraError {
    /// 构造无开发者细节的错误（合法形态：细节段允许缺省）。
    pub fn new(code: ErrorCode, user_message: impl Into<String>) -> Self {
        Self {
            code,
            user_message: user_message.into(),
            developer_detail: None,
        }
    }

    /// 构造带开发者细节的错误。
    pub fn with_detail(
        code: ErrorCode,
        user_message: impl Into<String>,
        developer_detail: impl Into<String>,
    ) -> Self {
        Self {
            code,
            user_message: user_message.into(),
            developer_detail: Some(developer_detail.into()),
        }
    }

    /// `E_HOST_*` 域专用入口：类型上只接受宿主域错误码，因此经此构造的错误
    /// [`is_host_bug`](Self::is_host_bug) 恒为真（宿主缺陷不被误报为插件问题）。
    pub fn host(
        code: HostCode,
        user_message: impl Into<String>,
        developer_detail: impl Into<String>,
    ) -> Self {
        Self {
            code: ErrorCode::Host(code),
            user_message: user_message.into(),
            developer_detail: Some(developer_detail.into()),
        }
    }

    /// 是否宿主自身的缺陷（`E_HOST_*` 域）。由错误码的域派生，不可能与域不一致。
    pub fn is_host_bug(&self) -> bool {
        self.code.domain() == Domain::Host
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 4.1 验证：每个错误码的字符串形式符合 `E_{DOMAIN}_{REASON}` 格式。
    #[test]
    fn all_codes_match_format() {
        let all = ErrorCode::all();
        assert!(
            all.len() >= 40,
            "§12.1 示例列 38 个 + §4.4 流水线明确使用的 E_MANIFEST_VERSION / E_MANIFEST_MAIN，实际 {}",
            all.len()
        );
        for code in &all {
            let s = code.to_string();
            assert!(s.starts_with("E_"), "{s} 缺少 E_ 前缀");
            let body = &s[2..];
            assert!(
                body.chars()
                    .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_'),
                "{s} 含非大写/非数字/非下划线字符"
            );
            assert_eq!(
                body.split('_').next().unwrap(),
                code.domain().as_str(),
                "{s} 域段与 domain() 不一致"
            );
            assert!(body.contains('_'), "{s} 缺少 REASON 段");
        }
    }

    /// 4.2 验证：全部错误码 FromStr round-trip 相等。
    #[test]
    fn from_str_round_trip() {
        for code in ErrorCode::all() {
            let parsed: ErrorCode = code
                .to_string()
                .parse()
                .unwrap_or_else(|e| panic!("解析 {code} 失败：{e}"));
            assert_eq!(parsed, code);
        }
        assert!("E_NOPE_X".parse::<ErrorCode>().is_err());
        assert!("E_MANIFEST".parse::<ErrorCode>().is_err());
        assert!("MANIFEST_PARSE".parse::<ErrorCode>().is_err());
    }

    /// 4.2 验证：serde 序列化为字符串、反序列化与原值相等。
    #[test]
    fn serde_round_trip() {
        for code in ErrorCode::all() {
            let json = serde_json::to_string(&code).unwrap();
            assert!(json.starts_with("\"E_"), "{json} 不是字符串形式");
            let back: ErrorCode = serde_json::from_str(&json).unwrap();
            assert_eq!(back, code);
        }
    }

    /// 4.3 验证：有/无开发者细节两种构造均合法。
    #[test]
    fn constructor_with_and_without_detail() {
        let bare = TesseraError::new(
            ErrorCode::Manifest(codes::ManifestCode::Parse),
            "插件清单无法读取",
        );
        assert_eq!(bare.developer_detail, None);
        assert_eq!(bare.user_message, "插件清单无法读取");

        let detailed = TesseraError::with_detail(
            ErrorCode::Manifest(codes::ManifestCode::Parse),
            "插件清单无法读取",
            "路径 C:\\x\\plugin.json，错误：EOF",
        );
        assert_eq!(
            detailed.developer_detail.as_deref(),
            Some("路径 C:\\x\\plugin.json，错误：EOF")
        );
    }

    /// 4.4 验证：宿主入口构造的错误带宿主 bug 标记，其他域不带。
    #[test]
    fn host_entry_marks_host_bug() {
        let host_err = TesseraError::host(HostCode::Panic, "宿主内部错误", "panic at kernel.rs:42");
        assert!(host_err.is_host_bug());

        let other = TesseraError::new(
            ErrorCode::Trap(codes::TrapCode::Unreachable),
            "插件执行了不可达指令",
        );
        assert!(!other.is_host_bug());

        // 全量核对：只有 Host 域的错误码 is_host_bug 为真
        for code in ErrorCode::all() {
            let err = TesseraError::new(code, "msg");
            assert_eq!(err.is_host_bug(), code.domain() == Domain::Host);
        }
    }

    /// 域集合封闭：12 个域与 §12.1 一致。
    #[test]
    fn domain_set_is_closed() {
        let domains: Vec<&str> = Domain::ALL.iter().map(|d| d.as_str()).collect();
        assert_eq!(
            domains,
            vec![
                "MANIFEST", "COMPAT", "DEPS", "CONFLICT", "LOAD", "ACTIVATE", "PERM", "QUOTA",
                "TIMEOUT", "TRAP", "CALL", "HOST",
            ]
        );
    }
}

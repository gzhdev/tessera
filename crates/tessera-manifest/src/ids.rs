//! 标识符类型（design.md D6）：局部名与全限定名用不同类型区分。
//!
//! §4.2 规定 `contributes` 内一律写**局部名**，宿主在注册时拼成
//! `{plugin_id}.{local_name}` 的全限定名。两者用 [`LocalName`] 与
//! [`QualifiedId`] 两个独立类型表示，拼接的唯一入口是
//! [`PluginId::qualify`]——不存在从 `LocalName` 直接得到 `QualifiedId`
//! 的 API，类型系统阻止「把局部名当全限定名注册」这类 bug。

use serde::{Deserialize, Deserializer, Serialize};

/// 插件标识的长度上限（§4.2：≤ 128）。
pub const PLUGIN_ID_MAX: usize = 128;
/// 局部名的长度上限（与插件 id 同一量级约束）。
pub const LOCAL_NAME_MAX: usize = 128;

/// 插件标识：反向域名格式（§4.2）。
///
/// 正则 `^[a-z0-9]+(\.[a-z0-9-]+)+$`，长度 ≤ 128。反序列化时即校验，
/// 非法值在清单解析阶段（流水线步骤 2）被拒绝。
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
pub struct PluginId(String);

impl PluginId {
    /// 构造并校验格式与长度。`raw` 格式非法时返回 `None`。
    pub fn parse(raw: &str) -> Option<Self> {
        plugin_id_bounds_check(raw)
            .ok()
            .map(|_| Self(raw.to_owned()))
    }

    /// 原始字符串形式。
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// 拼接全限定名（§4.2）：`com.example.myplugin` + `run` → `com.example.myplugin.run`。
    ///
    /// 这是从局部名得到全限定名的**唯一**入口。
    pub fn qualify(&self, local: &LocalName) -> QualifiedId {
        QualifiedId(format!("{}.{}", self.0, local.as_str()))
    }
}

impl<'de> Deserialize<'de> for PluginId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        plugin_id_bounds_check(&raw)
            .map(|_| Self(raw))
            .map_err(serde::de::Error::custom)
    }
}

impl std::fmt::Display for PluginId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// `contributes` 内部使用的局部名（§6.2）。
///
/// 正则 `^[a-zA-Z][a-zA-Z0-9-]*(\.[a-zA-Z][a-zA-Z0-9-]*)*$`（允许点分隔的
/// 多段，如 `tools.format`），长度 ≤ 128。反序列化时即校验。
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
pub struct LocalName(String);

impl LocalName {
    /// 构造并校验格式。非法时返回 `None`。
    pub fn parse(raw: &str) -> Option<Self> {
        local_name_bounds_check(raw)
            .ok()
            .map(|_| Self(raw.to_owned()))
    }

    /// 原始字符串形式。
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for LocalName {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        local_name_bounds_check(&raw)
            .map(|_| Self(raw))
            .map_err(serde::de::Error::custom)
    }
}

impl std::fmt::Display for LocalName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// 全限定贡献项标识：`{plugin_id}.{local_name}`。
///
/// 只能经 [`PluginId::qualify`]（本 crate 内拼接）或 [`QualifiedId::parse`]
/// （外部已占用 id 的反序列化，格式必须能拆回合法的插件 id + 局部名）产生。
/// 把裸局部名当全限定名传入 `parse` 会因拆不出插件 id 段而失败。
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
pub struct QualifiedId(String);

impl QualifiedId {
    /// 解析并校验：必须能拆成合法的 `{plugin_id}.{local_name}`，否则返回 `None`。
    pub fn parse(raw: &str) -> Option<Self> {
        let (head, tail) = raw.rsplit_once('.')?;
        PluginId::parse(head)?;
        LocalName::parse(tail)?;
        Some(Self(raw.to_owned()))
    }

    /// 原始字符串形式。
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// 拆解为（插件 id，局部名）。
    pub fn split(&self) -> (PluginId, LocalName) {
        let (head, tail) = self.0.rsplit_once('.').expect("parse 已保证含点分隔");
        (
            PluginId::parse(head).expect("parse 已保证插件 id 段合法"),
            LocalName::parse(tail).expect("parse 已保证局部名段合法"),
        )
    }
}

impl<'de> Deserialize<'de> for QualifiedId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        Self::parse(&raw).ok_or_else(|| {
            serde::de::Error::custom(format!(
                "`{raw}` 不是合法的全限定 id（应为 {{plugin_id}}.{{local_name}} 形式）"
            ))
        })
    }
}

impl std::fmt::Display for QualifiedId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// `^[a-z0-9]+(\.[a-z0-9-]+)+$`，总长 ≤ 128（§4.2）。
pub(crate) fn plugin_id_bounds_check(raw: &str) -> Result<(), String> {
    if raw.len() > PLUGIN_ID_MAX {
        return Err(format!("id 长度 {} 超过上限 {PLUGIN_ID_MAX}", raw.len()));
    }
    let mut segments = raw.split('.');
    let first_ok = segments.next().is_some_and(|s| {
        !s.is_empty()
            && s.chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
    });
    if !first_ok {
        return Err(format!(
            "id `{raw}` 必须是反向域名格式（如 com.example.myplugin），首段只含小写字母与数字"
        ));
    }
    for seg in segments {
        if seg.is_empty()
            || !seg
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        {
            return Err(format!(
                "id `{raw}` 必须是反向域名格式（如 com.example.myplugin），各段只含小写字母、数字与连字符"
            ));
        }
    }
    // 至少两段：`^...(\....)+$`
    if !raw.contains('.') {
        return Err(format!(
            "id `{raw}` 必须是反向域名格式（如 com.example.myplugin），至少两段点分隔"
        ));
    }
    Ok(())
}

/// `^[a-zA-Z][a-zA-Z0-9-]*(\.[a-zA-Z][a-zA-Z0-9-]*)*$`，总长 ≤ 128（§6.2）。
pub(crate) fn local_name_bounds_check(raw: &str) -> Result<(), String> {
    if raw.is_empty() {
        return Err("局部名不能为空".to_owned());
    }
    if raw.len() > LOCAL_NAME_MAX {
        return Err(format!("局部名 `{raw}` 长度超过上限 {LOCAL_NAME_MAX}"));
    }
    for seg in raw.split('.') {
        let mut chars = seg.chars();
        if !chars.next().is_some_and(|c| c.is_ascii_alphabetic()) {
            return Err(format!(
                "局部名 `{raw}` 的每段必须以字母开头（允许 a-zA-Z0-9-，点分隔多段）"
            ));
        }
        if !chars.all(|c| c.is_ascii_alphanumeric() || c == '-') {
            return Err(format!(
                "局部名 `{raw}` 的每段只允许字母、数字与连字符（点分隔多段）"
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 2.3 验证：`com.example.myplugin` + `run` 拼出 `com.example.myplugin.run`。
    #[test]
    fn qualify_joins_plugin_id_and_local_name() {
        let plugin = PluginId::parse("com.example.myplugin").unwrap();
        let local = LocalName::parse("run").unwrap();
        assert_eq!(plugin.qualify(&local).as_str(), "com.example.myplugin.run");
    }

    /// 2.3 验证：类型系统阻止把局部名直接当全限定名使用——
    /// `QualifiedId` 没有 `From<&LocalName>`，`PluginId::qualify` 是唯一拼接
    /// 入口；把局部名当字符串传给 `QualifiedId::parse` 也拆不出插件 id 段。
    #[test]
    fn qualified_id_only_comes_from_qualify() {
        let local = LocalName::parse("run").unwrap();
        // 单段局部名不含点，parse 无法拆出 {plugin_id}.{local} 两段
        assert!(QualifiedId::parse(local.as_str()).is_none());
        // 局部名段前的部分必须是合法插件 id（至少两段点分隔）
        assert!(QualifiedId::parse("run.open").is_none());
    }

    /// 全限定名可无损拆回（插件 id, 局部名）。
    #[test]
    fn qualified_id_round_trip() {
        let plugin = PluginId::parse("com.example.myplugin").unwrap();
        let local = LocalName::parse("run").unwrap();
        let (pid, lname) = plugin.qualify(&local).split();
        assert_eq!(pid, plugin);
        assert_eq!(lname, local);
    }

    #[test]
    fn plugin_id_format_rules() {
        assert!(PluginId::parse("com.example.myplugin").is_some());
        assert!(PluginId::parse("com.example.my-plugin2").is_some());
        // 单段、大写、下划线、空段均非法
        assert!(PluginId::parse("myplugin").is_none());
        assert!(PluginId::parse("MyPlugin").is_none());
        assert!(PluginId::parse("com..example").is_none());
        // 反向域名首段只允许字母数字（无连字符）
        assert!(PluginId::parse("my-plugin.example").is_none());
        // 长度上限 128
        let long = format!("a.{}", "b".repeat(126));
        assert_eq!(long.len(), 128);
        assert!(PluginId::parse(&long).is_some());
        let too_long = format!("a.{}", "b".repeat(127));
        assert!(PluginId::parse(&too_long).is_none());
    }

    #[test]
    fn local_name_format_rules() {
        assert!(LocalName::parse("run").is_some());
        assert!(LocalName::parse("tools.format-all").is_some());
        assert!(LocalName::parse("2run").is_none()); // 首字符必须是字母
        assert!(LocalName::parse("-run").is_none());
        assert!(LocalName::parse("").is_none());
        assert!(LocalName::parse("run..x").is_none());
    }
}

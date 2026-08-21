//! 插件清单的权威类型定义（设计书 §4.1/§4.2/§4.3）。
//!
//! 校验真值在本模块的 serde 类型上：`deny_unknown_fields` 拒绝未知字段，
//! 自定义标量类型（[`PluginId`]、[`StrictVersion`] 等）在反序列化时即做
//! 格式校验。`plugin.schema.json` 由 schemars 从这些类型生成，仅服务编辑器
//! 补全（§4.3），不是校验入口。
//!
//! 字段规范中「serde 派生表达不了」的规则（长度上限、非空、重复、挂载位）
//! 由 [`PluginManifest::field_issues`] 累积报出（design.md D2：步骤内累积）。

use std::collections::BTreeMap;

use schemars::JsonSchema;
use semver;
use serde::{Deserialize, Serialize};

use crate::ids::{LocalName, PluginId};

// ---------------------------------------------------------------------------
// 顶层结构（§4.2 字段规范）
// ---------------------------------------------------------------------------

/// 清单顶层结构。文件名固定为 `plugin.json`（§4.1）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct PluginManifest {
    /// 清单结构版本，当前仅接受 `1`（§4.2）。取值检查在流水线步骤 3。
    pub manifest_version: u32,
    pub id: PluginId,
    /// 展示名，长度 ≤ 64。
    pub name: String,
    /// 严格 SemVer 2.0.0。
    pub version: StrictVersion,
    /// 长度 ≤ 256。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    pub engines: Engines,
    /// 相对插件目录的 wasm 路径，语法与存在性校验见流水线步骤 6。
    pub main: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    /// 非空；语法在反序列化时校验，引用存在性在流水线步骤 9 校验。
    pub activation_events: Vec<ActivationEvent>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub permissions: Vec<Permission>,
    /// key 为依赖插件的 id，不得含自身（自环检查在依赖图，E_DEPS_CYCLE）。
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub dependencies: BTreeMap<PluginId, SemverRange>,
    #[serde(default, skip_serializing_if = "Contributes::is_empty")]
    pub contributes: Contributes,
}

/// 三条版本线中的两条载体（§13.1；`manifestVersion` 在顶层）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Engines {
    /// 插件要求的最低宿主能力级别，正整数。与应用 SemVer 解耦（§13.1 决策 A5）。
    pub core: u32,
    /// WIT world 版本区间（如 `^0.2.0`）。
    pub plugin_api: SemverRange,
}

// ---------------------------------------------------------------------------
// 权限（§7.1 权限目录；spec「权限声明必须归属已定义的权限类型」）
// ---------------------------------------------------------------------------

/// 权限声明。`type` 内部标记 + `deny_unknown_fields`（serde 行为已由
/// `tests/serde_probe.rs` 探针 A 验证生效，见 design.md D3）。
///
/// 三种 `net.*` 互不隐含（§7.1）：需要什么写什么。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", deny_unknown_fields, rename_all_fields = "camelCase")]
pub enum Permission {
    /// 按虚拟目录别名授予读。
    #[serde(rename = "fs.read")]
    FsRead { virtual_dir: String },
    /// 按虚拟目录别名授予写。
    #[serde(rename = "fs.write")]
    FsWrite { virtual_dir: String },
    /// https + 公网 IP，域名白名单。
    #[serde(rename = "net.http")]
    NetHttp { allowed_domains: Vec<String> },
    /// 额外允许 http 明文；IP 仍须公网。
    #[serde(rename = "net.insecure")]
    NetInsecure { allowed_domains: Vec<String> },
    /// 额外允许私有网段；协议仍须 https。
    #[serde(rename = "net.private")]
    NetPrivate { allowed_domains: Vec<String> },
    /// topic 必须在本插件命名空间下。
    #[serde(rename = "events.publish")]
    EventsPublish { topics: Vec<String> },
    /// 支持 `ns/*` 通配。
    #[serde(rename = "events.subscribe")]
    EventsSubscribe { topics: Vec<String> },
    /// 目标为全限定 command id，支持 `{plugin_id}.*`。
    #[serde(rename = "command.invoke")]
    CommandInvoke { targets: Vec<String> },
}

impl Permission {
    /// 权限类型名（`type` 标记值）。
    pub fn type_name(&self) -> &'static str {
        match self {
            Permission::FsRead { .. } => "fs.read",
            Permission::FsWrite { .. } => "fs.write",
            Permission::NetHttp { .. } => "net.http",
            Permission::NetInsecure { .. } => "net.insecure",
            Permission::NetPrivate { .. } => "net.private",
            Permission::EventsPublish { .. } => "events.publish",
            Permission::EventsSubscribe { .. } => "events.subscribe",
            Permission::CommandInvoke { .. } => "command.invoke",
        }
    }

    /// scope 的规范化串（type + scope 值），用于「同一 type 与 scope 不得重复」检查。
    ///
    /// 列表型 scope（域名 / topic / 目标）按**集合**语义比较：排序去重后再拼接，
    /// `["a","b"]` 与 `["b","a"]` 视为同一 scope（review.md #6）。
    pub fn scope_key(&self) -> String {
        let mut scope: Vec<String> = match self {
            Permission::FsRead { virtual_dir } | Permission::FsWrite { virtual_dir } => {
                vec![virtual_dir.clone()]
            }
            Permission::NetHttp { allowed_domains }
            | Permission::NetInsecure { allowed_domains }
            | Permission::NetPrivate { allowed_domains } => allowed_domains.clone(),
            Permission::EventsPublish { topics } | Permission::EventsSubscribe { topics } => {
                topics.clone()
            }
            Permission::CommandInvoke { targets } => targets.clone(),
        };
        scope.sort();
        scope.dedup();
        format!("{}|{}", self.type_name(), scope.join(","))
    }

    /// 引用的全部虚拟目录别名（步骤 10 存在性检查的输入）。
    pub fn virtual_dirs(&self) -> &[String] {
        match self {
            Permission::FsRead { virtual_dir } | Permission::FsWrite { virtual_dir } => {
                std::slice::from_ref(virtual_dir)
            }
            _ => &[],
        }
    }
}

// ---------------------------------------------------------------------------
// 激活事件（§3.4 目录；语法校验在反序列化时完成）
// ---------------------------------------------------------------------------

/// 激活事件。`onConfig:{key}` 已在 v0.3 移除（§3.4）。
///
/// JSON 表示是 §3.4 语法的字符串：`Deserialize` 与 `Serialize` 均为手写
/// （derive 会按 Rust enum 形状生成错误的 object schema 与不对称的
/// externally tagged 序列化，见 review.md #4）；`JsonSchema` 由
/// [`crate::schema`] 手写。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActivationEvent {
    /// `onCommand:{command_id}`：该命令被调用。id 可为局部名或全限定名。
    OnCommand { command: String },
    /// `onView:{view_id}`：该视图首次可见。
    OnView { view: String },
    /// `onEvent:{topic}`：该 topic 有事件发布。
    OnEvent { topic: String },
    /// `onStartup`：应用启动完成后（延迟到首帧渲染之后）。
    OnStartup,
    /// `*`：立即激活。仅供开发调试——校验通过但产生警告（spec）。
    Always,
}

impl ActivationEvent {
    /// 从 §3.4 语法的字符串解析。
    pub fn parse(raw: &str) -> Result<Self, String> {
        if raw == "*" {
            return Ok(ActivationEvent::Always);
        }
        if raw == "onStartup" {
            return Ok(ActivationEvent::OnStartup);
        }
        if let Some(id) = raw.strip_prefix("onCommand:") {
            if id.is_empty() {
                return Err(format!("激活事件 `{raw}` 缺少命令 id"));
            }
            return Ok(ActivationEvent::OnCommand {
                command: id.to_owned(),
            });
        }
        if let Some(id) = raw.strip_prefix("onView:") {
            if id.is_empty() {
                return Err(format!("激活事件 `{raw}` 缺少视图 id"));
            }
            return Ok(ActivationEvent::OnView {
                view: id.to_owned(),
            });
        }
        if let Some(topic) = raw.strip_prefix("onEvent:") {
            if topic.is_empty() {
                return Err(format!("激活事件 `{raw}` 缺少 topic"));
            }
            return Ok(ActivationEvent::OnEvent {
                topic: topic.to_owned(),
            });
        }
        Err(format!(
            "激活事件 `{raw}` 不匹配已定义语法（onCommand:{{id}} / onView:{{id}} / onEvent:{{topic}} / onStartup / *）"
        ))
    }

    /// 原始字符串形式（用于警告信息等）。
    pub fn to_raw(&self) -> String {
        match self {
            ActivationEvent::OnCommand { command } => format!("onCommand:{command}"),
            ActivationEvent::OnView { view } => format!("onView:{view}"),
            ActivationEvent::OnEvent { topic } => format!("onEvent:{topic}"),
            ActivationEvent::OnStartup => "onStartup".to_owned(),
            ActivationEvent::Always => "*".to_owned(),
        }
    }
}

impl<'de> Deserialize<'de> for ActivationEvent {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(d)?;
        Self::parse(&raw).map_err(serde::de::Error::custom)
    }
}

// 序列化为 §3.4 语法的字符串，与 Deserialize 对称（round-trip 稳定）
impl Serialize for ActivationEvent {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_raw())
    }
}

// ---------------------------------------------------------------------------
// contributes（§6.1 扩展点目录）
// ---------------------------------------------------------------------------

/// `contributes` 容器。全部子项可省略（§4.1 示例未写全也合法）。
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Contributes {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub commands: Vec<CommandContribution>,
    /// 挂载位合法 key：`toolbar` / `context` / `view/title` / `main/{菜单名}`（§6.1）。
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub menus: BTreeMap<String, Vec<MenuEntry>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub views: Vec<ViewContribution>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub keybindings: Vec<Keybinding>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub configuration: Option<Configuration>,
    /// 声明式事件订阅；额外作用是驱动 `onEvent:` 懒激活（§6.1）。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub event_subscriptions: Vec<String>,
}

impl Contributes {
    pub fn is_empty(&self) -> bool {
        *self == Contributes::default()
    }
}

/// 命令贡献。`id` 写局部名，注册时拼为全限定 id（§4.2）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CommandContribution {
    pub id: LocalName,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
}

/// 菜单项。`command` 引用可写局部名（本插件）或全限定名（他插件），§4.2。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct MenuEntry {
    pub command: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub order: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub when: Option<String>,
}

/// 视图种类（§6.1/§9.6）：当前仅声明式；`webview` 等为未来逃生舱，出现即拒绝。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum ViewKind {
    /// 声明式 UI 描述树。省略 `kind` 时的默认值。
    #[default]
    Declarative,
}

/// 视图贡献（停靠面板）。必须声明式——预注册（灰态）依赖于此（§6.1）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ViewContribution {
    pub id: LocalName,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub order: Option<u32>,
    /// 省略时取默认值 `declarative`。
    #[serde(default)]
    pub kind: ViewKind,
}

/// 快捷键。冲突时后注册者被丢弃并记 warning，不拖垮插件（§6.2）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Keybinding {
    pub command: String,
    pub key: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mac: Option<String>,
}

/// 配置项 schema（§11.1）。设置面板要在插件激活前就能显示，必须声明式。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Configuration {
    pub title: String,
    pub properties: BTreeMap<String, ConfigurationProperty>,
}

/// 配置项类型（§11.1 支持的 6 种）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum ConfigType {
    String,
    Integer,
    Number,
    Boolean,
    /// JSON 写作 `"string[]"`。
    #[serde(rename = "string[]")]
    StringArray,
    /// 渲染为下拉选择。
    Enum,
}

/// 单个配置项。`default` 必填（§11.1）；`sensitive: true` 表示密码框呈现与日志脱敏。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ConfigurationProperty {
    #[serde(rename = "type")]
    pub config_type: ConfigType,
    /// 必填——serde 层面无默认即必填，缺失直接解析失败。
    pub default: serde_json::Value,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub sensitive: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minimum: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maximum: Option<f64>,
}

// ---------------------------------------------------------------------------
// SemVer 包装（孤儿规则：外部类型不能在本 crate impl JsonSchema，故包装）
// ---------------------------------------------------------------------------

/// 严格 SemVer 2.0.0 版本号。`1.2` / `v1.2.3` 等宽松写法被拒绝。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StrictVersion(pub semver::Version);

impl std::fmt::Display for StrictVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0.to_string())
    }
}

impl Serialize for StrictVersion {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.0.to_string())
    }
}

impl<'de> Deserialize<'de> for StrictVersion {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(d)?;
        semver::Version::parse(&raw)
            .map(StrictVersion)
            .map_err(|e| {
                serde::de::Error::custom(format!(
                    "version `{raw}` 不是严格的 SemVer 2.0.0（如 1.2.0）：{e}"
                ))
            })
    }
}

/// SemVer 版本区间（如 `^0.2.0`、`>=1.0.0 <2.0.0`）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemverRange(pub semver::VersionReq);

impl std::fmt::Display for SemverRange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0.to_string())
    }
}

impl Serialize for SemverRange {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.0.to_string())
    }
}

impl<'de> Deserialize<'de> for SemverRange {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(d)?;
        semver::VersionReq::parse(&raw)
            .map(SemverRange)
            .map_err(|e| serde::de::Error::custom(format!("`{raw}` 不是合法的 SemVer 区间：{e}")))
    }
}

// ---------------------------------------------------------------------------
// 字段规范校验（serde 派生表达不了的规则；design.md D2 步骤内累积）
// ---------------------------------------------------------------------------

/// `name` 长度上限（§4.2）。
pub const NAME_MAX: usize = 64;
/// `description` 长度上限（§4.2）。
pub const DESCRIPTION_MAX: usize = 256;
/// 菜单挂载位的固定 key 与前缀（§6.1）。
const FIXED_MENU_LOCATIONS: &[&str] = &["toolbar", "context", "view/title"];
const MENU_LOCATION_PREFIX: &str = "main/";

impl PluginManifest {
    /// 字段规范检查（§4.2 中 serde 派生覆盖不了的规则），**累积**返回全部问题。
    ///
    /// 任一问题都对应流水线步骤 2 的失败（`E_MANIFEST_SCHEMA`）。
    pub fn field_issues(&self) -> Vec<String> {
        let mut issues = Vec::new();

        if self.name.chars().count() > NAME_MAX {
            issues.push(format!(
                "name 长度 {} 超过上限 {NAME_MAX}",
                self.name.chars().count()
            ));
        }
        if let Some(desc) = &self.description
            && desc.chars().count() > DESCRIPTION_MAX
        {
            issues.push(format!(
                "description 长度 {} 超过上限 {DESCRIPTION_MAX}",
                desc.chars().count()
            ));
        }
        if self.engines.core < 1 {
            issues.push(format!(
                "engines.core 必须是正整数，实际为 {}",
                self.engines.core
            ));
        }
        if self.activation_events.is_empty() {
            issues.push("activationEvents 必须非空".to_owned());
        }

        let mut seen_scopes = std::collections::BTreeSet::new();
        for perm in &self.permissions {
            if !seen_scopes.insert(perm.scope_key()) {
                issues.push(format!(
                    "permissions 中重复声明了同一 type 与 scope 的组合：`{}`",
                    perm.type_name()
                ));
            }
        }

        for location in self.contributes.menus.keys() {
            let valid = FIXED_MENU_LOCATIONS.contains(&location.as_str())
                || location.starts_with(MENU_LOCATION_PREFIX);
            if !valid {
                issues.push(format!(
                    "menus 挂载位 `{location}` 不在已定义目录（toolbar / context / view/title / main/{{菜单名}}）"
                ));
            }
        }

        issues
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 2.2 验证：8 种权限类型各有一条正例解析成功，且 variant 归属正确。
    #[test]
    fn eight_permission_types_parse() {
        let cases: Vec<(&str, &str)> = vec![
            (
                r#"{ "type": "fs.read", "virtualDir": "workspace" }"#,
                "fs.read",
            ),
            (
                r#"{ "type": "fs.write", "virtualDir": "plugin-data" }"#,
                "fs.write",
            ),
            (
                r#"{ "type": "net.http", "allowedDomains": ["api.example.com"] }"#,
                "net.http",
            ),
            (
                r#"{ "type": "net.insecure", "allowedDomains": ["example.com"] }"#,
                "net.insecure",
            ),
            (
                r#"{ "type": "net.private", "allowedDomains": ["10.0.0.5"] }"#,
                "net.private",
            ),
            (
                r#"{ "type": "events.publish", "topics": ["com.example.p/t"] }"#,
                "events.publish",
            ),
            (
                r#"{ "type": "events.subscribe", "topics": ["core/*"] }"#,
                "events.subscribe",
            ),
            (
                r#"{ "type": "command.invoke", "targets": ["com.example.other.format"] }"#,
                "command.invoke",
            ),
        ];
        for (json, expected_type) in cases {
            let perm: Permission =
                serde_json::from_str(json).unwrap_or_else(|e| panic!("{json}：{e}"));
            assert_eq!(perm.type_name(), expected_type);
        }
    }

    /// 2.2 验证：`shell.execute` 被拒绝（§7.1：明确不提供）。
    #[test]
    fn shell_execute_rejected() {
        let err = serde_json::from_str::<Permission>(r#"{ "type": "shell.execute" }"#)
            .expect_err("shell.execute 必须被拒绝");
        assert!(err.to_string().contains("shell.execute"), "{err}");
    }

    /// 2.4 验证：省略 `kind` 的视图解析成功且值为 `declarative`。
    #[test]
    fn view_omitted_kind_defaults_to_declarative() {
        let view: ViewContribution =
            serde_json::from_str(r#"{ "id": "panel", "name": "My Panel" }"#).unwrap();
        assert_eq!(view.kind, ViewKind::Declarative);
    }

    /// 2.4 验证：写 `"kind": "webview"` 解析失败。
    #[test]
    fn view_webview_kind_rejected() {
        let err = serde_json::from_str::<ViewContribution>(
            r#"{ "id": "panel", "name": "P", "kind": "webview" }"#,
        )
        .expect_err("webview 必须被拒绝");
        assert!(err.to_string().contains("webview"), "{err}");
    }

    /// 2.5 验证：缺 `default` 的配置项解析失败（serde 层面必填）。
    #[test]
    fn config_missing_default_rejected() {
        let err = serde_json::from_str::<ConfigurationProperty>(
            r#"{ "type": "string", "description": "无默认值" }"#,
        )
        .expect_err("缺 default 必须被拒绝");
        assert!(err.to_string().contains("default"), "{err}");
    }

    /// 2.5 验证：标注 `sensitive: true` 的项解析后该标注可被读取。
    #[test]
    fn config_sensitive_flag_readable() {
        let prop: ConfigurationProperty =
            serde_json::from_str(r#"{ "type": "string", "default": "", "sensitive": true }"#)
                .unwrap();
        assert!(prop.sensitive);
        // 对照：未标注时为 false
        let plain: ConfigurationProperty =
            serde_json::from_str(r#"{ "type": "integer", "default": 3 }"#).unwrap();
        assert!(!plain.sensitive);
    }

    /// 2.5 验证：6 种配置项值类型都可解析。
    #[test]
    fn six_config_types_parse() {
        for (json, expected) in [
            (r#""string""#, ConfigType::String),
            (r#""integer""#, ConfigType::Integer),
            (r#""number""#, ConfigType::Number),
            (r#""boolean""#, ConfigType::Boolean),
            (r#""string[]""#, ConfigType::StringArray),
            (r#""enum""#, ConfigType::Enum),
        ] {
            let ty: ConfigType = serde_json::from_str(json).unwrap();
            assert_eq!(ty, expected);
        }
    }

    /// review.md #4：`ActivationEvent` 序列化为 §3.4 语法字符串，
    /// 与反序列化对称（serialize → deserialize round-trip 相等）。
    #[test]
    fn activation_event_serde_round_trip() {
        for raw in [
            "onCommand:run",
            "onView:com.example.myplugin.panel",
            "onEvent:core/config-changed",
            "onStartup",
            "*",
        ] {
            let event: ActivationEvent =
                serde_json::from_str(&serde_json::to_string(raw).unwrap()).unwrap();
            let serialized = serde_json::to_string(&event).unwrap();
            assert_eq!(serialized, serde_json::to_string(raw).unwrap());
            let back: ActivationEvent = serde_json::from_str(&serialized).unwrap();
            assert_eq!(back, event, "{raw}");
        }
    }

    /// review.md #6：scope 查重按集合语义——域名顺序不同不算新 scope。
    #[test]
    fn scope_key_is_order_insensitive() {
        let a: Permission =
            serde_json::from_str(r#"{ "type": "net.http", "allowedDomains": ["a.com", "b.com"] }"#)
                .unwrap();
        let b: Permission =
            serde_json::from_str(r#"{ "type": "net.http", "allowedDomains": ["b.com", "a.com"] }"#)
                .unwrap();
        assert_eq!(a.scope_key(), b.scope_key());
    }
}

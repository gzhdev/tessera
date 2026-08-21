//! `plugin.schema.json` 的生成（设计书 §4.3）。
//!
//! 校验真值在 [`crate::manifest`] 的 serde 类型；本模块的 schema 仅供
//! 编辑器补全（VS Code `json.schemas`），**不得**用作校验入口。生成命令
//! 见 `src/bin/gen-schema.rs`，产出 check-in 于 `schema/plugin.schema.json`，
//! CI 校验重新生成后逐字节一致。

use schemars::JsonSchema;
use schemars::Schema;
use schemars::SchemaGenerator;

use crate::ids::{LOCAL_NAME_MAX, PLUGIN_ID_MAX};
use crate::manifest::{ActivationEvent, SemverRange, StrictVersion};

// 孤儿规则：`semver::Version` 等外部类型无法在本 crate 实现 `JsonSchema`，
// 因此 `StrictVersion` / `SemverRange` 以包装类型存在，impl 写在这里。
// 同理，JSON 表示为字符串的 `ActivationEvent` 与带格式约束的标识符类型
// 也手写 impl，使补全提示与实际校验规则一致。

impl JsonSchema for StrictVersion {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        std::borrow::Cow::Borrowed("StrictVersion")
    }

    fn json_schema(_gen: &mut SchemaGenerator) -> Schema {
        schemars::json_schema!({
            "type": "string",
            "pattern": r"^\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?(\+[0-9A-Za-z.-]+)?$",
            "description": "严格 SemVer 2.0.0（如 1.2.0）"
        })
    }
}

impl JsonSchema for SemverRange {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        std::borrow::Cow::Borrowed("SemverRange")
    }

    fn json_schema(_gen: &mut SchemaGenerator) -> Schema {
        schemars::json_schema!({
            "type": "string",
            "examples": ["^0.2.0", ">=1.0.0 <2.0.0"],
            "description": "SemVer 版本区间"
        })
    }
}

impl JsonSchema for ActivationEvent {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        std::borrow::Cow::Borrowed("ActivationEvent")
    }

    fn json_schema(_gen: &mut SchemaGenerator) -> Schema {
        schemars::json_schema!({
            "type": "string",
            "pattern": r"^(onCommand:.+|onView:.+|onEvent:.+|onStartup|\*)$",
            "description": "激活事件（§3.4）：onCommand:{id} / onView:{id} / onEvent:{topic} / onStartup / *"
        })
    }
}

impl JsonSchema for crate::ids::PluginId {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        std::borrow::Cow::Borrowed("PluginId")
    }

    fn json_schema(_gen: &mut SchemaGenerator) -> Schema {
        schemars::json_schema!({
            "type": "string",
            "pattern": r"^[a-z0-9]+(\.[a-z0-9-]+)+$",
            "maxLength": PLUGIN_ID_MAX,
            "description": "反向域名格式的插件 id（如 com.example.myplugin）"
        })
    }
}

impl JsonSchema for crate::ids::LocalName {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        std::borrow::Cow::Borrowed("LocalName")
    }

    fn json_schema(_gen: &mut SchemaGenerator) -> Schema {
        schemars::json_schema!({
            "type": "string",
            "pattern": r"^[a-zA-Z][a-zA-Z0-9-]*(\.[a-zA-Z][a-zA-Z0-9-]*)*$",
            "maxLength": LOCAL_NAME_MAX,
            "description": "contributes 内的局部名（注册时由宿主拼接为全限定 id）"
        })
    }
}

impl JsonSchema for crate::ids::QualifiedId {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        std::borrow::Cow::Borrowed("QualifiedId")
    }

    fn json_schema(_gen: &mut SchemaGenerator) -> Schema {
        schemars::json_schema!({
            "type": "string",
            "pattern": r"^[a-z0-9]+(\.[a-z0-9-]+)+\.[a-zA-Z][a-zA-Z0-9-]*(\.[a-zA-Z][a-zA-Z0-9-]*)*$",
            "description": "全限定贡献项 id：{plugin_id}.{local_name}"
        })
    }
}

/// 生成 `plugin.schema.json` 的文本内容。
///
/// 确定性：同一份类型定义产出的文本逐字节稳定（重复执行无 diff），
/// CI 据此校验生成物与类型的一致性。
pub fn generate_schema_json() -> String {
    let schema = schemars::schema_for!(crate::manifest::PluginManifest);
    let mut json = serde_json::to_string_pretty(&schema).expect("schema 序列化不会失败");
    json.push('\n');
    json
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 6.1 验证（部分）：生成内容稳定——同进程两次生成逐字节一致，
    /// 且是合法 JSON、根类型为 object。
    #[test]
    fn generated_schema_is_stable_and_valid() {
        let first = generate_schema_json();
        let second = generate_schema_json();
        assert_eq!(first, second);

        let value: serde_json::Value = serde_json::from_str(&first).unwrap();
        assert_eq!(value["type"], "object");
    }
}

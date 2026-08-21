//! 描述树校验与降级（design.md D3 / D4，spec「未知组件类型必须局部降级」）。
//!
//! 两段式校验（D4）：第一阶段只做最低限度的形状检查（节点是对象、`type`
//! 是字符串），其余字段全部以原始 [`serde_json::Value`] 保留；第二阶段逐
//! 节点构造强类型 JSON 交给 serde 解析为 [`UiNode`]，失败的节点标记为
//! 降级，不影响兄弟节点，不拒绝整树（D3）。
//!
//! 第一阶段刻意不设强类型字段——任何后代节点的 `id` / `on` / `children`
//! 形状错误都在第二阶段被检出并**节点级**降级，而不是让整棵树在宽松
//! 解析阶段就失败（design.md D4「宽松的中间表示」）。
//!
//! 同时在此检出同树重复 `id`、非容器携带 `children` 与「props 类型不符」
//! 并记警告（spec「节点标识唯一」「属性类型不符被报告」）。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use ts_rs::TS;

use crate::node::{Component, UiNode};
use crate::tree::UiTree;

/// 校验产出（design.md D3）：树 + 降级清单 + 警告清单。
///
/// 不返回 `Result`——「基本可用但有节点要降级」是合法状态，由调用方
/// （宿主记日志 / 前端渲染占位符）决定如何处置降级节点。
#[derive(Debug, Clone, PartialEq)]
pub struct ValidationOutcome {
    /// 解析出的树（根解析失败时为 `None`，见 [`Self::is_usable`]）。
    ///
    /// 未知类型节点在树中缺席（整棵降级），并列入降级清单，
    /// 渲染侧据此渲染错误占位符（§9.3）。
    pub tree: Option<UiTree>,
    /// 需要降级渲染的节点（含路径定位）。
    pub degraded: Vec<DegradedNode>,
    /// 非致命问题（重复 id、属性类型不符等）。
    pub warnings: Vec<Warning>,
}

impl ValidationOutcome {
    /// 树是否可用（根节点未整体失败）。即便有降级或警告，只要根解析出即为真。
    pub fn is_usable(&self) -> bool {
        self.tree.is_some()
    }
}

/// 一个需要降级渲染的节点。
#[derive(Debug, Clone, PartialEq)]
pub struct DegradedNode {
    /// 节点的 `id`（若可取得且为字符串）。
    pub node_id: Option<String>,
    /// 未知的组件类型标记（结构无效时为 `<…>` 形式的占位描述）。
    pub type_tag: String,
    /// 在树中的定位路径（如 `root/children[2]/children[0]`）。
    pub path: String,
}

/// 一条校验警告（非致命）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "Warning.ts", rename_all = "camelCase")]
pub struct Warning {
    /// 警告类别。
    pub kind: WarningKind,
    /// 定位路径。
    pub path: String,
    /// 人类可读描述（中文）。
    pub message: String,
}

/// 警告类别。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(export_to = "WarningKind.ts", rename_all = "kebab-case")]
pub enum WarningKind {
    /// 同一棵树中出现重复 `id`。
    DuplicateId,
    /// 属性类型与契约不符。
    PropTypeMismatch,
    /// 未知组件类型（伴随降级）。
    UnknownComponent,
    /// 非容器组件携带了 `children`（超契约输入，宽容接受并提示）。
    UnexpectedChildren,
}

/// 校验一棵 JSON 描述树，产出结构化降级结果（spec ui-schema）。
///
/// 入口接受 `serde_json::Value`（宿主已拿到的插件返回）。未知类型节点
/// 局部降级：整棵子树从渲染树中剔除并列入降级清单。
pub fn validate_tree(input: &serde_json::Value) -> ValidationOutcome {
    let mut degraded = Vec::new();
    let mut warnings = Vec::new();
    let mut seen_ids: HashMap<String, String> = HashMap::new();

    // 输入形状为 `{ "root": <节点> }`（tree.rs UiTree）。缺 root / root
    // 非对象是树本身的残缺，整树不可用；root 自身的字段错误则交给
    // resolve_node 做节点级降级。
    let root = match input.get("root") {
        Some(r) if r.is_object() => {
            resolve_node(r, "root", &mut degraded, &mut warnings, &mut seen_ids)
        }
        _ => {
            warnings.push(Warning {
                kind: WarningKind::PropTypeMismatch,
                path: "root".to_string(),
                message: "描述树缺少 `root` 字段或其不是对象".to_string(),
            });
            None
        }
    };

    let tree = root.map(|r| UiTree { root: r });
    ValidationOutcome {
        tree,
        degraded,
        warnings,
    }
}

/// 第二阶段：逐节点强类型解析，失败即降级（含其子树一并降级）。
///
/// 收原始 JSON 值（D4 宽松中间表示），只假设「若能解析则是对象」。
fn resolve_node(
    raw: &serde_json::Value,
    path: &str,
    degraded: &mut Vec<DegradedNode>,
    warnings: &mut Vec<Warning>,
    seen_ids: &mut HashMap<String, String>,
) -> Option<UiNode> {
    let obj = match raw.as_object() {
        Some(o) => o,
        None => {
            degrade(
                degraded,
                warnings,
                path,
                None,
                "<非对象>".to_string(),
                format!("节点不是 JSON 对象：{raw}"),
            );
            return None;
        }
    };

    let type_tag = match obj.get("type") {
        Some(serde_json::Value::String(t)) => t.clone(),
        Some(other) => {
            let node_id = obj.get("id").and_then(|v| v.as_str()).map(String::from);
            degrade(
                degraded,
                warnings,
                path,
                node_id,
                "<type 非字符串>".to_string(),
                format!("节点 `type` 字段必须是字符串，实际为 {other}"),
            );
            return None;
        }
        None => {
            let node_id = obj.get("id").and_then(|v| v.as_str()).map(String::from);
            degrade(
                degraded,
                warnings,
                path,
                node_id,
                "<缺 type>".to_string(),
                "节点缺少必需的 `type` 字段".to_string(),
            );
            return None;
        }
    };

    let raw_id = obj.get("id");
    // 重复 id 检出（在降级判断之前，未知节点的 id 也参与唯一性约束；
    // 非字符串 id 不参与——它会在下方强类型解析中失败并降级）。
    if let Some(id) = raw_id.and_then(|v| v.as_str()) {
        if let Some(first_path) = seen_ids.get(id) {
            warnings.push(Warning {
                kind: WarningKind::DuplicateId,
                path: path.to_string(),
                message: format!("重复的节点 id `{id}`（首次出现于 {first_path}）"),
            });
        } else {
            seen_ids.insert(id.to_string(), path.to_string());
        }
    }

    // 构造本节点的强类型 JSON：{type,id,props,children:[],on}。
    // - children 置空——子树由外层递归单独处理，坏子节点不连坐本节点；
    //   若 children 字段存在但不是数组，属于本节点的形状错误，显式降级；
    // - props 缺省补空对象（spec「`type` 与 `id` MUST 存在；其余字段可省略」）：
    //   全可选 props 的组件（如 vstack）恢复合法，有必填 props 的组件
    //   （如 button 的 label）正确报「属性缺失」；
    // - id / on 原样转发——形状错误（数字 id、非字符串动作值）让强类型
    //   解析失败，走节点级降级。
    let raw_children = match obj.get("children") {
        None => None,
        Some(serde_json::Value::Array(c)) => Some(c),
        Some(other) => {
            let node_id = raw_id.and_then(|v| v.as_str()).map(String::from);
            degrade(
                degraded,
                warnings,
                path,
                node_id,
                type_tag,
                format!("节点 `children` 必须是数组，实际为 {other}"),
            );
            return None;
        }
    };
    let mut node_json = serde_json::Map::new();
    node_json.insert(
        "type".to_string(),
        serde_json::Value::String(type_tag.clone()),
    );
    if let Some(id) = raw_id {
        node_json.insert("id".to_string(), id.clone());
    }
    let props = obj
        .get("props")
        .cloned()
        .unwrap_or_else(|| serde_json::Value::Object(serde_json::Map::new()));
    node_json.insert("props".to_string(), props);
    node_json.insert("children".to_string(), serde_json::Value::Array(Vec::new()));
    if let Some(on) = obj.get("on") {
        node_json.insert("on".to_string(), on.clone());
    }

    match serde_json::from_value::<UiNode>(serde_json::Value::Object(node_json)) {
        Ok(mut node) => {
            // 本节点解析成功。递归处理子节点；任一子节点失败只降级该子树。
            let mut children = Vec::new();
            if let Some(raw_children) = raw_children {
                if !raw_children.is_empty() && !node.component.is_container() {
                    warnings.push(Warning {
                        kind: WarningKind::UnexpectedChildren,
                        path: path.to_string(),
                        message: format!(
                            "组件 `{type_tag}` 不是容器类，携带的 `children` 不会渲染"
                        ),
                    });
                }
                children.reserve(raw_children.len());
                for (i, child) in raw_children.iter().enumerate() {
                    let child_path = format!("{path}/children[{i}]");
                    if let Some(c) = resolve_node(child, &child_path, degraded, warnings, seen_ids)
                    {
                        children.push(c);
                    }
                }
            }
            node.children = children;
            Some(node)
        }
        Err(e) => {
            // 白名单判别：type 不在组件清单 → 未知组件；否则失败必来自
            // props / id / on / children 的形状，归属性类型不符。
            let kind = if Component::is_known_type(&type_tag) {
                WarningKind::PropTypeMismatch
            } else {
                WarningKind::UnknownComponent
            };
            let node_id = raw_id.and_then(|v| v.as_str()).map(String::from);
            warnings.push(Warning {
                kind,
                path: path.to_string(),
                message: format!(
                    "节点 `{}`（type=`{type_tag}`）解析失败：{e}",
                    node_id.as_deref().unwrap_or("<无 id>"),
                ),
            });
            // 子树一并降级：整棵子树不单独渲染（spec 场景）。
            degraded.push(DegradedNode {
                node_id,
                type_tag,
                path: path.to_string(),
            });
            None
        }
    }
}

/// 结构无效节点的降级记录（非对象 / 缺 type / type 非字符串）。
fn degrade(
    degraded: &mut Vec<DegradedNode>,
    warnings: &mut Vec<Warning>,
    path: &str,
    node_id: Option<String>,
    type_tag: String,
    message: String,
) {
    warnings.push(Warning {
        kind: WarningKind::PropTypeMismatch,
        path: path.to_string(),
        message,
    });
    degraded.push(DegradedNode {
        node_id,
        type_tag,
        path: path.to_string(),
    });
}

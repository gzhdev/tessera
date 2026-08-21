//! 正例 fixture：六个代表性组件各一条解析成功（任务 2.2）。

use tessera_ui_schema::{UiTree, validate::validate_tree};

fn parse_positive(name: &str) -> UiTree {
    let path = format!(
        "{}/tests/fixtures/positive/{name}.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("读取 {path} 失败：{e}"));
    let value: serde_json::Value =
        serde_json::from_str(&text).unwrap_or_else(|e| panic!("{name}.json 不是合法 JSON：{e}"));
    let outcome = validate_tree(&value);
    assert!(
        outcome.degraded.is_empty(),
        "{name}.json 不应产生降级：{:?}",
        outcome.degraded
    );
    outcome
        .tree
        .unwrap_or_else(|| panic!("{name}.json 应解析出树，警告：{:?}", outcome.warnings))
}

#[test]
fn positive_vstack() {
    let tree = parse_positive("vstack");
    assert_eq!(tree.root.id, "root");
    assert_eq!(tree.root.component.type_tag(), "vstack");
    assert_eq!(tree.root.children.len(), 1);
    assert_eq!(tree.root.children[0].component.type_tag(), "text");
}

#[test]
fn positive_hstack() {
    let tree = parse_positive("hstack");
    assert_eq!(tree.root.component.type_tag(), "hstack");
    assert_eq!(tree.root.children.len(), 2);
}

#[test]
fn positive_text() {
    let tree = parse_positive("text");
    assert_eq!(tree.root.component.type_tag(), "text");
}

#[test]
fn positive_button() {
    let tree = parse_positive("button");
    assert_eq!(tree.root.component.type_tag(), "button");
    // on 透传（任务 2.3 也在此覆盖正例）
    assert_eq!(
        tree.root.on.get("click").map(String::as_str),
        Some("do-sync")
    );
}

#[test]
fn positive_text_input() {
    let tree = parse_positive("text-input");
    assert_eq!(tree.root.component.type_tag(), "text-input");
}

#[test]
fn positive_table() {
    let tree = parse_positive("table");
    assert_eq!(tree.root.component.type_tag(), "table");
    assert_eq!(
        tree.root.on.get("rowClick").map(String::as_str),
        Some("open-file")
    );
}

/// 2.1：只含 type 与 id 的最小节点解析成功（spec「最小合法节点」：
/// 其余字段可省略，含 props）。
#[test]
fn minimal_node_only_type_and_id() {
    let value = serde_json::json!({ "root": { "type": "vstack", "id": "x" } });
    let outcome = validate_tree(&value);
    assert!(
        outcome.tree.is_some(),
        "最小节点应合法，警告：{:?}",
        outcome.warnings
    );
    assert!(outcome.degraded.is_empty());
}

/// 最小节点对必填 props 组件：报「属性缺失」（PropTypeMismatch）而非结构拒绝。
#[test]
fn minimal_node_with_required_props_reports_missing_prop() {
    let value = serde_json::json!({ "root": { "type": "button", "id": "x" } });
    let outcome = validate_tree(&value);
    assert!(outcome.tree.is_none());
    assert_eq!(outcome.degraded.len(), 1);
    let w = outcome
        .warnings
        .iter()
        .find(|w| w.kind == tessera_ui_schema::WarningKind::PropTypeMismatch)
        .expect("应有属性类型不符警告");
    assert!(
        w.message.contains("label"),
        "警告应指出缺失的必填属性：{}",
        w.message
    );
}

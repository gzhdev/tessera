//! 5.1 综合 fixture：每条 spec 场景至少对应一个 fixture。
//!
//! - `all-components.json`：全组件树（34 组件各一，覆盖「已知组件通过校验」）
//! - `nested-containers.json`：嵌套容器树（覆盖「界面由节点树描述」的 children 递归）
//! - `unknown-type.json`：未知类型树（覆盖「未知组件类型必须局部降级」）
//! - `duplicate-id.json`：重复 id 树（覆盖「重复 id 被检出」）
//! - `invalid-props.json`：非法 props 树（覆盖「属性类型不符被报告」）

use tessera_ui_schema::validate::{ValidationOutcome, WarningKind, validate_tree};

fn load(name: &str) -> ValidationOutcome {
    let path = format!(
        "{}/tests/fixtures/trees/{name}.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("读取 {path} 失败：{e}"));
    let value: serde_json::Value =
        serde_json::from_str(&text).unwrap_or_else(|e| panic!("{name}.json 非合法 JSON：{e}"));
    validate_tree(&value)
}

/// 全组件树：34 个组件全部解析成功，无降级。
#[test]
fn all_components_tree_parses() {
    let outcome = load("all-components");
    assert!(outcome.is_usable());
    assert!(
        outcome.degraded.is_empty(),
        "全组件树不应有降级：{:?}",
        outcome.degraded
    );
    let root = outcome.tree.unwrap().root;
    assert_eq!(root.children.len(), 34, "应含全部 34 个组件");
    // 覆盖五类的代表抽查
    let tags: Vec<_> = root
        .children
        .iter()
        .map(|c| c.component.type_tag())
        .collect();
    for expected in [
        "vstack",
        "heading",
        "select",
        "table",
        "spinner",
        "file-picker",
    ] {
        assert!(tags.contains(&expected), "全组件树应含 {expected}");
    }
}

/// 嵌套容器树：三层嵌套（split > vstack > scroll > text / tabs > group > text）解析成功。
#[test]
fn nested_containers_tree_parses() {
    let outcome = load("nested-containers");
    assert!(outcome.is_usable());
    assert!(outcome.degraded.is_empty());
    let root = outcome.tree.unwrap().root;
    assert_eq!(root.component.type_tag(), "split");
    assert_eq!(root.children.len(), 2);
    // 左侧 split > vstack > scroll > text
    let left = &root.children[0];
    assert_eq!(left.component.type_tag(), "vstack");
    assert_eq!(left.children[0].component.type_tag(), "scroll");
    assert_eq!(left.children[0].children[0].component.type_tag(), "text");
}

/// 未知类型树：flying-toaster 降级，两个兄弟正常。
#[test]
fn unknown_type_tree_degrades() {
    let outcome = load("unknown-type");
    assert!(outcome.is_usable());
    assert_eq!(outcome.degraded.len(), 1);
    assert_eq!(outcome.degraded[0].type_tag, "flying-toaster");
    let root = outcome.tree.unwrap().root;
    assert_eq!(root.children.len(), 2, "两个正常节点保留");
    assert!(
        outcome
            .warnings
            .iter()
            .any(|w| w.kind == WarningKind::UnknownComponent),
        "应有未知组件警告"
    );
}

/// 重复 id 树：检出重复。
#[test]
fn duplicate_id_tree_warned() {
    let outcome = load("duplicate-id");
    assert!(
        outcome
            .warnings
            .iter()
            .any(|w| w.kind == WarningKind::DuplicateId),
        "应有重复 id 警告"
    );
}

/// 非法 props 树：slider.min 赋字符串，类型不符。
#[test]
fn invalid_props_tree_reported() {
    let outcome = load("invalid-props");
    assert!(
        outcome
            .warnings
            .iter()
            .any(|w| w.kind == WarningKind::PropTypeMismatch),
        "应有属性类型不符警告"
    );
}

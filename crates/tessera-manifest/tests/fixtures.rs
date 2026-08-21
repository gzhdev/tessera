//! fixture 遍历测试（任务 6.3/6.4）。
//!
//! 「每个错误码至少一条反例，且每个文件恰好触发该错误码」由目录遍历自动
//! 检查，不靠人工核对清单。
//!
//! fixture 形态约定：
//!
//! - `invalid/{错误码}/*.json`——目录名即预期错误码。三种文件形态：
//!   1. 内容不是合法 JSON：原文直接作为 `plugin.json` 输入（用于 `E_MANIFEST_PARSE`）；
//!   2. 顶层含 `"manifest"` 键：wrapper 形态，`manifest` 为清单本体，可选的
//!      `occupiedPluginIds` / `occupiedQualifiedIds` / `registeredVdirs`
//!      提供 [`ValidationContext`] 的入参；
//!   3. 顶层含 `"plugins"` 键：依赖图形态（`E_DEPS_*` 专用），每个元素是一个
//!      图节点，喂给 [`DepGraph::resolve`]。
//! - `valid/*/plugin.json`——wrapper 形态的正例；目录内的 `plugin.wasm`
//!   是占位文件，满足步骤 6 的存在性检查。全部正例必须零错误通过；
//!   `full-example`（设计书 §4.1）与 `permissions-all-8` 还必须零警告。

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use serde::Deserialize;
use tessera_manifest::DepGraph;
use tessera_manifest::DepNode;
use tessera_manifest::HostCompat;
use tessera_manifest::PluginId;
use tessera_manifest::SemverRange;
use tessera_manifest::StrictVersion;
use tessera_manifest::ValidationContext;
use tessera_manifest::validate::validate;

/// 本 change 涉及的全部错误码（proposal「全部 E_MANIFEST_* / E_COMPAT_* /
/// E_DEPS_* 错误码的产出，每个至少一条反例 fixture」+ 冲突与虚拟目录两类）。
const EXPECTED_CODES: &[&str] = &[
    "E_MANIFEST_PARSE",
    "E_MANIFEST_SCHEMA",
    "E_MANIFEST_VERSION",
    "E_MANIFEST_MAIN",
    "E_MANIFEST_DANGLING_REF",
    "E_COMPAT_CORE",
    "E_COMPAT_API",
    "E_CONFLICT_PLUGIN_ID",
    "E_CONFLICT_CONTRIB_ID",
    "E_PERM_UNKNOWN_VDIR",
    "E_DEPS_MISSING",
    "E_DEPS_VERSION",
    "E_DEPS_CYCLE",
    "E_DEPS_UPSTREAM_FAILED",
];

/// 必须零警告的正例目录（其余 valid 目录允许警告，但必须零错误）。
const ZERO_WARNING_VALID: &[&str] = &["full-example", "permissions-all-8"];

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

/// 校验用的临时插件目录（提供 main 存在性检查所需的 plugin.wasm）。
fn temp_plugin_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("tessera-manifest-fixture-test");
    fs::create_dir_all(&dir).expect("创建临时目录失败");
    fs::write(dir.join("plugin.wasm"), b"").expect("写入占位 wasm 失败");
    dir
}

#[derive(Deserialize)]
struct GraphFixture {
    plugins: Vec<GraphPlugin>,
}

#[derive(Deserialize)]
struct GraphPlugin {
    id: PluginId,
    version: StrictVersion,
    #[serde(default)]
    dependencies: BTreeMap<PluginId, SemverRange>,
}

/// 6.4 验证：每个错误码目录存在且至少有一条 fixture——遍历式检查，
/// 新增错误码时只需更新 [`EXPECTED_CODES`]，漏配目录即失败。
#[test]
fn every_error_code_has_at_least_one_fixture() {
    for code in EXPECTED_CODES {
        let dir = fixtures_dir().join("invalid").join(code);
        assert!(dir.is_dir(), "缺少错误码目录：invalid/{code}/");
        let count = fs::read_dir(&dir)
            .unwrap_or_else(|e| panic!("读取 {code} 目录失败：{e}"))
            .filter(|entry| {
                entry
                    .as_ref()
                    .unwrap()
                    .path()
                    .extension()
                    .is_some_and(|ext| ext == "json")
            })
            .count();
        assert!(count >= 1, "错误码 {code} 目录下至少需要一条 fixture");
    }
}

/// 6.4 验证：`invalid/` 下每个 fixture 恰好触发其所在目录的错误码。
#[test]
fn invalid_fixtures_trigger_exactly_their_directory_code() {
    let invalid_root = fixtures_dir().join("invalid");
    let plugin_dir = temp_plugin_dir();
    let mut checked = 0usize;

    for code_dir in sorted_dirs(&invalid_root) {
        let code = code_dir.file_name().unwrap().to_string_lossy().to_string();
        for file in sorted_json_files(&code_dir) {
            let raw = fs::read_to_string(&file).expect("读取 fixture 失败");
            let path_label = file.display();

            if code.starts_with("E_DEPS_") {
                // 依赖图形态：断言失败表中出现目录声明的错误码
                let fixture: GraphFixture = serde_json::from_str(&raw)
                    .unwrap_or_else(|e| panic!("{path_label} 不是合法的图 fixture：{e}"));
                let nodes: Vec<DepNode> = fixture
                    .plugins
                    .into_iter()
                    .map(|p| DepNode {
                        id: p.id,
                        version: p.version.0,
                        dependencies: p.dependencies.into_iter().map(|(k, v)| (k, v.0)).collect(),
                    })
                    .collect();
                let resolution = DepGraph::from_nodes(nodes).resolve();
                assert!(
                    !resolution.failures.is_empty(),
                    "{path_label} 期望触发 {code}，实际校验通过"
                );
                assert!(
                    resolution
                        .failures
                        .values()
                        .any(|err| err.code.to_string() == code),
                    "{path_label} 期望触发 {code}，实际错误码：{}",
                    resolution
                        .failures
                        .values()
                        .map(|e| e.code.to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                );
            } else if let Ok(wrapper) = serde_json::from_str::<serde_json::Value>(&raw) {
                // wrapper 形态：错误码必须与目录名严格相等
                assert!(
                    wrapper.get("manifest").is_some(),
                    "{path_label} 缺少 manifest 键（非 DEPS 目录必须用 wrapper 形态）"
                );
                let err = validate_wrapper(&wrapper, &plugin_dir)
                    .err()
                    .unwrap_or_else(|| panic!("{path_label} 期望触发 {code}，实际校验通过"));
                assert_eq!(
                    err.code.to_string(),
                    code,
                    "{path_label} 错误码不符：{}",
                    err.user_message
                );
            } else {
                // 内容不是合法 JSON：原文即 plugin.json 输入，只应触发 E_MANIFEST_PARSE
                let result = validate(
                    &raw,
                    &plugin_dir,
                    &HostCompat::default(),
                    &ValidationContext::empty(),
                );
                let err = result.expect_err(&format!("{path_label} 期望解析失败"));
                assert_eq!(err.code.to_string(), code, "{path_label} 错误码不符");
                assert_eq!(
                    code, "E_MANIFEST_PARSE",
                    "{path_label} 不是合法 JSON 却不在 E_MANIFEST_PARSE 目录"
                );
            }
            checked += 1;
        }
    }
    assert!(
        checked >= EXPECTED_CODES.len(),
        "遍历到的 fixture 数异常：{checked}"
    );
}

/// 6.3 验证：valid/ 下每个正例零错误通过；指定目录还必须零警告。
#[test]
fn valid_fixtures_pass_validation() {
    let valid_root = fixtures_dir().join("valid");
    let plugin_dir = temp_plugin_dir();

    for dir in sorted_dirs(&valid_root) {
        let name = dir.file_name().unwrap().to_string_lossy().to_string();
        let file = dir.join("plugin.json");
        let raw = fs::read_to_string(&file)
            .unwrap_or_else(|e| panic!("正例 {name} 缺少 plugin.json：{e}"));
        let wrapper: serde_json::Value =
            serde_json::from_str(&raw).unwrap_or_else(|e| panic!("正例 {name} 不是合法 JSON：{e}"));

        let outcome = validate_wrapper(&wrapper, &plugin_dir);
        match outcome {
            Ok(validated) => {
                if ZERO_WARNING_VALID.contains(&name.as_str()) {
                    assert!(
                        validated.warnings.is_empty(),
                        "正例 {name} 必须零警告：{:?}",
                        validated
                            .warnings
                            .iter()
                            .map(|w| w.message())
                            .collect::<Vec<_>>()
                    );
                }
            }
            Err(err) => panic!("正例 {name} 必须通过校验：{}", err.user_message),
        }
    }
}

/// 用 wrapper 构造上下文并执行校验，透传校验结果。
fn validate_wrapper(
    wrapper: &serde_json::Value,
    plugin_dir: &Path,
) -> Result<tessera_manifest::ValidatedManifest, tessera_error::TesseraError> {
    let occupied_plugin_ids: BTreeSet<PluginId> = wrapper
        .get("occupiedPluginIds")
        .map(|v| serde_json::from_value(v.clone()).expect("occupiedPluginIds 解析失败"))
        .unwrap_or_default();
    let occupied_qualified_ids: BTreeSet<tessera_manifest::QualifiedId> = wrapper
        .get("occupiedQualifiedIds")
        .map(|v| serde_json::from_value(v.clone()).expect("occupiedQualifiedIds 解析失败"))
        .unwrap_or_default();
    let registered_vdirs: BTreeSet<String> = wrapper
        .get("registeredVdirs")
        .map(|v| serde_json::from_value(v.clone()).expect("registeredVdirs 解析失败"))
        .unwrap_or_default();
    let ctx = ValidationContext {
        occupied_plugin_ids: &occupied_plugin_ids,
        occupied_qualified_ids: &occupied_qualified_ids,
        registered_vdirs: &registered_vdirs,
    };
    let manifest_json = serde_json::to_string(&wrapper["manifest"]).expect("manifest 键序列化失败");
    validate(&manifest_json, plugin_dir, &HostCompat::default(), &ctx)
}

fn sorted_dirs(root: &Path) -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = fs::read_dir(root)
        .unwrap_or_else(|e| panic!("读取 {} 失败：{e}", root.display()))
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    dirs.sort();
    dirs
}

fn sorted_json_files(dir: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("读取 {} 失败：{e}", dir.display()))
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|ext| ext == "json"))
        .collect();
    files.sort();
    files
}

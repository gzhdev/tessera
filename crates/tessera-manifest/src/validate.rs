//! 校验流水线（设计书 §4.4 步骤 1–10）。
//!
//! 步骤间**快速失败**、步骤内**尽量累积**（design.md D2）：任一步失败即返回
//! 该步错误，但同一步能发现的问题一次报全。步骤 11/12（依赖存在性与环）
//! 需要全部候选清单的集合，见 [`crate::depgraph`]；跨插件的编排归
//! `core-plugin-lifecycle`（§17.1 依赖方向）。
//!
//! 步骤 7/8/10 需要「需要外部信息」（design.md D1）：以不可变集合快照作为
//! 入参（[`ValidationContext`]），保持纯函数性质。

use std::collections::BTreeSet;
use std::path::Path;

use semver::Version;
use tessera_error::codes;
use tessera_error::{ErrorCode, TesseraError};

use crate::ids::{LocalName, PluginId, QualifiedId};
use crate::manifest::{ActivationEvent, Permission, PluginManifest};

/// 宿主兼容信息（§13.1 两条版本线的宿主侧）。
#[derive(Debug, Clone)]
pub struct HostCompat {
    /// 宿主当前能力级别 `CORE_API_LEVEL`（正整数，当前为 1）。
    pub core_level: u32,
    /// 宿主提供的插件 API（WIT world）版本。当前 `0.2.0`，未发布状态（§13.2）。
    pub plugin_api: Version,
}

impl Default for HostCompat {
    fn default() -> Self {
        Self {
            core_level: 1,
            plugin_api: Version::new(0, 2, 0),
        }
    }
}

/// 「需要外部信息」的校验步骤（§4.4 步骤 7/8/10）的入参快照（design.md D1）。
#[derive(Debug)]
pub struct ValidationContext<'a> {
    /// 已加载/已占用的插件 id 集合（步骤 7）。
    pub occupied_plugin_ids: &'a BTreeSet<PluginId>,
    /// 已占用的全限定贡献项 id 集合（步骤 8）。
    pub occupied_qualified_ids: &'a BTreeSet<QualifiedId>,
    /// 宿主已注册的虚拟目录别名集合（步骤 10）。
    pub registered_vdirs: &'a BTreeSet<String>,
}

static EMPTY_PLUGIN_IDS: BTreeSet<PluginId> = BTreeSet::new();
static EMPTY_QUALIFIED_IDS: BTreeSet<QualifiedId> = BTreeSet::new();
static EMPTY_VDIRS: BTreeSet<String> = BTreeSet::new();

impl Default for ValidationContext<'_> {
    fn default() -> Self {
        Self {
            occupied_plugin_ids: &EMPTY_PLUGIN_IDS,
            occupied_qualified_ids: &EMPTY_QUALIFIED_IDS,
            registered_vdirs: &EMPTY_VDIRS,
        }
    }
}

impl ValidationContext<'_> {
    /// 空上下文（各集合均为空：首个插件安装、无已占用 id、无别名注册时的形态）。
    pub fn empty() -> Self {
        Self::default()
    }
}

/// 校验通过但值得提示的警告（不阻塞加载）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationWarning {
    /// `activationEvents` 含 `*`：立即激活，仅供开发调试（§3.4）。
    AlwaysActivate,
}

impl ValidationWarning {
    pub fn message(&self) -> String {
        match self {
            ValidationWarning::AlwaysActivate => {
                "activationEvents 含 `*`（立即激活），仅供开发调试，正式插件应移除".to_owned()
            }
        }
    }
}

/// 校验成功的产物：解析后的清单 + 不阻塞加载的警告。
#[derive(Debug, Clone)]
pub struct ValidatedManifest {
    pub manifest: PluginManifest,
    pub warnings: Vec<ValidationWarning>,
}

/// 执行 §4.4 流水线的步骤 1–10。
///
/// `json` 为 `plugin.json` 的内容，`plugin_dir` 为该清单所在插件目录
/// （步骤 6 的 `main` 存在性检查基准）。
pub fn validate(
    json: &str,
    plugin_dir: &Path,
    host: &HostCompat,
    ctx: &ValidationContext<'_>,
) -> Result<ValidatedManifest, TesseraError> {
    // 步骤 1：合法 JSON
    let value: serde_json::Value = serde_json::from_str(json).map_err(|e| {
        TesseraError::new(
            ErrorCode::Manifest(codes::ManifestCode::Parse),
            format!("插件清单不是合法的 JSON：{e}"),
        )
    })?;

    // 步骤 2：serde 反序列化（含未知字段拒绝）+ 字段规范（步骤内累积）
    let manifest: PluginManifest = serde_path_to_error::deserialize(&value).map_err(|e| {
        TesseraError::new(
            ErrorCode::Manifest(codes::ManifestCode::Schema),
            format!("清单结构不合法（{}）：{}", e.path(), e.inner()),
        )
    })?;
    let issues = manifest.field_issues();
    if !issues.is_empty() {
        return Err(TesseraError::new(
            ErrorCode::Manifest(codes::ManifestCode::Schema),
            issues.join("；"),
        ));
    }

    // 步骤 3：manifestVersion 受支持
    if manifest.manifest_version != 1 {
        return Err(TesseraError::new(
            ErrorCode::Manifest(codes::ManifestCode::Version),
            format!(
                "此清单的 manifestVersion 为 {}，当前宿主仅支持 1，请更新插件",
                manifest.manifest_version
            ),
        ));
    }

    // 步骤 4：engines.core ≤ 宿主 CORE_API_LEVEL（整数比较，design.md D4）
    if manifest.engines.core > host.core_level {
        return Err(TesseraError::new(
            ErrorCode::Compat(codes::CompatCode::Core),
            format!(
                "此插件需要宿主能力级别 ≥ {}，当前宿主为 {}，请升级应用",
                manifest.engines.core, host.core_level
            ),
        ));
    }

    // 步骤 5：engines.pluginApi 与宿主提供的 WIT world 版本匹配
    if !manifest.engines.plugin_api.0.matches(&host.plugin_api) {
        return Err(TesseraError::new(
            ErrorCode::Compat(codes::CompatCode::Api),
            format!(
                "此插件需要插件 API {}，当前宿主提供 {}，请升级应用或使用匹配版本的插件",
                manifest.engines.plugin_api, host.plugin_api
            ),
        ));
    }

    // 步骤 6：main 路径合法且文件存在（语法非法 → SCHEMA，缺失 → MAIN）
    validate_main(&manifest.main, plugin_dir)?;

    // 步骤 7：插件 id 未与已加载插件冲突
    if ctx.occupied_plugin_ids.contains(&manifest.id) {
        return Err(TesseraError::new(
            ErrorCode::Conflict(codes::ConflictCode::PluginId),
            format!("插件 id `{}` 已被加载，同一 id 不允许安装两份", manifest.id),
        ));
    }

    // 步骤 8：contributes 内所有全限定 id 无冲突（步骤内累积：本插件内重复 + 与已占用冲突）
    let conflicts = collect_contrib_id_conflicts(&manifest, ctx.occupied_qualified_ids);
    if !conflicts.is_empty() {
        return Err(TesseraError::new(
            ErrorCode::Conflict(codes::ConflictCode::ContribId),
            conflicts.join("；"),
        ));
    }

    // 步骤 9：activationEvents 引用的本插件命令/视图存在（悬空 → DANGLING_REF）
    let warnings = check_activation_events(&manifest)?;

    // 步骤 10：permissions 声明的虚拟目录别名已在宿主注册
    let unknown: Vec<String> = manifest
        .permissions
        .iter()
        .filter(|p| matches!(p, Permission::FsRead { .. } | Permission::FsWrite { .. }))
        .flat_map(|p| p.virtual_dirs().iter())
        .filter(|dir| !ctx.registered_vdirs.contains(*dir))
        .cloned()
        .collect();
    if !unknown.is_empty() {
        return Err(TesseraError::new(
            ErrorCode::Perm(codes::PermCode::UnknownVdir),
            format!(
                "权限声明引用了宿主未注册的虚拟目录别名：{}",
                unknown.join("、")
            ),
        ));
    }

    Ok(ValidatedManifest { manifest, warnings })
}

/// §4.4 步骤 6：`main` 必须以 `.wasm` 结尾、不含 `..` 分量、非绝对路径、无盘符；
/// 路径合法但文件不存在时返回 `E_MANIFEST_MAIN`。
fn validate_main(main: &str, plugin_dir: &Path) -> Result<(), TesseraError> {
    let schema_err = |msg: String| {
        TesseraError::new(
            ErrorCode::Manifest(codes::ManifestCode::Schema),
            format!("main 路径 `{main}` 非法：{msg}"),
        )
    };
    if !main.ends_with(".wasm") {
        return Err(schema_err("必须以 .wasm 结尾".into()));
    }
    let has_parent_component = main.split(['/', '\\']).any(|component| component == "..");
    if has_parent_component {
        return Err(schema_err("不得包含 `..` 分量".into()));
    }
    if main.starts_with('/') || main.starts_with('\\') || main.contains("//") {
        return Err(schema_err("必须是相对插件目录的路径".into()));
    }
    // Windows 盘符（C:\… 或 C:…）与 UNC（\\…，上面 start 判断已覆盖 \\ 前缀）
    let bytes = main.as_bytes();
    if bytes.len() >= 2 && bytes[1] == b':' {
        return Err(schema_err("不得包含盘符".into()));
    }
    if !plugin_dir.join(main).is_file() {
        return Err(TesseraError::new(
            ErrorCode::Manifest(codes::ManifestCode::Main),
            format!("main 指向的文件不存在：{}", plugin_dir.join(main).display()),
        ));
    }
    Ok(())
}

/// §4.4 步骤 8：收集贡献项全限定 id 的冲突——
/// 本插件内重复声明，以及与已占用集合的冲突，一次报全。
fn collect_contrib_id_conflicts(
    manifest: &PluginManifest,
    occupied: &BTreeSet<QualifiedId>,
) -> Vec<String> {
    let mut declared: BTreeSet<QualifiedId> = BTreeSet::new();
    let mut conflicts = Vec::new();
    let mut check = |local: &LocalName, kind: &str, declared: &mut BTreeSet<QualifiedId>| {
        let qualified = manifest.id.qualify(local);
        if !declared.insert(qualified.clone()) {
            conflicts.push(format!("`{kind}` 的 id `{local}` 在本插件内重复声明"));
        } else if occupied.contains(&qualified) {
            conflicts.push(format!(
                "`{kind}` 的全限定 id `{qualified}` 已被其他插件占用"
            ));
        }
    };
    for cmd in &manifest.contributes.commands {
        check(&cmd.id, "commands", &mut declared);
    }
    for view in &manifest.contributes.views {
        check(&view.id, "views", &mut declared);
    }
    conflicts
}

/// §4.4 步骤 9：激活事件的悬空引用检查（仅本插件内可判定的引用）。
///
/// 引用解析规则（§4.2）：`{own_id}.` 前缀 → 拆出局部名查本插件 `contributes`；
/// 不含点的单段名 → 只能是本插件局部名，查不到即悬空；带点但非本插件前缀 →
/// 视为对他插件的全限定引用，放行（跨插件校验归编排层）。
/// `*` 语法合法，产生 [`ValidationWarning::AlwaysActivate`]。
fn check_activation_events(
    manifest: &PluginManifest,
) -> Result<Vec<ValidationWarning>, TesseraError> {
    let commands: BTreeSet<&str> = manifest
        .contributes
        .commands
        .iter()
        .map(|c| c.id.as_str())
        .collect();
    let views: BTreeSet<&str> = manifest
        .contributes
        .views
        .iter()
        .map(|v| v.id.as_str())
        .collect();

    let mut dangling = Vec::new();
    let mut warnings = Vec::new();
    let own_prefix = format!("{}.", manifest.id);

    for event in &manifest.activation_events {
        let (kind, id, known) = match event {
            ActivationEvent::OnCommand { command } => ("命令", command.as_str(), &commands),
            ActivationEvent::OnView { view } => ("视图", view.as_str(), &views),
            ActivationEvent::Always => {
                warnings.push(ValidationWarning::AlwaysActivate);
                continue;
            }
            _ => continue,
        };
        // 解析为本插件局部名的三种形态；其余视为外部全限定引用，放行
        let local = if let Some(rest) = id.strip_prefix(&own_prefix) {
            Some(rest)
        } else if !id.contains('.') {
            Some(id)
        } else {
            None
        };
        if let Some(local) = local
            && !known.contains(local)
        {
            dangling.push(format!(
                "激活事件 `on{kind}:{id}` 引用的{kind}在本插件 contributes 中不存在"
            ));
        }
    }

    if dangling.is_empty() {
        Ok(warnings)
    } else {
        Err(TesseraError::new(
            ErrorCode::Manifest(codes::ManifestCode::DanglingRef),
            dangling.join("；"),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    /// 合法最小清单的 JSON 基线，测试逐字段篡改。
    fn base_json() -> serde_json::Value {
        serde_json::json!({
            "manifestVersion": 1,
            "id": "com.example.myplugin",
            "name": "My Plugin",
            "version": "1.2.0",
            "engines": { "core": 1, "pluginApi": "^0.2.0" },
            "main": "plugin.wasm",
            "activationEvents": ["onCommand:run"],
            "contributes": {
                "commands": [
                    { "id": "run", "title": "运行" }
                ]
            }
        })
    }

    /// 跑流水线的测试捷径：默认宿主与空上下文，临时目录里放一个 plugin.wasm。
    fn run(json: &serde_json::Value) -> Result<ValidatedManifest, TesseraError> {
        let dir = std::env::temp_dir().join("tessera-manifest-test");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("plugin.wasm"), b"").unwrap();
        validate(
            &serde_json::to_string(json).unwrap(),
            &dir,
            &HostCompat::default(),
            &ValidationContext::default(),
        )
    }

    fn code_of(err: &TesseraError) -> String {
        err.code.to_string()
    }

    #[test]
    fn base_manifest_passes_without_warnings() {
        let ok = run(&base_json()).expect("合法基线必须通过");
        assert!(ok.warnings.is_empty());
    }

    // ---- 步骤 1：E_MANIFEST_PARSE ----

    #[test]
    fn step1_invalid_json_returns_parse_error() {
        let err = validate(
            "{ not json ,,",
            Path::new("."),
            &HostCompat::default(),
            &ValidationContext::default(),
        )
        .unwrap_err();
        assert_eq!(code_of(&err), "E_MANIFEST_PARSE");
        assert!(err.user_message.contains("JSON"), "{}", err.user_message);
    }

    // ---- 步骤 2：E_MANIFEST_SCHEMA（未知字段与字段规范） ----

    #[test]
    fn step2_top_level_unknown_field_returns_schema_error() {
        // 2.1 验证：顶层出现未知字段时反序列化失败并返回 E_MANIFEST_SCHEMA
        let mut json = base_json();
        json["unknownTopLevel"] = "x".into();
        let err = run(&json).unwrap_err();
        assert_eq!(code_of(&err), "E_MANIFEST_SCHEMA");
    }

    #[test]
    fn step2_permission_unknown_field_reports_field_name() {
        // spec 场景：{"type": "fs.read", "virtualDirectory": "workspace"} 的精确反馈
        let mut json = base_json();
        json["permissions"] = serde_json::json!([
            { "type": "fs.read", "virtualDirectory": "workspace" }
        ]);
        let err = run(&json).unwrap_err();
        assert_eq!(code_of(&err), "E_MANIFEST_SCHEMA");
        assert!(
            err.user_message.contains("virtualDirectory"),
            "错误信息必须含未知字段名：{}",
            err.user_message
        );
    }

    #[test]
    fn step2_field_issues_accumulate() {
        // 4.4 验证（部分）：同一步骤内的多个字段问题一次报出
        let mut json = base_json();
        json["name"] = "n".repeat(65).into();
        json["description"] = "d".repeat(257).into();
        json["engines"]["core"] = 0.into();
        let err = run(&json).unwrap_err();
        assert_eq!(code_of(&err), "E_MANIFEST_SCHEMA");
        assert!(err.user_message.contains("name"), "{}", err.user_message);
        assert!(
            err.user_message.contains("description"),
            "{}",
            err.user_message
        );
        assert!(
            err.user_message.contains("engines.core"),
            "{}",
            err.user_message
        );
    }

    #[test]
    fn step2_loose_semver_rejected() {
        // 2.6 验证：version 严格 SemVer 的反例
        let mut json = base_json();
        json["version"] = "1.2".into();
        let err = run(&json).unwrap_err();
        assert_eq!(code_of(&err), "E_MANIFEST_SCHEMA");
        assert!(err.user_message.contains("1.2"), "{}", err.user_message);
    }

    #[test]
    fn step2_id_length_bound() {
        // 2.6 验证：id ≤ 128 的越界反例
        let mut json = base_json();
        json["id"] = format!("a.{}", "b".repeat(127)).into();
        let err = run(&json).unwrap_err();
        assert_eq!(code_of(&err), "E_MANIFEST_SCHEMA");
    }

    #[test]
    fn step2_uppercase_id_rejected() {
        // spec 场景：id 为 MyPlugin（含大写、无点分隔）
        let mut json = base_json();
        json["id"] = "MyPlugin".into();
        let err = run(&json).unwrap_err();
        assert_eq!(code_of(&err), "E_MANIFEST_SCHEMA");
    }

    #[test]
    fn step2_duplicate_permission_scope_rejected() {
        let mut json = base_json();
        json["permissions"] = serde_json::json!([
            { "type": "fs.read", "virtualDir": "workspace" },
            { "type": "fs.read", "virtualDir": "workspace" }
        ]);
        let err = run(&json).unwrap_err();
        assert_eq!(code_of(&err), "E_MANIFEST_SCHEMA");
        assert!(err.user_message.contains("重复"), "{}", err.user_message);
    }

    // ---- 步骤 3：E_MANIFEST_VERSION ----

    #[test]
    fn step3_unsupported_manifest_version() {
        // 3.2 验证：2 返回 E_MANIFEST_VERSION
        let mut json = base_json();
        json["manifestVersion"] = 2.into();
        let err = run(&json).unwrap_err();
        assert_eq!(code_of(&err), "E_MANIFEST_VERSION");
    }

    // ---- 步骤 4：E_COMPAT_CORE ----

    #[test]
    fn step4_core_level_too_high() {
        // 3.3 验证：插件要 2 而宿主为 1 → E_COMPAT_CORE，信息含双方数值与建议动作
        let mut json = base_json();
        json["engines"]["core"] = 2.into();
        let err = run(&json).unwrap_err();
        assert_eq!(code_of(&err), "E_COMPAT_CORE");
        let msg = err.user_message;
        assert!(msg.contains('2') && msg.contains('1'), "{msg}");
        assert!(msg.contains("升级"), "{msg}");
    }

    #[test]
    fn step4_core_level_backward_compatible() {
        // 3.3 验证：插件要 1 而宿主为 3 → 通过
        let mut json = base_json();
        json["engines"]["core"] = 1.into();
        let dir = std::env::temp_dir().join("tessera-manifest-test");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("plugin.wasm"), b"").unwrap();
        let host = HostCompat {
            core_level: 3,
            ..HostCompat::default()
        };
        validate(
            &serde_json::to_string(&json).unwrap(),
            &dir,
            &host,
            &ValidationContext::default(),
        )
        .expect("能力级别向上兼容必须通过");
    }

    // ---- 步骤 5：E_COMPAT_API ----

    #[test]
    fn step5_plugin_api_mismatch() {
        // 3.4 验证：插件要 ^0.3.0 而宿主提供 0.2.0 → E_COMPAT_API
        let mut json = base_json();
        json["engines"]["pluginApi"] = "^0.3.0".into();
        let err = run(&json).unwrap_err();
        assert_eq!(code_of(&err), "E_COMPAT_API");
        let msg = err.user_message;
        assert!(msg.contains("^0.3.0") && msg.contains("0.2.0"), "{msg}");
    }

    // ---- 步骤 6：main 路径 ----

    #[test]
    fn step6_path_escape_rejected_as_schema_error() {
        // 3.1 验证：../../evil.wasm 返回 E_MANIFEST_SCHEMA
        for bad in [
            "../../evil.wasm",
            "/abs/path.wasm",
            "C:\\x\\p.wasm",
            "plugin.wat",
        ] {
            let mut json = base_json();
            json["main"] = bad.into();
            let err = run(&json).unwrap_err();
            assert_eq!(code_of(&err), "E_MANIFEST_SCHEMA", "main = {bad}");
        }
    }

    #[test]
    fn step6_missing_file_returns_main_error() {
        // 3.1 验证：路径合法但文件不存在 → E_MANIFEST_MAIN
        let mut json = base_json();
        json["main"] = "absent.wasm".into();
        let err = run(&json).unwrap_err();
        assert_eq!(code_of(&err), "E_MANIFEST_MAIN");
    }

    // ---- 步骤 7/8：E_CONFLICT_* ----

    #[test]
    fn step7_plugin_id_conflict() {
        // 4.1 验证：id 已在集合中 → E_CONFLICT_PLUGIN_ID
        let json = base_json();
        let dir = std::env::temp_dir().join("tessera-manifest-test");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("plugin.wasm"), b"").unwrap();
        let occupied = BTreeSet::from([PluginId::parse("com.example.myplugin").unwrap()]);
        let ctx = ValidationContext {
            occupied_plugin_ids: &occupied,
            ..ValidationContext::empty()
        };
        let err = validate(
            &serde_json::to_string(&json).unwrap(),
            &dir,
            &HostCompat::default(),
            &ctx,
        )
        .unwrap_err();
        assert_eq!(code_of(&err), "E_CONFLICT_PLUGIN_ID");
    }

    #[test]
    fn step8_contrib_id_conflict() {
        // 4.2 验证：贡献项拼接后的全限定 id 已被占用 → E_CONFLICT_CONTRIB_ID
        let json = base_json();
        let dir = std::env::temp_dir().join("tessera-manifest-test");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("plugin.wasm"), b"").unwrap();
        let occupied = BTreeSet::from([PluginId::parse("com.example.myplugin")
            .unwrap()
            .qualify(&LocalName::parse("run").unwrap())]);
        let ctx = ValidationContext {
            occupied_qualified_ids: &occupied,
            ..ValidationContext::empty()
        };
        let err = validate(
            &serde_json::to_string(&json).unwrap(),
            &dir,
            &HostCompat::default(),
            &ctx,
        )
        .unwrap_err();
        assert_eq!(code_of(&err), "E_CONFLICT_CONTRIB_ID");
    }

    #[test]
    fn step8_internal_duplicate_contrib_rejected() {
        // 同一插件内重复声明同一贡献 id 也是冲突（§6.4）
        let mut json = base_json();
        json["contributes"]["commands"] = serde_json::json!([
            { "id": "run", "title": "运行" },
            { "id": "run", "title": "再运行" }
        ]);
        let err = run(&json).unwrap_err();
        assert_eq!(code_of(&err), "E_CONFLICT_CONTRIB_ID");
    }

    // ---- 步骤 9：E_MANIFEST_DANGLING_REF ----

    #[test]
    fn step9_dangling_command_reference() {
        // 3.5 验证：onCommand:doesNotExist → E_MANIFEST_DANGLING_REF
        let mut json = base_json();
        json["activationEvents"] = serde_json::json!(["onCommand:doesNotExist"]);
        let err = run(&json).unwrap_err();
        assert_eq!(code_of(&err), "E_MANIFEST_DANGLING_REF");
        assert!(
            err.user_message.contains("doesNotExist"),
            "{}",
            err.user_message
        );
    }

    #[test]
    fn step9_star_passes_with_warning() {
        // 3.5 验证：* 通过但产生警告
        let mut json = base_json();
        json["activationEvents"] = serde_json::json!(["*"]);
        let ok = run(&json).expect("`*` 必须通过校验");
        assert_eq!(ok.warnings, vec![ValidationWarning::AlwaysActivate]);
    }

    #[test]
    fn step9_external_qualified_reference_allowed() {
        // 引用其他插件的命令必须写全限定名；这里放行（跨插件校验归编排层）
        let mut json = base_json();
        json["activationEvents"] = serde_json::json!(["onCommand:com.example.other.format"]);
        run(&json).expect("外部全限定引用应放行");
    }

    #[test]
    fn step9_own_qualified_reference_resolves() {
        // 引用本插件也可写全限定名
        let mut json = base_json();
        json["activationEvents"] = serde_json::json!(["onCommand:com.example.myplugin.run"]);
        run(&json).expect("本插件全限定引用应解析成功");
    }

    #[test]
    fn step9_own_qualified_dangling_detected() {
        let mut json = base_json();
        json["activationEvents"] = serde_json::json!(["onView:com.example.myplugin.noview"]);
        let err = run(&json).unwrap_err();
        assert_eq!(code_of(&err), "E_MANIFEST_DANGLING_REF");
    }

    #[test]
    fn step9_bad_activation_syntax_rejected_as_schema() {
        // 语法不合法属于结构错误（§4.2 activationEvents 行的语法要求）
        let mut json = base_json();
        json["activationEvents"] = serde_json::json!(["onConfig:endpoint"]);
        let err = run(&json).unwrap_err();
        assert_eq!(code_of(&err), "E_MANIFEST_SCHEMA");
    }

    // ---- 步骤 10：E_PERM_UNKNOWN_VDIR ----

    #[test]
    fn step10_unknown_virtual_dir() {
        // 4.3 验证：{"type":"fs.read","virtualDir":"nonexistent"} → E_PERM_UNKNOWN_VDIR
        let dir = std::env::temp_dir().join("tessera-manifest-test");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("plugin.wasm"), b"").unwrap();
        let vdirs = BTreeSet::from(["workspace".to_owned()]);
        let ctx = ValidationContext {
            registered_vdirs: &vdirs,
            ..ValidationContext::empty()
        };

        let mut json = base_json();
        json["permissions"] = serde_json::json!([
            { "type": "fs.read", "virtualDir": "nonexistent" }
        ]);
        let err = validate(
            &serde_json::to_string(&json).unwrap(),
            &dir,
            &HostCompat::default(),
            &ctx,
        )
        .unwrap_err();
        assert_eq!(code_of(&err), "E_PERM_UNKNOWN_VDIR");
        assert!(
            err.user_message.contains("nonexistent"),
            "{}",
            err.user_message
        );

        // 对照：别名已注册时通过
        let mut ok_json = base_json();
        ok_json["permissions"] = serde_json::json!([
            { "type": "fs.read", "virtualDir": "workspace" }
        ]);
        validate(
            &serde_json::to_string(&ok_json).unwrap(),
            &dir,
            &HostCompat::default(),
            &ctx,
        )
        .expect("已注册别名必须通过");
    }

    // ---- 4.4：流水线编排语义 ----

    #[test]
    fn pipeline_reports_first_failing_step() {
        // 4.4 验证：一份同时含多种错误的清单，返回第一个失败步骤的错误。
        // 步骤 3 的 manifestVersion=2 与步骤 4 的 core=99 同时存在 → 报 VERSION。
        let mut json = base_json();
        json["manifestVersion"] = 2.into();
        json["engines"]["core"] = 99.into();
        let err = run(&json).unwrap_err();
        assert_eq!(code_of(&err), "E_MANIFEST_VERSION");
    }

    #[test]
    fn unknown_vdir_checked_after_dangling() {
        // 步骤 9 失败先于步骤 10：悬空引用 + 未知别名并存时报 DANGLING_REF
        let mut json = base_json();
        json["activationEvents"] = serde_json::json!(["onCommand:nope"]);
        json["permissions"] = serde_json::json!([
            { "type": "fs.read", "virtualDir": "nonexistent" }
        ]);
        let err = run(&json).unwrap_err();
        assert_eq!(code_of(&err), "E_MANIFEST_DANGLING_REF");
    }

    /// 设计书 §4.1 完整示例必须通过流水线步骤 1–10（依赖 §3.3 归 depgraph）。
    /// 权限中的 workspace / plugin-data 别名在上下文里视为已注册。
    #[test]
    fn full_design_example_passes() {
        let wrapper: serde_json::Value = serde_json::from_str(include_str!(
            "../tests/fixtures/valid/full-example/plugin.json"
        ))
        .expect("fixture 必须存在且是合法 JSON");
        let json = &wrapper["manifest"];
        let dir = std::env::temp_dir().join("tessera-manifest-test");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("plugin.wasm"), b"").unwrap();
        let vdirs = BTreeSet::from(["workspace".to_owned(), "plugin-data".to_owned()]);
        let ctx = ValidationContext {
            registered_vdirs: &vdirs,
            ..ValidationContext::empty()
        };
        let ok = validate(
            &serde_json::to_string(&json).unwrap(),
            &dir,
            &HostCompat::default(),
            &ctx,
        )
        .expect("§4.1 完整示例必须零错误通过");
        assert!(
            ok.warnings.is_empty(),
            "§4.1 完整示例必须零警告：{:?}",
            ok.warnings
        );
    }

    /// BTreeMap 依赖在基线中可用（覆盖 dependencies 的反序列化路径）。
    #[test]
    fn dependencies_deserialize() {
        let mut json = base_json();
        json["dependencies"] = serde_json::json!({ "com.example.other": "^1.0.0" });
        let ok = run(&json).unwrap();
        let deps: BTreeMap<String, String> = ok
            .manifest
            .dependencies
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        assert_eq!(
            deps.get("com.example.other").map(String::as_str),
            Some("^1.0.0")
        );
    }
}

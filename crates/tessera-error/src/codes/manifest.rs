//! `E_MANIFEST_*`：清单解析与校验（§12.1）。

crate::codes::reason_enum!(ManifestCode {
    Parse => "PARSE",
    Schema => "SCHEMA",
    // §12.1 示例列未列，但 §4.4 校验流水线步骤 3 与 §13.1 文案范式明确使用：
    Version => "VERSION", // 步骤 3：manifestVersion 不受支持
    Main => "MAIN",       // 步骤 6：main 路径非法或文件不存在
    DanglingRef => "DANGLING_REF",
});

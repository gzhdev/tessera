//! `E_MANIFEST_*`：清单解析与校验（§12.1）。

crate::codes::reason_enum!(ManifestCode {
    Parse => "PARSE",
    Schema => "SCHEMA",
    DanglingRef => "DANGLING_REF",
});

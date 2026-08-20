//! `E_LOAD_*`：编译与实例化（§12.1）。

crate::codes::reason_enum!(LoadCode {
    Compile => "COMPILE",
    Instantiate => "INSTANTIATE",
    MissingImport => "MISSING_IMPORT",
});

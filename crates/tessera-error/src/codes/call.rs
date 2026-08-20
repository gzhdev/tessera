//! `E_CALL_*`：跨插件调用（§12.1；CYCLE 为同链回环，DEADLOCK 为并发互等，二者不合并）。

crate::codes::reason_enum!(CallCode {
    Cycle => "CYCLE",
    Deadlock => "DEADLOCK",
    Depth => "DEPTH",
    TargetNotFound => "TARGET_NOT_FOUND",
});

//! `E_TIMEOUT_*`：超时（§12.1）。

crate::codes::reason_enum!(TimeoutCode {
    Exec => "EXEC",
    Call => "CALL",
    Deactivate => "DEACTIVATE",
});

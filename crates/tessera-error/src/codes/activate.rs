//! `E_ACTIVATE_*`：激活失败（§12.1）。

crate::codes::reason_enum!(ActivateCode {
    ReturnedErr => "RETURNED_ERR",
    Trap => "TRAP",
});

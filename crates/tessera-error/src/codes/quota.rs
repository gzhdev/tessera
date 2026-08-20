//! `E_QUOTA_*`：配额超限（§12.1）。

crate::codes::reason_enum!(QuotaCode {
    Memory => "MEMORY",
    Storage => "STORAGE",
    Response => "RESPONSE",
    Fuel => "FUEL",
});

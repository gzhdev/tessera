//! `E_CONFLICT_*`：id 冲突（§12.1）。

crate::codes::reason_enum!(ConflictCode {
    PluginId => "PLUGIN_ID",
    ContribId => "CONTRIB_ID",
});

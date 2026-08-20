//! `E_DEPS_*`：依赖问题（§12.1）。

crate::codes::reason_enum!(DepsCode {
    Missing => "MISSING",
    Version => "VERSION",
    Cycle => "CYCLE",
    UpstreamFailed => "UPSTREAM_FAILED",
});

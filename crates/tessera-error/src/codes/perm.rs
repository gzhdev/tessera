//! `E_PERM_*`：权限拒绝（§12.1；含 v0.3 新增的 INSECURE / PRIVATE_NET）。

crate::codes::reason_enum!(PermCode {
    Denied => "DENIED",
    UnknownVdir => "UNKNOWN_VDIR",
    PathEscape => "PATH_ESCAPE",
    Domain => "DOMAIN",
    Insecure => "INSECURE",
    PrivateNet => "PRIVATE_NET",
});

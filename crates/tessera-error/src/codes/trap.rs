//! `E_TRAP_*`：WASM trap（§12.1）。

crate::codes::reason_enum!(TrapCode {
    Unreachable => "UNREACHABLE",
    Oob => "OOB",
    StackOverflow => "STACK_OVERFLOW",
});

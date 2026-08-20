//! WASM 沙箱：wasmtime 封装、host 函数实现、worker 线程、资源限额。
//!
//! 所有 wasmtime 类型被封在本 crate 之内（设计书 §17.1 约束 2）。
//! 本 change 只建立骨架；实现见 `core-wasm-sandbox`。

//! 端到端往返冒烟：实例化 guest component → 调用 export → export 内部回调 import。

use std::path::PathBuf;
use std::process::Command;

use roundtrip_host::instantiate;

/// guest 产物路径：examples/toolchain-roundtrip/target/wasm32-wasip2/debug/roundtrip_guest.wasm
fn guest_wasm_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../target/wasm32-wasip2/debug/roundtrip_guest.wasm")
}

/// 若 guest 尚未构建（例如直接 `cargo test -p roundtrip-host`），先构建一次。
fn ensure_guest_built() -> PathBuf {
    let path = guest_wasm_path();
    if !path.exists() {
        let status = Command::new(env!("CARGO"))
            .args(["build", "-p", "roundtrip-guest", "--target", "wasm32-wasip2"])
            .current_dir(env!("CARGO_MANIFEST_DIR").to_string() + "/..")
            .status()
            .expect("无法启动 cargo 构建 guest");
        assert!(status.success(), "guest 构建失败");
    }
    path
}

#[test]
fn roundtrip_calls_back_into_host() {
    let wasm = std::fs::read(ensure_guest_built()).expect("读取 guest wasm 失败");
    let engine = roundtrip_host::engine();

    let (mut store, bindings) = instantiate(&engine, &wasm, None).expect("实例化失败");

    let result = bindings
        .call_plugin_run(&mut store, "tessera")
        .expect("调用 plugin-run 失败");

    assert_eq!(result, "plugin(tessera) <- host[tessera]");
    // 宿主 import 确实被插件回调到
    assert_eq!(store.data().host_calls(), 1);
    assert_eq!(store.data().last_arg().as_deref(), Some("tessera"));
}

/// §8.2 实测：对该 component 而言 `StoreLimits::instances` 的最小可行值。
/// 从 1 开始逐档试探，首个「实例化 + 调用 + 回调」全链路成功的档位即为最小值；
/// 循环结构保证该档位之前的所有档位都失败（否则会更早 break）。
#[test]
fn store_limits_instances_probe() {
    let wasm = std::fs::read(ensure_guest_built()).expect("读取 guest wasm 失败");
    let engine = roundtrip_host::engine();

    let mut minimal: Option<usize> = None;
    for n in 1..=8 {
        let ok = match instantiate(&engine, &wasm, Some(n)) {
            Ok((mut store, bindings)) => {
                bindings
                    .call_plugin_run(&mut store, "probe")
                    .map(|_| ())
                    .is_ok()
            }
            Err(_) => false,
        };
        if ok {
            minimal = Some(n);
            break;
        }
    }
    let n = minimal.expect("instances 上限 8 仍无法完成实例化与调用");
    eprintln!("StoreLimits::instances 最小可行值 = {n}（更小档位均失败）");
    assert!((1..=8).contains(&n));
}

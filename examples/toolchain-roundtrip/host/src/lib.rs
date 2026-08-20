//! 工具链冒烟 host：wasmtime 实例化 roundtrip component，
//! 调用其 export，并在 export 内部回调宿主 import。

use std::cell::{Cell, RefCell};

use wasmtime::component::{bindgen, Component, Linker, ResourceTable};
use wasmtime::{Config, Engine, Store, StoreLimits, StoreLimitsBuilder};
use wasmtime_wasi::{WasiCtx, WasiCtxView, WasiView};

bindgen!({
    path: "../wit",
    world: "roundtrip",
});

/// 宿主状态：承载 WASI 上下文与限额，实现 world 的 import，并记录被插件回调的痕迹。
pub struct HostState {
    wasi: WasiCtx,
    table: ResourceTable,
    limits: StoreLimits,
    host_calls: Cell<u32>,
    last_arg: RefCell<Option<String>>,
}

impl HostState {
    pub fn new(limits: StoreLimits) -> Self {
        let wasi = WasiCtx::builder().build();
        Self {
            wasi,
            table: ResourceTable::new(),
            limits,
            host_calls: Cell::new(0),
            last_arg: RefCell::new(None),
        }
    }

    pub fn host_calls(&self) -> u32 {
        self.host_calls.get()
    }

    pub fn last_arg(&self) -> Option<String> {
        self.last_arg.borrow().clone()
    }
}

impl WasiView for HostState {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.wasi,
            table: &mut self.table,
        }
    }
}

impl RoundtripImports for HostState {
    fn host_tag(&mut self, name: String) -> String {
        self.host_calls.set(self.host_calls.get() + 1);
        self.last_arg.replace(Some(name.clone()));
        format!("host[{name}]")
    }
}

/// 用与设计书 §8.1 一致的基础配置构建 Engine（component model、同步模型）。
/// 注：wasmtime 47 中 `async_support(false)` 已废弃——同步是默认行为。
pub fn engine() -> Engine {
    let mut config = Config::new();
    config.wasm_component_model(true);
    Engine::new(&config).expect("构建 wasmtime Engine 失败")
}

/// 实例化 guest component 并返回导出绑定。
/// `instances_limit` 用于 §8.2 的 StoreLimits::instances 实测（None 表示不设限）。
pub fn instantiate(
    engine: &Engine,
    wasm: &[u8],
    instances_limit: Option<usize>,
) -> anyhow::Result<(Store<HostState>, Roundtrip)> {
    let mut store = Store::new(
        engine,
        HostState::new(match instances_limit {
            Some(n) => StoreLimitsBuilder::new().instances(n).build(),
            None => StoreLimitsBuilder::new().build(),
        }),
    );
    store.limiter(|state| &mut state.limits);

    let component = Component::new(engine, wasm)?;
    let mut linker = Linker::new(engine);
    wasmtime_wasi::p2::add_to_linker_sync(&mut linker)?;
    Roundtrip::add_to_linker::<HostState, wasmtime::component::HasSelf<HostState>>(
        &mut linker,
        |state| state,
    )?;
    let bindings = Roundtrip::instantiate(&mut store, &component, &mut linker)?;
    Ok((store, bindings))
}

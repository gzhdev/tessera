//! 工具链冒烟 guest：实现 `plugin-run` export，内部回调宿主的 `host-tag` import。

wit_bindgen::generate!({ path: "../wit" });

struct Component;

export!(Component);

impl Guest for Component {
    fn plugin_run(input: String) -> String {
        let tagged = host_tag(&input);
        format!("plugin({input}) <- {tagged}")
    }
}

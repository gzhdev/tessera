//! 插件日志限速（spec observability Requirement 3）：单插件每秒 100 条，
//! 超出丢弃；每次进入丢弃状态记一条警告。按插件独立分桶，不波及其他插件与宿主。
//!
//! 实现为 per-layer `Filter` 而非在 host-log 入口判断：宿主为插件写的日志
//! 同样带插件标识、同样可能被失控插件放大，只有 Layer 层能一处覆盖全部路径（design.md D4）。

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, UNIX_EPOCH};

use tracing::field::{Field, Visit};
use tracing::{Event, Metadata, Subscriber};
use tracing_subscriber::layer::{Context, Filter};

use super::PLUGIN_ID_FIELD;
use super::rotate::{Clock, DailyRotatingFile};

/// 单插件每秒日志条数上限（默认 100，§12.2）。
pub const DEFAULT_RATE_LIMIT: u32 = 100;
/// 限速窗口长度。
pub const WINDOW: Duration = Duration::from_secs(1);

/// 每插件一个桶：固定 1 秒窗口计数。
#[derive(Default)]
struct Bucket {
    window_start_secs: u64,
    count: u32,
    dropping: bool,
}

enum Decision {
    Allow,
    Deny,
    /// 拒绝，且这是本窗口第一次进入丢弃状态——需要记录一条警告。
    DenyAndWarn,
}

/// 按插件独立限速的事件过滤器。事件无 `plugin_id` 字段（宿主自身日志）时恒放行。
///
/// 丢弃警告**旁路直写**日志文件而不走 `tracing::warn!`：per-layer filter 的分发
/// 依赖 thread-local FilterState 两阶段协议，在过滤器回调内再发事件会破坏该状态机
/// （tracing-subscriber 已知约束）。旁路写入的警告不可能被限速自身丢弃。
pub struct PluginLogRateLimit {
    buckets: Mutex<HashMap<String, Bucket>>,
    limit: u32,
    clock: Clock,
    sink: Option<DailyRotatingFile>,
}

impl PluginLogRateLimit {
    pub fn new() -> Self {
        Self::with_clock(super::rotate::system_clock())
    }

    pub fn with_clock(clock: Clock) -> Self {
        Self {
            buckets: Mutex::new(HashMap::new()),
            limit: DEFAULT_RATE_LIMIT,
            clock,
            sink: None,
        }
    }

    /// 设置旁路写入目标（与 fmt 层同一个轮转文件）。
    pub fn with_sink(mut self, sink: DailyRotatingFile) -> Self {
        self.sink = Some(sink);
        self
    }

    fn now_secs(&self) -> u64 {
        (self.clock)()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }

    fn decide(&self, plugin_id: &str) -> Decision {
        let now = self.now_secs();
        let mut buckets = self.buckets.lock().expect("限速器锁中毒");
        let bucket = buckets.entry(plugin_id.to_string()).or_default();
        if bucket.window_start_secs != now {
            bucket.window_start_secs = now;
            bucket.count = 0;
            bucket.dropping = false;
        }
        bucket.count += 1;
        if bucket.count <= self.limit {
            Decision::Allow
        } else if bucket.dropping {
            Decision::Deny
        } else {
            bucket.dropping = true;
            Decision::DenyAndWarn
        }
    }
}

impl Default for PluginLogRateLimit {
    fn default() -> Self {
        Self::new()
    }
}

impl<S: Subscriber> Filter<S> for PluginLogRateLimit {
    fn enabled(&self, _metadata: &Metadata<'_>, _cx: &Context<'_, S>) -> bool {
        // 字段值要到事件级才可见，全部交给 event_enabled 判定
        true
    }

    fn event_enabled(&self, event: &Event<'_>, _cx: &Context<'_, S>) -> bool {
        let Some(plugin_id) = extract_plugin_id(event) else {
            return true; // 宿主自身日志不限速
        };
        match self.decide(&plugin_id) {
            Decision::Allow => true,
            Decision::Deny => false,
            Decision::DenyAndWarn => {
                self.write_drop_warning(&plugin_id);
                false
            }
        }
    }
}

impl PluginLogRateLimit {
    /// 丢弃警告以与 fmt 层同构的 JSON 行旁路写入。
    fn write_drop_warning(&self, plugin_id: &str) {
        let Some(sink) = &self.sink else { return };
        let line = format!(
            "{{\"timestamp\":\"{}\",\"level\":\"WARN\",\"target\":\"tessera::observability\",\"fields\":{{\"rate_limited_plugin\":\"{}\",\"limit\":{},\"message\":\"插件日志超过每秒 {} 条，超出条目将被丢弃\"}}}}\n",
            sink.iso_now(),
            plugin_id.escape_default(),
            self.limit,
            self.limit
        );
        sink.write_raw(line.as_bytes());
    }
}

/// 从事件的 `plugin_id` 字段取值。
fn extract_plugin_id(event: &Event<'_>) -> Option<String> {
    struct PluginIdGatherer(Option<String>);

    impl Visit for PluginIdGatherer {
        fn record_str(&mut self, field: &Field, value: &str) {
            if field.name() == PLUGIN_ID_FIELD {
                self.0 = Some(value.to_string());
            }
        }

        fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
            if field.name() == PLUGIN_ID_FIELD {
                self.0 = Some(format!("{value:?}"));
            }
        }
    }

    let mut gatherer = PluginIdGatherer(None);
    event.record(&mut gatherer);
    gatherer.0
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 桶逻辑：窗口内第 100 条放行、第 101 条首次拒绝并要求警告、其后静默拒绝；新窗口重置。
    #[test]
    fn bucket_transitions() {
        let limit = 3; // 用小限额验证状态机，默认值见 DEFAULT_RATE_LIMIT
        let limiter = PluginLogRateLimit {
            buckets: Mutex::new(HashMap::new()),
            limit,
            clock: std::sync::Arc::new(|| UNIX_EPOCH + Duration::from_secs(1000)),
            sink: None,
        };

        for _ in 0..3 {
            assert!(matches!(limiter.decide("p"), Decision::Allow));
        }
        assert!(matches!(limiter.decide("p"), Decision::DenyAndWarn));
        assert!(matches!(limiter.decide("p"), Decision::Deny));

        // 下一秒窗口重置
        let limiter2 = PluginLogRateLimit {
            buckets: {
                let mut m = HashMap::new();
                m.insert(
                    "p".to_string(),
                    Bucket {
                        window_start_secs: 999,
                        count: limit,
                        dropping: true,
                    },
                );
                Mutex::new(m)
            },
            limit,
            clock: std::sync::Arc::new(|| UNIX_EPOCH + Duration::from_secs(1000)),
            sink: None,
        };
        assert!(matches!(limiter2.decide("p"), Decision::Allow));
    }
}

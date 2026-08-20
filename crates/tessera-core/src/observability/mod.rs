//! 可观测性基座（设计书 §12.2，`specs/observability`）。
//!
//! - 日志输出到应用数据目录下的 `{dir}/tessera-{YYYY-MM-DD}.log`，按天轮转、保留 7 天
//! - 与插件相关的日志强制带 `plugin_id`（span 约定 + 事件字段），来源可区分（host/plugin）
//! - 单条 8 KB 截断；单插件每秒 100 条限速，按插件独立
//! - 无任何网络出口（默认不上传日志，§16）

pub mod limits;
pub mod rotate;

use std::path::Path;

use tracing::Level;
use tracing_subscriber::prelude::*;

pub use limits::PluginLogRateLimit;
pub use rotate::DailyRotatingFile;

/// 插件标识字段名：span 与事件上都用它，仅凭它即可筛出某插件的全部条目。
pub const PLUGIN_ID_FIELD: &str = "plugin_id";
/// 来源标记字段名：区分宿主写入与插件写入。
pub const LOG_SOURCE_FIELD: &str = "log_source";

/// 日志来源：宿主自身 / 插件经 host-log 写入。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogSource {
    Host,
    Plugin,
}

impl LogSource {
    pub const fn as_str(&self) -> &'static str {
        match self {
            LogSource::Host => "host",
            LogSource::Plugin => "plugin",
        }
    }
}

/// 写一条与插件相关的日志（§12.2：强制带 plugin_id；来源可区分）。
///
/// - 插件经 `host-log.log` 写入 → `source = LogSource::Plugin`
/// - 宿主为插件记录（权限拒绝、生命周期事件等）→ `source = LogSource::Host`
///
/// 事件同时携带 `plugin_id` 字段（供限速与筛选）并处于 `plugin` span 之中（供上下文追踪）。
pub fn plugin_log(level: Level, plugin_id: &str, source: LogSource, message: &str) {
    let span = tracing::info_span!("plugin", plugin_id = plugin_id);
    let _guard = span.enter();
    // event! 的 callsite 元数据要求编译期常量 level，运行时 level 按 5 级分发
    macro_rules! emit {
        ($($lvl:tt)+) => {
            tracing::event!(
                $($lvl)+,
                plugin_id = plugin_id,
                log_source = source.as_str(),
                message = message
            )
        };
    }
    match level {
        Level::TRACE => emit!(Level::TRACE),
        Level::DEBUG => emit!(Level::DEBUG),
        Level::INFO => emit!(Level::INFO),
        Level::WARN => emit!(Level::WARN),
        Level::ERROR => emit!(Level::ERROR),
    }
}

/// 组装日志订阅者：JSON 格式 + 按天轮转写入器 + 插件限速过滤。
/// `clock` 仅测试注入用；`init_global` 走系统时钟。
pub fn build_subscriber(
    log_dir: &Path,
    clock: Option<rotate::Clock>,
) -> std::io::Result<Box<dyn tracing::Subscriber + Send + Sync>> {
    let clock = clock.unwrap_or_else(rotate::system_clock);
    let writer = DailyRotatingFile::with_clock(log_dir, clock.clone())?;
    let fmt_layer = tracing_subscriber::fmt::layer()
        .json()
        .with_current_span(true)
        .with_span_list(true)
        .with_writer(writer.clone())
        .with_filter(PluginLogRateLimit::with_clock(clock).with_sink(writer));
    Ok(Box::new(tracing_subscriber::registry().with(fmt_layer)))
}

/// 进程级初始化（由外壳在启动时调用，任务 6.1）。
pub fn init_global(log_dir: &Path) -> std::io::Result<()> {
    let subscriber = build_subscriber(log_dir, None)?;
    tracing::subscriber::set_global_default(subscriber)
        .map_err(|e| std::io::Error::other(e.to_string()))?;
    tracing::info!(target: "tessera::observability", log_dir = %log_dir.display(), "日志系统初始化完成");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    fn fixed_clock(secs: u64) -> rotate::Clock {
        std::sync::Arc::new(move || std::time::UNIX_EPOCH + std::time::Duration::from_secs(secs))
    }

    /// 读取目录下全部日志行，解析为 JSON。
    fn read_json_entries(dir: &Path) -> Vec<serde_json::Value> {
        let mut entries = Vec::new();
        for name in std::fs::read_dir(dir).unwrap().flatten() {
            let path = name.path();
            if path.extension().and_then(|e| e.to_str()) != Some("log") {
                continue;
            }
            let mut content = String::new();
            std::fs::File::open(&path)
                .unwrap()
                .read_to_string(&mut content)
                .unwrap();
            for line in content.lines() {
                if line.is_empty() {
                    continue;
                }
                entries.push(serde_json::from_str(line).expect("日志行应是合法 JSON"));
            }
        }
        entries
    }

    /// 事件字段都在 JSON 的 "fields" 对象里（tracing-subscriber JSON 格式）。
    fn field<'v>(entry: &'v serde_json::Value, name: &str) -> Option<&'v str> {
        entry.get("fields")?.get(name)?.as_str()
    }

    /// 5.2 验证：混合写入后，仅凭插件标识可筛出该插件全部条目，且两种来源可区分。
    #[test]
    fn filter_by_plugin_id_and_distinguish_source() {
        let dir = tempfile::tempdir().unwrap();
        let subscriber = build_subscriber(dir.path(), Some(fixed_clock(1_800_000_000))).unwrap();

        tracing::subscriber::with_default(subscriber, || {
            tracing::info!("宿主自身的日志");
            plugin_log(Level::INFO, "plugin-a", LogSource::Plugin, "插件写的");
            plugin_log(Level::WARN, "plugin-a", LogSource::Host, "宿主为插件记的");
            plugin_log(Level::INFO, "plugin-b", LogSource::Plugin, "另一插件");
        });

        let entries = read_json_entries(dir.path());
        // plugin-a 相关条目（宿主为其记的 + 它自己写的）都能筛出来
        let about_a: Vec<_> = entries
            .iter()
            .filter(|e| field(e, PLUGIN_ID_FIELD) == Some("plugin-a"))
            .collect();
        assert_eq!(about_a.len(), 2, "plugin-a 的两种来源条目都应可筛出");
        let sources: Vec<&str> = about_a
            .iter()
            .map(|e| field(e, LOG_SOURCE_FIELD).expect("来源字段"))
            .collect();
        assert!(
            sources.contains(&"plugin") && sources.contains(&"host"),
            "{sources:?}"
        );

        // 宿主自身条目不带 plugin_id，不会混入
        assert_eq!(
            entries
                .iter()
                .filter(|e| field(e, PLUGIN_ID_FIELD).is_none())
                .count(),
            1
        );

        // span 约定：插件条目处于带 plugin_id 的 plugin span 中
        for entry in &about_a {
            let spans_json =
                serde_json::to_string(entry.get("spans").unwrap_or(&serde_json::Value::Null))
                    .unwrap();
            assert!(
                spans_json.contains("plugin-a"),
                "插件条目的 span 应携带 plugin_id：{spans_json}"
            );
        }
    }

    /// 5.3 验证（限速部分）：一秒内 10000 条时写入不超过 100 条，且产生丢弃警告。
    #[test]
    fn rate_limit_drops_storm_and_warns() {
        let dir = tempfile::tempdir().unwrap();
        let subscriber = build_subscriber(dir.path(), Some(fixed_clock(1_800_000_000))).unwrap();

        tracing::subscriber::with_default(subscriber, || {
            for i in 0..10_000 {
                plugin_log(Level::INFO, "storm", LogSource::Plugin, "风暴");
                let _ = i;
            }
        });

        let entries = read_json_entries(dir.path());
        let written: Vec<_> = entries
            .iter()
            .filter(|e| field(e, PLUGIN_ID_FIELD) == Some("storm"))
            .collect();
        assert!(written.len() <= 100, "实际写入 {} 条", written.len());

        let warnings: Vec<_> = entries
            .iter()
            .filter(|e| field(e, "rate_limited_plugin") == Some("storm"))
            .collect();
        assert!(!warnings.is_empty(), "应记录丢弃警告");
    }

    /// 5.4 验证：插件 A 触发限速时，插件 B 与宿主自身的日志不被削减。
    #[test]
    fn rate_limit_is_per_plugin() {
        let dir = tempfile::tempdir().unwrap();
        let subscriber = build_subscriber(dir.path(), Some(fixed_clock(1_800_000_000))).unwrap();

        tracing::subscriber::with_default(subscriber, || {
            for _ in 0..150 {
                plugin_log(Level::INFO, "plugin-a", LogSource::Plugin, "a");
            }
            for _ in 0..80 {
                plugin_log(Level::INFO, "plugin-b", LogSource::Plugin, "b");
                tracing::info!("宿主自身");
            }
        });

        let entries = read_json_entries(dir.path());
        let count = |id: &str| {
            entries
                .iter()
                .filter(|e| field(e, PLUGIN_ID_FIELD) == Some(id))
                .count()
        };
        assert_eq!(count("plugin-a"), 100, "A 应恰好写到上限");
        assert_eq!(count("plugin-b"), 80, "B 不受 A 限速影响");
        let host_count = entries
            .iter()
            .filter(|e| {
                field(e, PLUGIN_ID_FIELD).is_none() && field(e, "rate_limited_plugin").is_none()
            })
            .count();
        assert_eq!(host_count, 80, "宿主自身日志不受插件限速影响");
    }

    /// 5.5 验证（测试断言部分）：日志子系统的直接依赖不含任何网络客户端。
    /// （依赖树层面的检查见 scripts/check-log-no-network.sh，两者互补。）
    #[test]
    fn no_network_client_in_direct_deps() {
        const BANNED: &[&str] = &[
            "reqwest",
            "ureq",
            "curl",
            "isahc",
            "surf",
            "hyper",
            "attohttpc",
            "minreq",
        ];
        let toml = include_str!("../../Cargo.toml");
        let in_deps = toml
            .split("[dependencies]")
            .nth(1)
            .map(|section| section.split('[').next().unwrap_or(""))
            .unwrap_or("");
        for line in in_deps.lines() {
            let dep = line.split('=').next().unwrap_or("").trim();
            if dep.is_empty() {
                continue;
            }
            assert!(
                !BANNED.contains(&dep),
                "日志子系统所在 crate 出现网络客户端依赖：{dep}"
            );
        }
    }
}

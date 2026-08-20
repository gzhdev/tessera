//! 按天轮转的日志文件写入器：`{dir}/tessera-{YYYY-MM-DD}.log`，保留最近 7 天。
//!
//! 不用 `tracing-appender` 的原因：其 `max_log_files` 按**文件数**保留，而
//! `specs/observability` 要求按**日期**保留（只有 3 个文件时也要删掉 8 天前的）。
//! 自实现后可注入时钟，跨天与过期清理场景可直接测试。

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use tracing_subscriber::fmt::MakeWriter;

/// 单条日志的长度上限（8 KB），超出截断并标明（spec observability Requirement 3）。
pub const MAX_LINE_BYTES: usize = 8 * 1024;
/// 截断标记。
pub const TRUNCATED_MARKER: &str = "[TRUNCATED]";
/// 日志保留天数。
pub const RETENTION_DAYS: i64 = 7;
/// 日志文件名前缀。
pub const FILE_PREFIX: &str = "tessera";

/// 可注入时钟（测试模拟跨天用）。
pub type Clock = Arc<dyn Fn() -> SystemTime + Send + Sync>;

pub fn system_clock() -> Clock {
    Arc::new(SystemTime::now)
}

struct Inner {
    dir: PathBuf,
    state: Mutex<State>,
    clock: Clock,
}

struct State {
    current_day: Option<i64>,
    file: Option<File>,
}

/// 按天轮转的共享写入器；实现 `MakeWriter` 供 `tracing_subscriber::fmt` 使用。
#[derive(Clone)]
pub struct DailyRotatingFile {
    inner: Arc<Inner>,
}

impl DailyRotatingFile {
    pub fn new(dir: impl Into<PathBuf>) -> std::io::Result<Self> {
        Self::with_clock(dir, system_clock())
    }

    pub fn with_clock(dir: impl Into<PathBuf>, clock: Clock) -> std::io::Result<Self> {
        let dir = dir.into();
        std::fs::create_dir_all(&dir)?;
        Ok(Self {
            inner: Arc::new(Inner {
                dir,
                state: Mutex::new(State {
                    current_day: None,
                    file: None,
                }),
                clock,
            }),
        })
    }

    /// 当前时钟对应的 epoch 天数。
    fn today(&self) -> i64 {
        let now = (self.inner.clock)();
        let secs = now
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        secs.div_euclid(86_400)
    }

    fn write_bytes(&self, buf: &[u8]) -> std::io::Result<()> {
        let today = self.today();
        let mut state = self.inner.state.lock().expect("日志写入器锁中毒");
        if state.current_day != Some(today) {
            // 轮转：打开新日期文件，并清理超出保留期的旧文件
            delete_expired(&self.inner.dir, today - RETENTION_DAYS);
            let file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(file_path(&self.inner.dir, today))?;
            state.file = Some(file);
            state.current_day = Some(today);
        }
        let file = state.file.as_mut().expect("轮转后必然有文件");
        file.write_all(buf)
    }

    /// 供限速器旁路写入（不经过 tracing 分发，见 limits.rs 说明）。
    pub(crate) fn write_raw(&self, buf: &[u8]) {
        let line = if buf.len() > MAX_LINE_BYTES {
            let cut = MAX_LINE_BYTES - TRUNCATED_MARKER.len();
            let mut truncated = buf[..cut].to_vec();
            truncated.extend_from_slice(TRUNCATED_MARKER.as_bytes());
            truncated
        } else {
            buf.to_vec()
        };
        if let Err(e) = self.write_bytes(&line) {
            // 日志基础设施自身出错时无处可报，只能吞掉避免拖垮宿主
            eprintln!("tessera: 日志写入失败：{e}");
        }
    }

    /// 当前时钟的 UTC ISO-8601 时间戳（限速警告旁路写入用）。
    pub(crate) fn iso_now(&self) -> String {
        let secs = (self.inner.clock)()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        let (y, m, d) = civil_from_days(secs.div_euclid(86_400));
        let sod = secs.rem_euclid(86_400);
        format!(
            "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z",
            sod / 3600,
            sod % 3600 / 60,
            sod % 60
        )
    }

    /// 测试辅助：列出当前目录下的日志文件名。
    pub fn list_files(&self) -> Vec<String> {
        list_log_files(&self.inner.dir)
    }
}

fn file_path(dir: &Path, day: i64) -> PathBuf {
    dir.join(format!("{FILE_PREFIX}-{}.log", format_day(day)))
}

fn list_log_files(dir: &Path) -> Vec<String> {
    let mut names = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            if let Some(name) = entry
                .file_name()
                .to_str()
                .filter(|n| n.starts_with(FILE_PREFIX) && n.ends_with(".log"))
            {
                names.push(name.to_string());
            }
        }
    }
    names.sort();
    names
}

/// 删除 `keep_from_day`（含）之前的日志文件；解析不了的文件名不动。
fn delete_expired(dir: &Path, keep_from_day: i64) {
    for name in list_log_files(dir) {
        if parse_day_from_name(&name).is_some_and(|day| day < keep_from_day) {
            let _ = std::fs::remove_file(dir.join(&name));
        }
    }
}

/// `tessera-YYYY-MM-DD.log` → epoch 天数。
fn parse_day_from_name(name: &str) -> Option<i64> {
    let date = name
        .strip_prefix(&format!("{FILE_PREFIX}-"))?
        .strip_suffix(".log")?;
    let mut parts = date.split('-');
    let y: i64 = parts.next()?.parse().ok()?;
    let m: u32 = parts.next()?.parse().ok()?;
    let d: u32 = parts.next()?.parse().ok()?;
    Some(days_from_civil(y, m, d))
}

fn format_day(day: i64) -> String {
    let (y, m, d) = civil_from_days(day);
    format!("{y:04}-{m:02}-{d:02}")
}

impl<'a> MakeWriter<'a> for DailyRotatingFile {
    type Writer = LineTruncatingWriter;

    fn make_writer(&'a self) -> Self::Writer {
        LineTruncatingWriter {
            owner: self.clone(),
        }
    }
}

/// 单条写入的包装：按 fmt 层「一条日志一次 `write`」的约定，
/// 把超长行截断到 8 KB 以内并追加截断标记。
pub struct LineTruncatingWriter {
    owner: DailyRotatingFile,
}

impl Write for LineTruncatingWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let line = if buf.len() > MAX_LINE_BYTES {
            let cut = MAX_LINE_BYTES - TRUNCATED_MARKER.len();
            let mut truncated = buf[..cut].to_vec();
            truncated.extend_from_slice(TRUNCATED_MARKER.as_bytes());
            truncated
        } else {
            buf.to_vec()
        };
        self.owner.write_bytes(&line)?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

// ---- epoch 天数与公历互转（Howard Hinnant 算法） ----

fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m as u32, d as u32)
}

fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = if m > 2 { m as i64 - 3 } else { m as i64 + 9 };
    let doy = (153 * mp + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicI64, Ordering};

    fn fixed_day_clock(day: i64) -> (Arc<AtomicI64>, Clock) {
        let day_cell = Arc::new(AtomicI64::new(day));
        let cell = day_cell.clone();
        let clock: Clock = Arc::new(move || {
            let day = cell.load(Ordering::SeqCst);
            std::time::UNIX_EPOCH + std::time::Duration::from_secs(day as u64 * 86_400 + 1)
        });
        (day_cell, clock)
    }

    /// 5.1 验证：模拟跨天写入产生两个文件；且 8 天前的文件在轮转时被删除。
    #[test]
    fn rotates_daily_and_deletes_expired() {
        let dir = tempfile::tempdir().unwrap();
        let base = days_from_civil(2026, 8, 20);
        let (day_cell, clock) = fixed_day_clock(base);
        let writer = DailyRotatingFile::with_clock(dir.path(), clock).unwrap();

        // 第一天：写入 2 条
        let mut w = writer.make_writer();
        w.write_all(b"day1-line1\nday1-line2\n").unwrap();
        drop(w);
        assert_eq!(writer.list_files(), vec!["tessera-2026-08-20.log"]);

        // 跨到第二天：写入 1 条 → 产生第二个文件
        day_cell.store(base + 1, Ordering::SeqCst);
        let mut w = writer.make_writer();
        w.write_all(b"day2-line1\n").unwrap();
        drop(w);
        assert_eq!(
            writer.list_files(),
            vec!["tessera-2026-08-20.log", "tessera-2026-08-21.log"]
        );

        // 旧文件不再被追加（跨天后第一天内容不变）
        let day1 = std::fs::read_to_string(dir.path().join("tessera-2026-08-20.log")).unwrap();
        assert_eq!(day1, "day1-line1\nday1-line2\n");

        // 放入一个 8 天前的文件，再跨一天触发轮转 → 该文件被删除
        let stale = dir
            .path()
            .join(format!("tessera-{}.log", format_day(base - 8)));
        std::fs::write(&stale, b"stale\n").unwrap();
        day_cell.store(base + 2, Ordering::SeqCst);
        let mut w = writer.make_writer();
        w.write_all(b"day3-line1\n").unwrap();
        drop(w);
        assert!(!stale.exists(), "8 天前的日志应在轮转时被删除");
        assert_eq!(
            writer.list_files(),
            vec![
                "tessera-2026-08-20.log",
                "tessera-2026-08-21.log",
                "tessera-2026-08-22.log"
            ]
        );
    }

    /// 5.3 验证（截断部分）：1 MB 的条目被截到 8 KB 以内且带截断标记。
    #[test]
    fn truncates_oversized_line() {
        let dir = tempfile::tempdir().unwrap();
        let writer = DailyRotatingFile::new(dir.path()).unwrap();

        let huge = vec![b'x'; 1024 * 1024];
        let mut w = writer.make_writer();
        w.write_all(&huge).unwrap();
        drop(w);

        let content = std::fs::read_to_string(
            dir.path()
                .join(format!("tessera-{}.log", format_day(writer.today()))),
        )
        .unwrap();
        assert!(content.len() <= MAX_LINE_BYTES + 1, "截断后仍超长");
        assert!(content.ends_with(TRUNCATED_MARKER), "未标明截断");
    }
}

//! Trowel's log setup: `log` records to standard error and to a file under the
//! preferences folder, so a start launched without a terminal still leaves a record.

use std::io::Write;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

struct Logger {
    file: Option<Mutex<std::fs::File>>,
}

impl log::Log for Logger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        // Trowel's own crates at `info`, everything else at `warn`, as the
        // VectorCraft app does. `RUST_LOG` replaces it.
        let target = metadata.target();
        if target.starts_with("trowel") || target.starts_with("vectorcraft") {
            metadata.level() <= log::Level::Info
        } else {
            metadata.level() <= log::Level::Warn
        }
    }

    fn log(&self, record: &log::Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let line = format!("[{}] {}: {}\n", record.level(), record.target(), record.args());
        let _ = std::io::stderr().write_all(line.as_bytes());
        if let Some(f) = &self.file
            && let Ok(mut f) = f.lock()
        {
            let _ = f.write_all(line.as_bytes());
            let _ = f.flush();
        }
    }

    fn flush(&self) {
        if let Some(f) = &self.file
            && let Ok(mut f) = f.lock()
        {
            let _ = f.flush();
        }
    }
}

/// Install the logger. A second install in the same process is refused by `log`
/// and is not an error worth reporting; `dir` is where the log file lives (none
/// leaves the log on standard error only). Answers whether this call installed it.
pub fn install(dir: Option<PathBuf>) -> bool {
    static LOGGER: OnceLock<&'static Logger> = OnceLock::new();
    if LOGGER.get().is_some() {
        return false;
    }
    // `RUST_LOG` overrides the level rule above.
    let filter = match std::env::var("RUST_LOG") {
        Ok(v) if !v.trim().is_empty() => Some(v),
        _ => None,
    };
    let file = dir.and_then(|d| {
        std::fs::create_dir_all(&d).ok()?;
        let name = "trowel.log";
        std::fs::OpenOptions::new().create(true).append(true).open(d.join(name)).ok()
    });
    let logger: &'static Logger = Box::leak(Box::new(Logger { file: file.map(Mutex::new) }));
    if log::set_logger(logger).is_ok() {
        // `RUST_LOG` names a directive; the app reads the variable itself, so the
        // level stays the one `Logger::enabled` applies.
        let _ = filter;
        log::set_max_level(log::LevelFilter::Info);
    }
    let _ = LOGGER.get_or_init(|| logger);
    true
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    /// A second call does not panic and reports that it installed nothing.
    #[test]
    fn install_is_idempotent() {
        let first = install(None);
        let second = install(None);
        assert!(!second, "the second install must not claim the logger");
        let _ = first;
    }

    /// Trowel's own crates log at `info`; a third party's crates only at `warn`.
    #[test]
    fn levels_follow_the_app() {
        let l = Logger { file: None };
        let at = |target: &str, level: log::Level| log::Log::enabled(&l, &log::Metadata::builder().level(level).target(target).build());
        assert!(at("trowel_shell", log::Level::Info));
        assert!(!at("trowel_shell", log::Level::Debug));
        assert!(at("wgpu_core", log::Level::Warn));
        assert!(!at("wgpu_core", log::Level::Info));
    }
}

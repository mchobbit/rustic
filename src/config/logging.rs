use std::{path::PathBuf, sync::OnceLock};

use anyhow::Result;
use clap::{Parser, ValueHint};
use conflate::Merge;
use jiff::tz::TimeZone;
use log::{LevelFilter, debug, warn};
use log4rs::{
    Handle,
    append::{
        console::{ConsoleAppender, Target},
        file::FileAppender,
    },
    config::{Appender, Config, Logger, Root},
    encode::pattern::PatternEncoder,
    filter::threshold::ThresholdFilter,
};
use serde::{Deserialize, Serialize};
use serde_with::{DisplayFromStr, serde_as};

use crate::config::progress_options::multi_progress;

/// Logging Config
#[serde_as]
#[derive(Default, Debug, Parser, Clone, Deserialize, Serialize, Merge)]
#[serde(default, rename_all = "kebab-case", deny_unknown_fields)]
pub struct LoggingOptions {
    /// Use this log level [default: info]
    #[clap(long, global = true, env = "RUSTIC_LOG_LEVEL",
        value_parser(["off", "error", "warn", "info", "debug", "trace"]))]
    #[serde_as(as = "Option<DisplayFromStr>")]
    #[merge(strategy=conflate::option::overwrite_none)]
    pub log_level: Option<String>,

    /// Use this log level for the log file [default: info]
    #[clap(long, global = true, env = "RUSTIC_LOG_LEVEL_LOGFILE",
        value_parser(["off", "error", "warn", "info", "debug", "trace"]))]
    #[merge(strategy=conflate::option::overwrite_none)]
    pub log_level_logfile: Option<String>,

    /// Use this log level in dry-run mode [default: info]
    #[clap(long, global = true, env = "RUSTIC_LOG_LEVEL_DRYRUN",
        value_parser(["off", "error", "warn", "info", "debug", "trace"]))]
    #[merge(strategy=conflate::option::overwrite_none)]
    pub log_level_dryrun: Option<String>,

    /// Use this log level for dependencies [default: warn]
    #[clap(long, global = true, env = "RUSTIC_LOG_LEVEL_DEPENDENCIES",
        value_parser(["off", "error", "warn", "info", "debug", "trace"]))]
    #[merge(strategy=conflate::option::overwrite_none)]
    pub log_level_dependencies: Option<String>,

    /// Write log messages to the given file (using log-level-logfile)
    #[clap(long, global = true, env = "RUSTIC_LOG_FILE", value_name = "LOGFILE", value_hint = ValueHint::FilePath)]
    #[merge(strategy=conflate::option::overwrite_none)]
    pub log_file: Option<PathBuf>,
}

impl LoggingOptions {
    pub fn config(&self, dry_run: bool) -> Result<Config> {
        let log_level = if dry_run {
            &self.log_level_dryrun
        } else {
            &self.log_level
        };

        let level_filter = log_level
            .as_ref()
            .map_or(LevelFilter::Info, |l| l.parse().unwrap());
        let level_filter_logfile = self
            .log_level_logfile
            .as_ref()
            .map_or(LevelFilter::Info, |l| l.parse().unwrap());
        let level_filter_dependencies = self
            .log_level_dependencies
            .as_ref()
            .map_or(LevelFilter::Warn, |l| l.parse().unwrap());

        let stdout = ConsoleAppender::builder()
            .target(Target::Stderr)
            .encoder(Box::new(PatternEncoder::new("{h([{l}])} {m}{n}")))
            .build();
        let stdout = PbPauseAppender(stdout);

        let mut root_builder = Root::builder().appender("stdout");
        let mut config_builder = Config::builder().appender(
            Appender::builder()
                .filter(Box::new(ThresholdFilter::new(level_filter)))
                .build("stdout", Box::new(stdout)),
        );

        if let Some(file) = &self.log_file {
            let file_appender = FileAppender::builder()
                .encoder(Box::new(PatternEncoder::new("{d} [{l}] - {m}{n}")))
                .build(file)?;
            root_builder = root_builder.appender("logfile");
            config_builder = config_builder.appender(
                Appender::builder()
                    .filter(Box::new(ThresholdFilter::new(level_filter_logfile)))
                    .build("logfile", Box::new(file_appender)),
            );
        }

        // BackOtter: jiff warns each time it looks for the system's time
        // zone and finds none, hundreds of times in a backup;
        // `report_time_zone` says it once. Its log is there when asked for.
        let level_filter_jiff = if level_filter_dependencies > LevelFilter::Warn {
            level_filter_dependencies
        } else {
            level_filter_dependencies.min(LevelFilter::Error)
        };

        let root = root_builder.build(level_filter_dependencies);
        let config = config_builder
            .logger(Logger::builder().build("rustic_rs", LevelFilter::Trace))
            .logger(Logger::builder().build("rustic_core", LevelFilter::Trace))
            .logger(Logger::builder().build("rustic_backend", LevelFilter::Trace))
            .logger(Logger::builder().build(JIFF, level_filter_jiff))
            .build(root)?;
        Ok(config)
    }

    pub fn start_logger(&self, dry_run: bool) -> Result<()> {
        static HANDLE: OnceLock<Handle> = OnceLock::new();

        let config = self.config(dry_run)?;
        if let Some(handle) = HANDLE.get() {
            handle.set_config(config);
        } else {
            let handle = log4rs::init_config(config)?;
            _ = HANDLE.set(handle);
        }
        Ok(())
    }
}

// BackOtter: time zones.

/// jiff's log, whose warnings are all about time zones: each time
/// `TimeZone::system` finds none and falls back to UTC (`Etc/Unknown`), and
/// once when there is no time zone database (`/usr/share/zoneinfo`).
const JIFF: &str = "jiff";

/// Where Unix keeps the system's time zone, when it has one.
const LOCALTIME: &str = "/etc/localtime";

/// Says once that times are in UTC for want of a time zone, which jiff would
/// warn about each time it looks for one (silenced in
/// [`LoggingOptions::config`]). A system without one, `TZ` unset and no
/// `/etc/localtime` (FreeBSD installs without it, as a rescue system may
/// run), is no news, only for the debug log: its own tools use UTC then
/// too. Nor is an `/etc/localtime` that links to nothing (Alpine without
/// tzdata links it to `/usr/share/zoneinfo/UTC`), with which they use UTC
/// as well, and BackOtter's restic says nothing. A `TZ`, or an
/// `/etc/localtime` there is, that jiff can't read is warned about.
pub fn report_time_zone() {
    let Err(err) = TimeZone::try_system() else {
        return;
    };
    if let Some(tz) = std::env::var_os("TZ") {
        warn!("TZ={tz:?} not understood, times are in UTC: {err}");
    } else if std::fs::metadata(LOCALTIME).is_ok() {
        warn!("{LOCALTIME} not understood, times are in UTC: {err}");
    } else {
        debug!("no system time zone, times are in UTC: {err}");
    }
}

/// A wrapper around [`ConsoleAppender`] that suspends the progress bar when writing logs.
#[derive(Debug)]
struct PbPauseAppender(ConsoleAppender);

impl log4rs::append::Append for PbPauseAppender {
    fn append(&self, record: &log::Record<'_>) -> Result<()> {
        multi_progress().suspend(|| self.0.append(record))
    }

    fn flush(&self) {
        // as of log4rs 1.4.0, <ConsoleAppender as Append>::flush does nothing,
        // so we do not need to pause the progress bar here. In the future,
        // if log4rs changes this behavior, we might need to add a suspend here.
        // But that's not necessary right now, so we just call flush directly
        // to avoid unnecessary suspends.
        self.0.flush();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn jiff_level(options: &LoggingOptions) -> LevelFilter {
        let config = options.config(false).unwrap();
        config
            .loggers()
            .iter()
            .find(|logger| logger.name() == JIFF)
            .unwrap()
            .level()
    }

    #[test]
    fn jiffs_time_zone_warnings_silenced() {
        assert_eq!(jiff_level(&LoggingOptions::default()), LevelFilter::Error);
        let options = LoggingOptions {
            log_level_dependencies: Some("off".to_string()),
            ..Default::default()
        };
        assert_eq!(jiff_level(&options), LevelFilter::Off);
        let options = LoggingOptions {
            log_level_dependencies: Some("info".to_string()),
            ..Default::default()
        };
        assert_eq!(jiff_level(&options), LevelFilter::Info);
    }
}

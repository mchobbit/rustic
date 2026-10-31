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

        let root = root_builder.build(level_filter_dependencies);
        let config = config_builder
            .logger(Logger::builder().build("rustic_rs", LevelFilter::Trace))
            .logger(Logger::builder().build("rustic_core", LevelFilter::Trace))
            .logger(Logger::builder().build("rustic_backend", LevelFilter::Trace))
            // BackOtter: jiff warns each time it looks for the system's time
            // zone and finds none, hundreds of times in a backup;
            // `report_time_zone` says it once.
            .logger(Logger::builder().build(
                JIFF_SYSTEM_TIME_ZONE,
                level_filter_dependencies.min(LevelFilter::Error),
            ))
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

/// The module of jiff's `TimeZone::system`, which warns each time it finds
/// no time zone and falls back to UTC (`Etc/Unknown`).
const JIFF_SYSTEM_TIME_ZONE: &str = "jiff::tz::timezone";

/// Says once that times are in UTC for want of a time zone, which jiff would
/// warn about each time it looks for one (silenced in
/// [`LoggingOptions::config`]). With `TZ` unset that is no news, only for
/// the debug log: FreeBSD, among others, installs without `/etc/localtime`,
/// as a rescue system may run, and its own tools use UTC then too. A `TZ`
/// that jiff doesn't understand is warned about.
pub fn report_time_zone() {
    let Err(err) = TimeZone::try_system() else {
        return;
    };
    match std::env::var_os("TZ") {
        Some(tz) => warn!("TZ={tz:?} not understood, times are in UTC: {err}"),
        None => debug!("no system time zone, times are in UTC: {err}"),
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
            .find(|logger| logger.name() == JIFF_SYSTEM_TIME_ZONE)
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
            log_level_dependencies: Some("trace".to_string()),
            ..Default::default()
        };
        assert_eq!(jiff_level(&options), LevelFilter::Error);
    }
}

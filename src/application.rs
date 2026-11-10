//! Rustic Abscissa Application
use std::{
    env, process,
    sync::atomic::{AtomicBool, Ordering},
};

use abscissa_core::{
    Application, Component, FrameworkError, FrameworkErrorKind, Shutdown, StandardPaths,
    application::{self, AppCell, fatal_error},
    config::{self, CfgCell},
    terminal::component::Terminal,
};

use anyhow::Result;

// use crate::helpers::*;
use crate::{commands::EntryPoint, config::RusticConfig};

/// Application state
pub static RUSTIC_APP: AppCell<RusticApp> = AppCell::new();

/// BackOtter: set by the first shutdown under `--strict-exit`, so that an
/// interrupt racing the main thread's own shutdown can't run both the after
/// and the failed hooks: the first caller's hooks and exit code win.
static SHUTTING_DOWN: AtomicBool = AtomicBool::new(false);

/// BackOtter: set by a command that an interrupt ends as it is meant to end
/// (`mount`, `webdav`: "press Ctrl-C to cancel"), before it blocks.
static ENDED_BY_INTERRUPT: AtomicBool = AtomicBool::new(false);

/// BackOtter: set under `--strict-exit` once an interrupt is taken as a
/// failure, before its failed hooks run, so that the main thread runs no
/// command-level hooks after it ([`halt_if_interrupted`]).
static INTERRUPTED: AtomicBool = AtomicBool::new(false);

/// BackOtter: the running command is one an interrupt ends as it is meant
/// to: under `--strict-exit` too, the interrupt then exits 0 and runs the
/// after hooks, as BackOtter's restic's `mount` exits 0 on one.
pub(crate) fn ended_by_interrupt() {
    ENDED_BY_INTERRUPT.store(true, Ordering::SeqCst);
}

/// BackOtter: under `--strict-exit`, once an interrupt is taken as a
/// failure, the main thread stops here for good (the interrupt's thread
/// runs the failed hooks and exits 130), so that no command-level hooks run
/// after the interrupt, as BackOtter's restic runs none of its own after a
/// cancel. Without the option this does nothing.
pub(crate) fn halt_if_interrupted() {
    if INTERRUPTED.load(Ordering::SeqCst) {
        loop {
            std::thread::park();
        }
    }
}

/// BackOtter: how an interrupt (SIGINT, SIGTERM, SIGHUP) shuts rustic down:
/// gracefully with exit status 0, as stock rustic does, or, under
/// `--strict-exit`, as a failure with exit status 130, as BackOtter's restic
/// does, so that the failed hooks run, not the after hooks; but gracefully
/// for a command an interrupt `ends` as it is meant to ([`ended_by_interrupt`]).
pub(crate) const fn interrupt_shutdown(strict: bool, ends: bool) -> (Shutdown, i32) {
    if strict && !ends {
        (Shutdown::Crash, 130)
    } else {
        (Shutdown::Graceful, 0)
    }
}

/// BackOtter: shuts rustic down for an interrupt ([`interrupt_shutdown`]).
pub(crate) fn shutdown_on_interrupt(strict: bool) -> ! {
    let (shutdown, code) = interrupt_shutdown(strict, ENDED_BY_INTERRUPT.load(Ordering::SeqCst));
    if matches!(shutdown, Shutdown::Crash) {
        INTERRUPTED.store(true, Ordering::SeqCst);
    }
    RUSTIC_APP.shutdown_with_exitcode(shutdown, code)
}

// Constants
pub mod constants {
    pub const RUSTIC_DOCS_URL: &str = "https://rustic.cli.rs/docs";
    pub const RUSTIC_DEV_DOCS_URL: &str = "https://rustic.cli.rs/dev-docs";
    pub const RUSTIC_CONFIG_DOCS_URL: &str =
        "https://github.com/rustic-rs/rustic/blob/main/config/README.md";
}

/// Rustic Application
#[derive(Debug)]
pub struct RusticApp {
    /// Application configuration.
    config: CfgCell<RusticConfig>,

    /// Application state.
    state: application::State<Self>,
}

/// Initialize a new application instance.
///
/// By default no configuration is loaded, and the framework state is
/// initialized to a default, empty state (no components, threads, etc).
impl Default for RusticApp {
    fn default() -> Self {
        Self {
            config: CfgCell::default(),
            state: application::State::default(),
        }
    }
}

impl Application for RusticApp {
    /// Entrypoint command for this application.
    type Cmd = EntryPoint;

    /// Application configuration.
    type Cfg = RusticConfig;

    /// Paths to resources within the application.
    type Paths = StandardPaths;

    /// Accessor for application configuration.
    fn config(&self) -> config::Reader<RusticConfig> {
        self.config.read()
    }

    /// Borrow the application state immutably.
    fn state(&self) -> &application::State<Self> {
        &self.state
    }

    /// Returns the framework components used by this application.
    fn framework_components(
        &mut self,
        command: &Self::Cmd,
    ) -> Result<Vec<Box<dyn Component<Self>>>, FrameworkError> {
        // we only use the terminal component
        let terminal = Terminal::new(self.term_colors(command));

        Ok(vec![Box::new(terminal)])
    }

    /// Register all components used by this application.
    ///
    /// If you would like to add additional components to your application
    /// beyond the default ones provided by the framework, this is the place
    /// to do so.
    fn register_components(&mut self, command: &Self::Cmd) -> Result<(), FrameworkError> {
        let framework_components = self.framework_components(command)?;
        let mut app_components = self.state.components_mut();
        app_components.register(framework_components)
    }

    /// Post-configuration lifecycle callback.
    ///
    /// Called regardless of whether config is loaded to indicate this is the
    /// time in app lifecycle when configuration would be loaded if
    /// possible.
    fn after_config(&mut self, config: Self::Cfg) -> Result<(), FrameworkError> {
        // Configure components
        self.state.components_mut().after_config(&config)?;

        // set all given environment variables
        for (env, value) in &config.global.env {
            unsafe {
                env::set_var(env, value);
            }
        }

        let global_hooks = config.global.hooks.clone();
        self.config.set_once(config);

        global_hooks.run_before().map_err(|err| -> FrameworkError {
            FrameworkErrorKind::ProcessError.context(err).into()
        })?;

        Ok(())
    }

    /// Shut down this application gracefully
    fn shutdown(&self, shutdown: Shutdown) -> ! {
        let exit_code = match shutdown {
            Shutdown::Crash => 1,
            _ => 0,
        };
        self.shutdown_with_exitcode(shutdown, exit_code)
    }

    /// Shut down this application gracefully, exiting with given exit code.
    fn shutdown_with_exitcode(&self, shutdown: Shutdown, exit_code: i32) -> ! {
        // BackOtter: under `--strict-exit`, only the first shutdown runs its
        // hooks and exits; a second one (an interrupt during the first) waits
        // for the process to end.
        if RUSTIC_APP.config().global.strict_exit && SHUTTING_DOWN.swap(true, Ordering::SeqCst) {
            loop {
                std::thread::park();
            }
        }
        let hooks = &RUSTIC_APP.config().global.hooks;
        match shutdown {
            Shutdown::Crash => _ = hooks.run_failed(),
            _ => _ = hooks.run_after(),
        };
        _ = hooks.run_finally();
        let result = self.state().components().shutdown(self, shutdown);
        if let Err(e) = result {
            fatal_error(self, &e)
        }

        process::exit(exit_code);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interrupt_shutdown_is_graceful_unless_strict() {
        assert!(matches!(interrupt_shutdown(false, false), (Shutdown::Graceful, 0)));
        assert!(matches!(interrupt_shutdown(true, false), (Shutdown::Crash, 130)));
        // mount, webdav: an interrupt is how they end.
        assert!(matches!(interrupt_shutdown(false, true), (Shutdown::Graceful, 0)));
        assert!(matches!(interrupt_shutdown(true, true), (Shutdown::Graceful, 0)));
    }
}

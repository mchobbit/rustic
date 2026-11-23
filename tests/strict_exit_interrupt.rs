//! BackOtter: under `--strict-exit`, once an interrupt is taken as a failure
//! the main thread starts no command-level hook command and no command
//! (ForkAudit1 G9-01). Runs rustic as a subprocess and sends it, and it
//! alone, SIGTERM while a repository `run-before` hook runs.
#![cfg(unix)]

use std::{
    fs,
    path::Path,
    process::{Command, Stdio},
    thread::sleep,
    time::{Duration, Instant},
};

use rustic_testing::TestResult;
use tempfile::tempdir;

/// The hooks write their names to `log` in rustic's working directory. The
/// first repository `run-before` command goes on for two seconds after the
/// interrupt and then exits with `first_exit`; the global failed hook keeps
/// the interrupt's thread busy for four, so that whatever the main thread
/// still does in between is seen.
fn profile(first_exit: u8) -> String {
    format!(
        r#"[global.hooks]
run-after = ["sh -c 'echo global-after >> log'"]
run-failed = ["sh -c 'echo global-failed >> log; sleep 4'"]
run-finally = ["sh -c 'echo global-finally >> log'"]

[repository.hooks]
run-before = [
  "sh -c 'echo repo-before >> log; sleep 2; exit {first_exit}'",
  "sh -c 'echo repo-before-2 >> log'",
]
run-after = ["sh -c 'echo repo-after >> log'"]
run-failed = ["sh -c 'echo repo-failed >> log'"]
run-finally = ["sh -c 'echo repo-finally >> log'"]
"#
    )
}

fn rustic(dir: &Path) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_rustic"));
    _ = cmd
        .current_dir(dir)
        .env_remove("RUSTIC_STRICT_EXIT")
        .arg("-r")
        .arg(dir.join("repo"))
        .args(["--password", "test", "--no-progress"]);
    cmd
}

/// Runs `repoinfo` with the hooks of [`profile`], sends rustic SIGTERM once
/// the first repository `run-before` command has started, and gives its
/// exit code, the hooks' log and whether `repoinfo` itself ran.
fn interrupted_repoinfo(first_exit: u8, strict: bool) -> TestResult<(Option<i32>, String, bool)> {
    let dir = tempdir()?;
    let dir = dir.path();
    assert!(rustic(dir).arg("init").output()?.status.success());
    fs::write(dir.join("hooks.toml"), profile(first_exit))?;

    let mut cmd = rustic(dir);
    if strict {
        _ = cmd.env("RUSTIC_STRICT_EXIT", "true");
    }
    let child = cmd
        .arg("-P")
        .arg(dir.join("hooks"))
        .arg("repoinfo")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let log = dir.join("log");
    let started = Instant::now();
    while !fs::read_to_string(&log).is_ok_and(|log| log.contains("repo-before")) {
        assert!(
            started.elapsed() < Duration::from_secs(60),
            "the run-before hook never started"
        );
        sleep(Duration::from_millis(20));
    }
    // To rustic alone: the hook's command goes on, as it does when a service
    // manager or `kill` ends rustic.
    assert!(
        Command::new("kill")
            .args(["-TERM", &child.id().to_string()])
            .status()?
            .success()
    );

    let output = child.wait_with_output()?;
    let ran = String::from_utf8_lossy(&output.stdout).contains("Total Size");
    Ok((output.status.code(), fs::read_to_string(&log)?, ran))
}

/// The hook returns Ok two seconds after the interrupt: the rest of its
/// list, the command and the repository's other hooks must not start.
#[test]
fn test_interrupt_in_a_before_hook_starts_nothing_more() -> TestResult<()> {
    assert_eq!(
        interrupted_repoinfo(0, true)?,
        (
            Some(130),
            "repo-before\nglobal-failed\nglobal-finally\n".to_string(),
            false
        )
    );
    Ok(())
}

/// The hook fails after the interrupt (as one a terminal's Ctrl-C kills
/// does): the repository's failed and finally hooks must not run beside the
/// global ones.
#[test]
fn test_interrupt_and_a_failing_before_hook_run_no_repository_hooks() -> TestResult<()> {
    assert_eq!(
        interrupted_repoinfo(1, true)?,
        (
            Some(130),
            "repo-before\nglobal-failed\nglobal-finally\n".to_string(),
            false
        )
    );
    Ok(())
}

/// Without `--strict-exit` an interrupt is stock rustic's: the global after
/// and finally hooks, and exit 0 at once.
#[test]
fn test_interrupt_without_strict_exit_is_graceful() -> TestResult<()> {
    assert_eq!(
        interrupted_repoinfo(0, false)?,
        (
            Some(0),
            "repo-before\nglobal-after\nglobal-finally\n".to_string(),
            false
        )
    );
    Ok(())
}

//! Rustic Integration Test for Backups and Restore
//!
//! Runs the application as a subprocess and asserts its
//! output for the `init`, `backup`, `restore`, `check`,
//! and `snapshots` command
//!
//! You can run them with 'nextest':
//! `cargo nextest run -E 'test(backup)'`.

#[cfg(unix)]
use std::os::unix::fs::symlink;

use dircmp::Comparison;
use tempfile::{TempDir, tempdir};

use assert_cmd::Command;
use predicates::prelude::{PredicateBooleanExt, predicate};

mod repositories;
use repositories::src_snapshot;

use rustic_testing::TestResult;

pub fn rustic_runner(temp_dir: &TempDir) -> TestResult<Command> {
    let password = "test";
    let repo_dir = temp_dir.path().join("repo");

    let mut runner = Command::new(env!("CARGO_BIN_EXE_rustic"));

    runner
        .arg("-r")
        .arg(repo_dir)
        .arg("--password")
        .arg(password)
        .arg("--no-progress");

    Ok(runner)
}

fn setup() -> TestResult<TempDir> {
    let temp_dir = tempdir()?;
    rustic_runner(&temp_dir)?
        .args(["init"])
        .assert()
        .success()
        .stderr(predicate::str::contains("successfully created."))
        .stderr(predicate::str::contains("successfully added."));

    Ok(temp_dir)
}

#[test]
fn test_backup_and_check_passes() -> TestResult<()> {
    let temp_dir = setup()?;
    let backup = src_snapshot()?.into_path();
    // BackOtter: saved as "/src", so the snapshot keeps no metadata of the
    // real directories above the source (/tmp, which tests running alongside
    // change); the second backup then adds nothing however /tmp changes.
    let as_path = "/src";

    {
        // Run `backup` for the first time
        rustic_runner(&temp_dir)?
            .arg("backup")
            .arg(backup.path())
            .args(["--as-path", as_path])
            .assert()
            .success()
            .stderr(predicate::str::contains("successfully saved."));
    }

    {
        // Run `snapshots`
        rustic_runner(&temp_dir)?
            .arg("snapshots")
            .assert()
            .success()
            .stdout(predicate::str::contains("total: 1 snapshot(s)"));
    }

    {
        // Run `backup` a second time
        rustic_runner(&temp_dir)?
            .arg("backup")
            .arg(backup.path())
            .args(["--as-path", as_path])
            .assert()
            .success()
            .stderr(predicate::str::contains("Added to the repo: 0 B"))
            .stderr(predicate::str::contains("successfully saved."));
    }

    {
        // Run `snapshots` a second time
        rustic_runner(&temp_dir)?
            .arg("snapshots")
            .assert()
            .success()
            .stdout(predicate::str::contains("total: 2 snapshot(s)"));
    }

    {
        // Run `check --read-data`
        rustic_runner(&temp_dir)?
            .args(["check", "--read-data"])
            .assert()
            .success()
            .stderr(predicate::str::contains("WARN").not())
            .stderr(predicate::str::contains("ERROR").not());
    }

    Ok(())
}

#[cfg(unix)]
#[test]
fn diff_reports_unchanged_symlinks_as_identical() -> TestResult<()> {
    let temp_dir = setup()?;
    let source = temp_dir.path().join("source");
    std::fs::create_dir(&source)?;
    std::fs::write(source.join("target.txt"), "target")?;
    symlink("target.txt", source.join("link.txt"))?;

    rustic_runner(&temp_dir)?
        .arg("backup")
        .arg(&source)
        .assert()
        .success();

    std::fs::write(source.join("added.txt"), "added")?;

    rustic_runner(&temp_dir)?
        .arg("backup")
        .arg(&source)
        .assert()
        .success();

    let output = rustic_runner(&temp_dir)?
        .args(["diff", "latest~1", "latest"])
        .output()?;

    assert!(
        output.status.success(),
        "diff command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout)?;
    assert!(
        stdout.contains("Symlinks: 1 =, 0 +, 0 -, 0 M, 0 U"),
        "unexpected diff output: {stdout}"
    );
    assert!(
        !stdout.contains("link.txt"),
        "unchanged symlink was reported as changed: {stdout}"
    );

    Ok(())
}

#[test]
fn test_backup_records_cli_version_in_snapshot() -> TestResult<()> {
    let temp_dir = setup()?;
    let backup = src_snapshot()?.into_path();

    let version_output = Command::new(env!("CARGO_BIN_EXE_rustic"))
        .arg("--version")
        .output()?;
    assert!(version_output.status.success());
    let version = String::from_utf8(version_output.stdout)?;

    let backup_output = rustic_runner(&temp_dir)?
        .args(["backup", "--json"])
        .arg(backup.path())
        .output()?;
    assert!(backup_output.status.success());
    let snapshot: serde_json::Value = serde_json::from_slice(&backup_output.stdout)?;

    assert_eq!(snapshot["program_version"].as_str(), Some(version.trim()));

    Ok(())
}

#[test]
fn test_backup_and_restore_passes() -> TestResult<()> {
    let temp_dir = setup()?;
    let restore_dir = temp_dir.path().join("restore");
    let backup_files = src_snapshot()?.into_path();

    {
        // Run `backup` for the first time
        rustic_runner(&temp_dir)?
            .arg("backup")
            .arg(backup_files.path())
            .arg("--as-path")
            .arg("/")
            .assert()
            .success()
            .stderr(predicate::str::contains("successfully saved."));
    }
    {
        // Run `restore`
        rustic_runner(&temp_dir)?
            .arg("restore")
            .arg("latest")
            .arg(&restore_dir)
            .assert()
            .success()
            .stdout(predicate::str::contains("restore done"));
    }

    // Compare the backup and the restored directory
    let compare_result = Comparison::default().compare(backup_files.path(), &restore_dir)?;

    // no differences
    assert!(compare_result.is_empty());

    let dump_tar_file = restore_dir.join("test.tar");
    {
        // Run `dump`
        rustic_runner(&temp_dir)?
            .arg("dump")
            .arg("latest")
            .arg("--file")
            .arg(&dump_tar_file)
            .assert()
            .success();
    }
    // TODO: compare dump output with fixture

    Ok(())
}

/// BackOtter: a backup that leaves out a file it may not read saves its
/// snapshot either way, and exits 0, as stock rustic does, or, with
/// `--strict-exit` (or `RUSTIC_STRICT_EXIT`), 3, as BackOtter's restic does.
/// Not as root, who reads it.
#[test]
#[cfg(unix)]
fn test_backup_incomplete_exits_3_with_strict_exit() -> TestResult<()> {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};

    let temp_dir = setup()?;
    let source = tempdir()?;
    std::fs::write(source.path().join("ok"), "ok")?;
    let secret = source.path().join("secret");
    std::fs::write(&secret, "secret")?;
    if std::fs::metadata(&secret)?.uid() == 0 {
        return Ok(());
    }
    std::fs::set_permissions(&secret, std::fs::Permissions::from_mode(0o000))?;

    let backup = |strict: Option<&str>, env: bool| -> TestResult<i32> {
        let mut runner = rustic_runner(&temp_dir)?;
        _ = runner.env_remove("RUSTIC_STRICT_EXIT");
        if env {
            _ = runner.env("RUSTIC_STRICT_EXIT", "true");
        }
        _ = runner.arg("backup");
        if let Some(flag) = strict {
            _ = runner.arg(flag);
        }
        let output = runner.arg(source.path()).output()?;
        Ok(output.status.code().unwrap_or(-1))
    };
    let codes = (
        backup(None, false),
        backup(Some("--strict-exit"), false),
        backup(None, true),
    );
    std::fs::set_permissions(&secret, std::fs::Permissions::from_mode(0o644))?;
    assert_eq!((codes.0?, codes.1?, codes.2?), (0, 3, 3));

    rustic_runner(&temp_dir)?
        .arg("snapshots")
        .assert()
        .success()
        .stdout(predicate::str::contains("total: 3 snapshot(s)"));
    Ok(())
}

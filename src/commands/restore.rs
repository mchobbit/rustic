//! `restore` subcommand

use crate::{
    Application, RUSTIC_APP, helpers::bytes_size_to_string, repository::IndexedRepo, status_err,
};

use abscissa_core::{Command, Runnable, Shutdown};
use anyhow::Result;
use log::{debug, info};

use std::path::PathBuf;

use rustic_core::{
    LocalDestination, LsOptions, RestoreOptions, SnapshotOrigin, repofile::Metadata,
    restore_target_is_fresh,
};

use crate::filtering::SnapshotFilter;

/// `restore` subcommand
#[allow(clippy::struct_excessive_bools)]
#[derive(clap::Parser, Command, Debug)]
pub(crate) struct RestoreCmd {
    /// Snapshot/path to restore
    ///
    /// Snapshot can be identified the following ways: "01a2b3c4" or "latest" or "latest~N" (N >= 0)
    #[clap(value_name = "SNAPSHOT[:PATH]")]
    snap: String,

    /// Restore destination
    #[clap(value_name = "DESTINATION")]
    dest: String,

    /// Restore options
    #[clap(flatten)]
    opts: RestoreOptions,

    /// List options
    #[clap(flatten)]
    ls_opts: LsOptions,

    /// Snapshot filter options (when using latest)
    #[clap(
        flatten,
        next_help_heading = "Snapshot filter options (when using latest)"
    )]
    filter: SnapshotFilter,
}
impl Runnable for RestoreCmd {
    fn run(&self) {
        if let Err(err) = RUSTIC_APP
            .config()
            .repository
            .run_indexed(|repo| self.inner_run(repo))
        {
            status_err!("{}", err);
            RUSTIC_APP.shutdown(Shutdown::Crash);
        };
    }
}

impl RestoreCmd {
    fn inner_run(&self, repo: IndexedRepo) -> Result<()> {
        let config = RUSTIC_APP.config();
        let dry_run = config.global.dry_run;

        // BackOtter: the snapshot itself too, whose program version says
        // what a node's missing attributes mean (SnapshotOrigin).
        let (id, path) = self.snap.split_once(':').unwrap_or((&self.snap, ""));
        let snap = repo.get_snapshot_from_str(id, |sn| config.snapshot_filter.matches(sn))?;
        let node = repo.node_from_snapshot_and_path(&snap, path)?;
        let opts = self.opts.origin(SnapshotOrigin::of(&snap));

        // for restore, always recurse into tree
        let mut ls_opts = self.ls_opts.clone();
        ls_opts.recursive = true;
        // BackOtter: restoring "ID:subfolder" into a destination that the
        // restore creates, or that is empty (`lost+found` alone counts as
        // empty: a new file system's root), gives the destination the
        // subfolder's own metadata, as everything below it gets its own:
        // the subfolder's node goes first, as the destination itself (an
        // empty path). Without it a restored file system's root keeps
        // mkfs's mode and has no SELinux label. A destination with anything
        // in it stays as it is (owner, mode, times, extended attributes,
        // ACLs, flags), as in stock rustic: /tmp keeps its sticky bit. A
        // whole snapshot has no node of its own (empty metadata), and leaves
        // the destination as it is.
        let target = (node.is_dir()
            && node.meta != Metadata::default()
            && restore_target_is_fresh(&self.dest))
        .then(|| (PathBuf::new(), node.clone()));
        let ls = target.clone().into_iter().map(Ok).chain(repo.ls(&node, &ls_opts)?);

        let dest = LocalDestination::new(&self.dest, true, !node.is_dir())?;

        let restore_infos = repo.prepare_restore(&opts, ls, &dest, dry_run)?;

        let fs = restore_infos.stats.files;
        println!(
            "Files:  {} to restore, {} unchanged, {} verified, {} to modify, {} additional",
            fs.restore, fs.unchanged, fs.verified, fs.modify, fs.additional
        );
        let ds = restore_infos.stats.dirs;
        println!(
            "Dirs:   {} to restore, {} to modify, {} additional",
            ds.restore, ds.modify, ds.additional
        );

        info!(
            "total restore size: {}",
            bytes_size_to_string(restore_infos.restore_size)
        );
        if restore_infos.matched_size > 0 {
            info!(
                "using {} of existing file contents.",
                bytes_size_to_string(restore_infos.matched_size)
            );
        }
        if restore_infos.restore_size == 0 {
            info!("all file contents are fine.");
        }

        if dry_run && config.global.dry_run_warmup {
            repo.warm_up(restore_infos.to_packs().into_iter())?;
        } else if !dry_run && !config.global.dry_run_warmup {
            // save some memory
            let repo = repo.drop_data_from_index();

            let ls = target.into_iter().map(Ok).chain(repo.ls(&node, &ls_opts)?);
            repo.restore(restore_infos, &opts, ls, &dest)?;
            println!("restore done.");
        } else {
            debug!(
                "--dry-run is without warmup, --dry-run --dry-run-warmup also issues the warmup script."
            );
        }

        Ok(())
    }
}

//! `restore` subcommand

use crate::{
    Application, RUSTIC_APP, helpers::bytes_size_to_string, repository::IndexedRepo, status_err,
};

use abscissa_core::{Command, Runnable, Shutdown};
use anyhow::{Result, bail};
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

    // BackOtter: bko-holt's btrfs snapshots, each made exact from the one
    // before it, clone what the live tree already has.
    /// With --exact: files whose contents the snapshot's PATH has, restored at DIR (unchanged since), are cloned from there instead of written
    #[clap(long, value_name = "PATH=DIR", requires = "exact")]
    clone_from: Option<String>,

    // BackOtter: bko-holt's main restore, whose target holds other
    // subvolumes (not fresh) and whose root's metadata the backup holds
    // only once its alias is left out.
    /// The destination gets the subfolder's own metadata, set last, even when it isn't empty
    #[clap(long)]
    target_metadata: bool,

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
        // BackOtter: the directories above a node the globs let through,
        // that they don't let through themselves, are restored too, with
        // their metadata, as in BackOtter's restic.
        ls_opts.with_parents = true;
        // BackOtter: restoring "ID:subfolder" into a destination that the
        // restore creates, or that holds nothing but what a file system
        // makes itself (`lost+found`, UFS's `.snap` and `.sujournal`,
        // nilfs2's empty `.nilfs`; on Haiku, Tracker's empty `trash`),
        // mount points and the directories on the path to them (a new file
        // system's root with the others mounted below it,
        // restore_target_is_fresh), gives the destination the
        // subfolder's own metadata, as everything below it gets its own:
        // the subfolder's node goes first, as the destination itself (an
        // empty path). Without it a restored file system's root keeps
        // mkfs's mode and has no SELinux label. Any other destination stays
        // as it is (owner, mode, times, extended attributes, ACLs, flags),
        // as in stock rustic: /tmp keeps its sticky bit, even empty or
        // holding only empty directories (.X11-unix); a sticky destination,
        // or one others may write in, is never fresh. A whole snapshot has
        // no node of its own (empty metadata), and leaves the destination
        // as it is.
        // BackOtter: under --exact the destination always gets the
        // subfolder's metadata (it is then set only if it differs).
        let target = (node.is_dir()
            && node.meta != Metadata::default()
            && (self.opts.exact || self.target_metadata || restore_target_is_fresh(&self.dest)))
        .then(|| (PathBuf::new(), node.clone()));
        let ls = target.clone().into_iter().map(Ok).chain(repo.ls(&node, &ls_opts)?);

        let mut dest = LocalDestination::new(&self.dest, true, !node.is_dir())?;
        // BackOtter: --clone-from PATH=DIR, the same snapshot's PATH. Nothing
        // in the destination is cloned from when DIR holds it (a snapper
        // snapshot inside the subvolume, seeded from it): it holds the seed,
        // not what the snapshot has there, and a file cloned onto itself was
        // emptied.
        if let Some(spec) = &self.clone_from {
            let Some((from, dir)) = spec.split_once('=') else {
                bail!("--clone-from takes PATH=DIR, not {spec}");
            };
            let from_node = repo.node_from_snapshot_and_path(&snap, from)?;
            let sources = repo.clone_sources(
                &from_node,
                std::path::Path::new(dir),
                Some(std::path::Path::new(&self.dest)),
            )?;
            info!("{} files' contents to clone from {dir}", sources.len());
            dest = dest.with_clone_sources(sources);
        }

        // BackOtter: --dry-run-warmup without --dry-run restores nothing,
        // so it plans as --dry-run does: nothing in the destination is made,
        // removed or changed (an existing directory's XFS options taken off
        // for the restore).
        let plan_only = dry_run || config.global.dry_run_warmup;
        let restore_infos = repo.prepare_restore(&opts, ls, &dest, plan_only)?;
        // BackOtter: what the restore couldn't do and only warned about, which
        // BackOtter's restic counts as errors (exit 1 under `--strict-exit`)
        let mut errors = restore_infos.stats.errors;

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
        if restore_infos.cloned_size > 0 {
            info!(
                "cloning {} of file contents.",
                bytes_size_to_string(restore_infos.cloned_size)
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
            errors += repo.restore(restore_infos, &opts, ls, &dest)?.errors;
            println!("restore done.");
        } else {
            debug!(
                "--dry-run is without warmup, --dry-run --dry-run-warmup also issues the warmup script."
            );
        }

        if config.global.strict_exit && errors > 0 {
            bail!("There were {errors} errors (see the warnings above)");
        }

        Ok(())
    }
}

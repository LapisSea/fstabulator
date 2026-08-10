//! Documented mount options for every supported filesystem type.
//!
//! Each option can be looked up by name, either across all options valid for a
//! filesystem ([`options_for`]) or for a single name ([`lookup`]). Generic
//! fstab options ([`GENERIC_OPTIONS`]) are merged into every filesystem's list,
//! since they are accepted by every filesystem.

use crate::fs_value::FsType;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BoolType {
	YesNo,
	TrueFalse,
	OneZero,
}

impl BoolType {
	pub fn values(self) -> (&'static str, &'static str) {
		match self {
			BoolType::YesNo => ("yes", "no"),
			BoolType::TrueFalse => ("true", "false"),
			BoolType::OneZero => ("1", "0"),
		}
	}

	pub fn parse(self, current: &str) -> Option<bool> {
		match current.to_ascii_lowercase().as_str() {
			"yes" | "true" | "1" => Some(true),
			"no" | "false" | "0" => Some(false),
			_ => None,
		}
	}
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum OptionValue {
	/// Bare flag with no value (defaults, nofail...)
	Toggle,
	/// Generic unrestricted integer
	Integer,
	/// Integer restricted to an inclusive range of `[min, max]`
	IntegerRange(i64, i64),
	/// Integer in octal notation. Used for permissions or masks
	Octal,
	/// Integer size, with a `K/M/G/T/%` marking its size/type
	Size,
	/// Similar to Toggle but has a truthy/falsy value
	Bool(BoolType),
	/// One of a fixed set of string literals
	Enum(&'static [&'static str]),
	/// Arbitrary free-form text (paths, names, labels)
	String,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct FsOption {
	pub name: &'static str,
	pub description: &'static str,
	pub value: OptionValue,
	pub default: Option<&'static str>,
}

macro_rules! opt {
	($name:literal, $value:expr, $description:literal) => {
		FsOption {
			name: $name,
			description: $description,
			value: $value,
			default: None,
		}
	};
	($name:literal, $value:expr, $description:literal, $default:literal) => {
		FsOption {
			name: $name,
			description: $description,
			value: $value,
			default: Some($default),
		}
	};
}

#[rustfmt::skip]
pub const GENERIC_OPTIONS: &[FsOption] = &[
	opt!("async", OptionValue::Toggle, "All I/O to the filesystem is done asynchronously."),
	opt!("atime", OptionValue::Toggle, "Update inode access time on every access."),
	opt!("auto", OptionValue::Toggle, "Can be mounted with the -a (mount all) option."),
	opt!("bind", OptionValue::Toggle, "Remount a subtree elsewhere; mount only that subtree."),
	opt!("comment", OptionValue::String, "Comment field; ignored by the mount command."),
	opt!("context", OptionValue::String, "Set the SELinux context for the whole filesystem."),
	opt!("defaults", OptionValue::Toggle, "Use default options: rw, suid, dev, exec, auto, nouser, async."),
	opt!("defcontext", OptionValue::String, "Set the default SELinux context for unlabeled files."),
	opt!("dev", OptionValue::Toggle, "Interpret character or block special devices on the filesystem."),
	opt!("diratime", OptionValue::Toggle, "Update directory access times on reads."),
	opt!("dirsync", OptionValue::Toggle, "Make all directory updates synchronous."),
	opt!("exec", OptionValue::Toggle, "Permit execution of binaries on the filesystem."),
	opt!("fscontext", OptionValue::String, "Set the SELinux context for the filesystem being mounted."),
	opt!("group", OptionValue::Toggle, "Allow mounting by a user whose group matches the device's group."),
	opt!("iversion", OptionValue::Toggle, "Increment the inode version on every inode modification."),
	opt!("lazytime", OptionValue::Toggle, "Update on-disk timestamps lazily, flushing them later."),
	opt!("loop", OptionValue::Toggle, "Mount the file through a loop device."),
	opt!("loud", OptionValue::Toggle, "Turn off the silent flag (verbose errors)."),
	opt!("mand", OptionValue::Toggle, "Allow mandatory locking on the filesystem."),
	opt!("move", OptionValue::Toggle, "Atomically move a mounted tree to a new location."),
	opt!("noatime", OptionValue::Toggle, "Do not update inode access times on reads."),
	opt!("noauto", OptionValue::Toggle, "Can only be mounted explicitly, never with -a."),
	opt!("nodev", OptionValue::Toggle, "Do not interpret character or block special devices."),
	opt!("nodiratime", OptionValue::Toggle, "Do not update directory access times."),
	opt!("noexec", OptionValue::Toggle, "Do not permit execution of binaries on the filesystem."),
	opt!("nofail", OptionValue::Toggle, "Do not report errors if the device does not exist."),
	opt!("noiversion", OptionValue::Toggle, "Do not increment the inode version counter."),
	opt!("nolazytime", OptionValue::Toggle, "Do not use the lazytime feature; flush timestamps at once."),
	opt!("nomand", OptionValue::Toggle, "Do not allow mandatory locking on the filesystem."),
	opt!("norelatime", OptionValue::Toggle, "Do not use the relatime feature."),
	opt!("nosuid", OptionValue::Toggle, "Do not honor setuid or setgid bits or file capabilities."),
	opt!("nosymfollow", OptionValue::Toggle, "Do not follow symbolic links when resolving paths."),
	opt!("nostrictatime", OptionValue::Toggle, "Use the kernel's default access-time behavior."),
	opt!("nouser", OptionValue::Toggle, "Only root may mount the filesystem."),
	opt!("_netdev", OptionValue::Toggle, "Mount only after the network is available."),
	opt!("owner", OptionValue::Toggle, "Allow mounting by the owner of the device."),
	opt!("private", OptionValue::Toggle, "Make the mount private (no propagation to other mounts)."),
	opt!("rbind", OptionValue::Toggle, "Remount a subtree and all its submounts elsewhere."),
	opt!("relatime", OptionValue::Toggle, "Update access times relative to modify or change time."),
	opt!("remount", OptionValue::Toggle, "Attempt to remount an already-mounted filesystem."),
	opt!("ro", OptionValue::Toggle, "Mount the filesystem read-only."),
	opt!("rootcontext", OptionValue::String, "Set the SELinux context of the filesystem's root inode."),
	opt!("rprivate", OptionValue::Toggle, "Make the mount and its submounts private."),
	opt!("rshared", OptionValue::Toggle, "Make the mount and its submounts shared."),
	opt!("rslave", OptionValue::Toggle, "Make the mount and its submounts slaves of their master."),
	opt!("runbindable", OptionValue::Toggle, "Make the mount and its submounts unbindable."),
	opt!("rw", OptionValue::Toggle, "Mount the filesystem read-write."),
	opt!("silent", OptionValue::Toggle, "Turn on the silent flag (suppress errors)."),
	opt!("slave", OptionValue::Toggle, "Make the mount a slave of its master (receive propagation)."),
	opt!("shared", OptionValue::Toggle, "Make the mount shared with peer mounts."),
	opt!("strictatime", OptionValue::Toggle, "Update access times on every access (full atime)."),
	opt!("suid", OptionValue::Toggle, "Honor setuid or setgid bits and file capabilities."),
	opt!("sync", OptionValue::Toggle, "All I/O to the filesystem is done synchronously."),
	opt!("unbindable", OptionValue::Toggle, "Make the mount unbindable."),
	opt!("user", OptionValue::Toggle, "Allow an ordinary user to mount the filesystem."),
	opt!("users", OptionValue::Toggle, "Allow every user to mount and unmount the filesystem."),
	opt!("X-mount.auto-fstypes", OptionValue::String, "Restrict automatic filesystem detection to the listed types."),
	opt!("X-mount.group", OptionValue::String, "Set the group that may unmount the mount."),
	opt!("X-mount.idmap", OptionValue::String, "Create an idmapped mount using the given ID mapping."),
	opt!("X-mount.mkdir", OptionValue::String, "Create the mountpoint directory if it does not exist.", "0755"),
	opt!("X-mount.mode", OptionValue::String, "Set the mountpoint directory mode after mounting."),
	opt!("X-mount.nocanonicalize", OptionValue::Enum(&["source", "target"]), "Do not canonicalize mount source and target paths."),
	opt!("X-mount.noloop", OptionValue::Toggle, "Do not create a loop device even if the source is a regular file."),
	opt!("X-mount.owner", OptionValue::String, "Set the owner that may unmount the mount."),
	opt!("X-mount.subdir", OptionValue::String, "Mount a subdirectory of the filesystem instead of its root."),
];

#[rustfmt::skip]
pub const EXT2_OPTIONS: &[FsOption] = &[
	opt!("acl", OptionValue::Toggle, "Enables POSIX access control lists."),
	opt!("noacl", OptionValue::Toggle, "Disables POSIX access control lists."),
	opt!("bsddf", OptionValue::Toggle, "statfs reports usable blocks minus filesystem overhead."),
	opt!("minixdf", OptionValue::Toggle, "statfs reports total blocks including overhead."),
	opt!("check", OptionValue::Enum(&["none"]), "Performs mount-time checking; only check=none supported now.", "none"),
	opt!("nocheck", OptionValue::Toggle, "Skips checking at mount time (default)."),
	opt!("debug", OptionValue::Toggle, "Prints debugging info upon each (re)mount."),
	opt!("errors", OptionValue::Enum(&["continue", "remount-ro", "panic"]), "Sets behavior on errors: continue, remount-ro, or panic.", "continue"),
	opt!("grpid", OptionValue::Toggle, "New files inherit the group ID of their parent directory."),
	opt!("bsdgroups", OptionValue::Toggle, "Same as grpid."),
	opt!("nogrpid", OptionValue::Toggle, "New files take the creating process's fsgid (default)."),
	opt!("sysvgroups", OptionValue::Toggle, "Same as nogrpid."),
	opt!("grpquota", OptionValue::Toggle, "Enables group quota support."),
	opt!("usrquota", OptionValue::Toggle, "Enables user quota support."),
	opt!("quota", OptionValue::Toggle, "Enables user quota support (same as usrquota)."),
	opt!("noquota", OptionValue::Toggle, "Disables all quota support."),
	opt!("nouid32", OptionValue::Toggle, "Disables 32-bit UIDs/GIDs for old-kernel compatibility."),
	opt!("oldalloc", OptionValue::Toggle, "Uses the old inode allocator instead of Orlov."),
	opt!("orlov", OptionValue::Toggle, "Uses the Orlov allocator for new inodes (default)."),
	opt!("resgid", OptionValue::Integer, "Group ID allowed to use reserved filesystem blocks.", "0"),
	opt!("resuid", OptionValue::Integer, "User ID allowed to use reserved filesystem blocks.", "0"),
	opt!("sb", OptionValue::Integer, "Mounts using an alternate superblock number (recovery)."),
	opt!("user_xattr", OptionValue::Toggle, "Enables \"user.\" extended attributes."),
	opt!("nouser_xattr", OptionValue::Toggle, "Disables \"user.\" extended attributes."),
];

#[rustfmt::skip]
pub const EXT3_OPTIONS: &[FsOption] = &[
	opt!("barrier", OptionValue::Bool(BoolType::OneZero), "Enables write barriers in the journal code.", "1"),
	opt!("commit", OptionValue::Integer, "Sets journal commit interval in seconds (default 5).", "5"),
	opt!("data", OptionValue::Enum(&["journal", "ordered", "writeback"]), "Sets data journaling mode: journal, ordered, or writeback.", "ordered"),
	opt!("data_err", OptionValue::Enum(&["abort", "ignore"]), "Action on data errors in ordered mode: abort or ignore.", "ignore"),
	opt!("grpjquota", OptionValue::String, "Sets the group quota database file for journaled quotas."),
	opt!("journal_dev", OptionValue::String, "Specifies external journal device by major/minor number."),
	opt!("journal_path", OptionValue::String, "Specifies external journal device by device path."),
	opt!("jqfmt", OptionValue::Enum(&["vfsold", "vfsv0", "vfsv1"]), "Sets quota format: vfsold, vfsv0, or vfsv1."),
	opt!("noload", OptionValue::Toggle, "Does not load the journal on mount."),
	opt!("norecovery", OptionValue::Toggle, "Does not load the journal on mount (same as noload)."),
	opt!("usrjquota", OptionValue::String, "Sets the user quota database file for journaled quotas."),
];

#[rustfmt::skip]
pub const EXT4_OPTIONS: &[FsOption] = &[
	opt!("abort", OptionValue::Toggle, "Simulates an ext4_abort() for debugging purposes."),
	opt!("auto_da_alloc", OptionValue::Toggle, "Forces delayed blocks to disk before rename/truncate commits."),
	opt!("noauto_da_alloc", OptionValue::Toggle, "Disables forced allocation on rename/truncate."),
	opt!("block_validity", OptionValue::Toggle, "Tracks metadata blocks internally (debug; off by default)."),
	opt!("noblock_validity", OptionValue::Toggle, "Disables internal metadata block tracking."),
	opt!("delalloc", OptionValue::Toggle, "Defers block allocation until write-out time."),
	opt!("nodelalloc", OptionValue::Toggle, "Disables delayed allocation."),
	opt!("dioread_lock", OptionValue::Toggle, "Uses inode mutex for DIO reads (default)."),
	opt!("dioread_nolock", OptionValue::Toggle, "Avoids inode mutex for DIO reads for scalability."),
	opt!("discard", OptionValue::Toggle, "Issues discard/TRIM commands when blocks are freed."),
	opt!("nodiscard", OptionValue::Toggle, "Disables issuing discard/TRIM commands."),
	opt!("encoding", OptionValue::String, "Sets the case-insensitive lookup charset for casefolded directories (e.g. utf8-12.1).", "utf8-12.1"),
	opt!("encoding_flags", OptionValue::String, "Sets flags controlling casefold behavior (e.g. strict)."),
	opt!("fast_commit", OptionValue::Toggle, "Enables fast commits, a lightweight journaling mode for lower commit latency."),
	opt!("init_itable", OptionValue::Integer, "Tunes the lazy inode-table initialization speed.", "10"),
	opt!("noinit_itable", OptionValue::Toggle, "Does not initialize inode tables in the background."),
	opt!("inode_readahead_blks", OptionValue::Integer, "Max inode table blocks pre-read during mount.", "32"),
	opt!("i_version", OptionValue::Toggle, "Enables 64-bit inode version support."),
	opt!("journal_async_commit", OptionValue::Toggle, "Writes commit block without waiting for descriptors."),
	opt!("journal_checksum", OptionValue::Toggle, "Enables checksumming of journal transactions."),
	opt!("nojournal_checksum", OptionValue::Toggle, "Disables journal transaction checksumming."),
	opt!("journal_ioprio", OptionValue::IntegerRange(0, 7), "Sets I/O priority for journal commits (0-7).", "3"),
	opt!("max_batch_time", OptionValue::Integer, "Max microseconds to wait to batch sync-write operations.", "15000"),
	opt!("max_dir_size_kb", OptionValue::Integer, "Limits maximum directory size in kilobytes.", "0"),
	opt!("min_batch_time", OptionValue::Integer, "Minimum commit time for batch optimization (us).", "0"),
	opt!("nobarrier", OptionValue::Toggle, "Disables write barriers."),
	opt!("nombcache", OptionValue::Toggle, "Disables mbcache for extended attribute deduplication."),
	opt!("prjquota", OptionValue::Toggle, "Enables project quota support."),
	opt!("stripe", OptionValue::Integer, "Sets stripe width (blocks) for mballoc allocation alignment."),
];

#[rustfmt::skip]
pub const XFS_OPTIONS: &[FsOption] = &[
	opt!("allocsize", OptionValue::Size, "Sets end-of-file preallocation size for buffered I/O."),
	opt!("attr2", OptionValue::Toggle, "Enables opportunistic inline xattr format; deprecated since 5.10."),
	opt!("noattr2", OptionValue::Toggle, "Disables attr2 format; deprecated, rejected on CRC filesystems."),
	opt!("dax", OptionValue::Enum(&["inode", "never", "always"]), "Sets direct-access mode: never, always, or inode.", "inode"),
	opt!("discard", OptionValue::Toggle, "Issues commands to reclaim freed blocks (SSD-friendly)."),
	opt!("nodiscard", OptionValue::Toggle, "Disables issuing discard commands."),
	opt!("filestreams", OptionValue::Toggle, "Applies filestreams allocation mode to whole filesystem."),
	opt!("gqnoenforce", OptionValue::Toggle, "Enables group quota accounting without enforcement."),
	opt!("gquota", OptionValue::Toggle, "Enables group quota accounting and enforcement."),
	opt!("grpquota", OptionValue::Toggle, "Same as gquota."),
	opt!("grpid", OptionValue::Toggle, "New files inherit the parent directory's group ID."),
	opt!("bsdgroups", OptionValue::Toggle, "Same as grpid."),
	opt!("nogrpid", OptionValue::Toggle, "New files take the process's fsgid (default)."),
	opt!("sysvgroups", OptionValue::Toggle, "Same as nogrpid."),
	opt!("ikeep", OptionValue::Toggle, "Keeps empty inode clusters on disk; deprecated since 5.10."),
	opt!("noikeep", OptionValue::Toggle, "Recycles empty inode clusters; deprecated since 5.10."),
	opt!("inode32", OptionValue::Toggle, "Restricts inode numbers to 32 bits."),
	opt!("inode64", OptionValue::Toggle, "Allows 64-bit inode numbers (default)."),
	opt!("largeio", OptionValue::Toggle, "Reports swidth/allocsize as st_blksize."),
	opt!("nolargeio", OptionValue::Toggle, "Reports minimal st_blksize (default)."),
	opt!("logbufs", OptionValue::IntegerRange(2, 8), "Sets number of in-memory log buffers (2-8).", "8"),
	opt!("logbsize", OptionValue::Size, "Sets size of each in-memory log buffer.", "32k"),
	opt!("logdev", OptionValue::String, "Uses an external log (metadata journal) device."),
	opt!("noalign", OptionValue::Toggle, "Skips stripe-unit alignment for data allocations."),
	opt!("norecovery", OptionValue::Toggle, "Mounts without running log recovery (read-only)."),
	opt!("nouuid", OptionValue::Toggle, "Skips duplicate-UUID mount detection."),
	opt!("noquota", OptionValue::Toggle, "Forcibly turns off all quota accounting and enforcement."),
	opt!("pquota", OptionValue::Toggle, "Enables project quota accounting and enforcement."),
	opt!("pqnoenforce", OptionValue::Toggle, "Enables project quota accounting without enforcement."),
	opt!("prjquota", OptionValue::Toggle, "Same as pquota."),
	opt!("qnoenforce", OptionValue::Toggle, "Enables user quota accounting without enforcement."),
	opt!("quota", OptionValue::Toggle, "Enables user quota accounting and enforcement."),
	opt!("rtdev", OptionValue::String, "Uses an external realtime-section device."),
	opt!("sunit", OptionValue::Integer, "Sets stripe unit in 512-byte blocks."),
	opt!("swalloc", OptionValue::Toggle, "Rounds data allocations up to stripe-width boundaries."),
	opt!("swidth", OptionValue::Integer, "Sets stripe width in 512-byte blocks."),
	opt!("uquota", OptionValue::Toggle, "Enables user quota accounting and enforcement."),
	opt!("uqnoenforce", OptionValue::Toggle, "Enables user quota accounting without enforcement."),
	opt!("usrquota", OptionValue::Toggle, "Same as uquota."),
	opt!("wsync", OptionValue::Toggle, "Runs filesystem namespace operations synchronously."),
];

#[rustfmt::skip]
pub const BTRFS_OPTIONS: &[FsOption] = &[
	opt!("acl", OptionValue::Toggle, "Enables POSIX access control lists (default on)."),
	opt!("noacl", OptionValue::Toggle, "Disables POSIX access control lists."),
	opt!("autodefrag", OptionValue::Toggle, "Auto-defragments detected small random writes."),
	opt!("noautodefrag", OptionValue::Toggle, "Disables automatic file defragmentation."),
	opt!("barrier", OptionValue::Toggle, "Ensures writes flush to permanent storage at checkpoints."),
	opt!("nobarrier", OptionValue::Toggle, "Disables write barriers; risks corruption on crash."),
	opt!("clear_cache", OptionValue::Toggle, "Clears and rebuilds the free space cache."),
	opt!("commit", OptionValue::Integer, "Sets transaction commit interval in seconds (default 30).", "30"),
	opt!("compress", OptionValue::Enum(&["zlib", "lzo", "zstd", "no"]), "Enables data compression (zlib, lzo, or zstd).", "zstd"),
	opt!("compress-force", OptionValue::Enum(&["zlib", "lzo", "zstd", "no"]), "Always attempts compression, even if ineffective.", "zstd"),
	opt!("datacow", OptionValue::Toggle, "Enables data copy-on-write for new files."),
	opt!("nodatacow", OptionValue::Toggle, "Disables data COW (implies nodatasum, no compression)."),
	opt!("datasum", OptionValue::Toggle, "Enables data checksumming."),
	opt!("nodatasum", OptionValue::Toggle, "Disables data checksumming."),
	opt!("degraded", OptionValue::Toggle, "Allows mount with fewer devices than RAID requires."),
	opt!("device", OptionValue::String, "Scans the given device for btrfs during mount."),
	opt!("discard", OptionValue::Enum(&["sync", "async"]), "Discards freed file blocks (TRIM).", "async"),
	opt!("nodiscard", OptionValue::Toggle, "Disables discarding of freed blocks."),
	opt!("enospc_debug", OptionValue::Toggle, "Enables verbose output for ENOSPC conditions."),
	opt!("noenospc_debug", OptionValue::Toggle, "Disables verbose ENOSPC output."),
	opt!("fatal_errors", OptionValue::Enum(&["bug", "panic"]), "Sets action on fatal errors: bug or panic.", "bug"),
	opt!("flushoncommit", OptionValue::Toggle, "Flushes all prior-transaction data at each commit."),
	opt!("noflushoncommit", OptionValue::Toggle, "Disables forced full flush on commit."),
	opt!("fragment", OptionValue::Enum(&["data", "metadata", "all"]), "Intentionally fragments block groups (debug builds only)."),
	opt!("max_inline", OptionValue::Size, "Max bytes that can be inlined in a metadata leaf.", "2048"),
	opt!("metadata_ratio", OptionValue::Integer, "Allocates one metadata chunk per N data chunks.", "0"),
	opt!("nologreplay", OptionValue::Toggle, "Does not replay the tree log (same as norecovery)."),
	opt!("norecovery", OptionValue::Toggle, "Does not attempt recovery at mount time."),
	opt!("rescan_uuid_tree", OptionValue::Toggle, "Forces rebuild of the UUID tree."),
	opt!(
		"rescue",
		OptionValue::Enum(&["usebackuproot","nologreplay","ignorebadroots","ibadroots","ignoredatacsums","idatacsums","ignoremetacsums","imetacsums","all",]),
		"Applies recovery workarounds: usebackuproot, nologreplay, ignorebadroots, ignore*sums, or all (read-only)."
	),
	opt!("skip_balance", OptionValue::Toggle, "Skips automatic resume of an interrupted balance."),
	opt!("space_cache", OptionValue::Enum(&["v1", "v2"]), "Enables free space caching (v1 or v2).", "v2"),
	opt!("nospace_cache", OptionValue::Toggle, "Disables the free space cache."),
	opt!("ssd", OptionValue::Toggle, "Enables SSD allocation optimizations."),
	opt!("nossd", OptionValue::Toggle, "Disables SSD allocation optimizations."),
	opt!("ssd_spread", OptionValue::Toggle, "Allocates into larger aligned chunks for low-end SSDs."),
	opt!("nossd_spread", OptionValue::Toggle, "Disables the ssd_spread allocation scheme."),
	opt!("subvol", OptionValue::String, "Mounts a subvolume at the given path, not the toplevel.", "@"),
	opt!("subvolid", OptionValue::Integer, "Mounts the subvolume with the given numeric ID."),
	opt!("thread_pool", OptionValue::Integer, "Sets number of worker threads."),
	opt!("treelog", OptionValue::Toggle, "Enables tree logging for fsync/O_SYNC."),
	opt!("notreelog", OptionValue::Toggle, "Disables tree logging."),
	opt!("usebackuproot", OptionValue::Toggle, "Tries backup roots if the tree root is bad; removed since 7.3."),
	opt!("user_subvol_rm_allowed", OptionValue::Toggle, "Lets subvolume owners delete their own subvolumes."),
];

#[rustfmt::skip]
pub const F2FS_OPTIONS: &[FsOption] = &[
	opt!("active_logs", OptionValue::Enum(&["2", "4", "6"]), "Sets number of active log segments (2, 4, or 6).", "6"),
	opt!("age_extent_cache", OptionValue::Toggle, "Enables age extent cache for allocation hints."),
	opt!("alloc_mode", OptionValue::Enum(&["reuse", "default"]), "Sets block allocation policy: reuse or default.", "default"),
	opt!("atgc", OptionValue::Toggle, "Enables age-threshold background garbage collection."),
	opt!("background_gc", OptionValue::Enum(&["on", "off", "sync"]), "Turns background garbage collection on/off/sync.", "on"),
	opt!("barrier", OptionValue::Toggle, "Allows cache_flush commands to be issued."),
	opt!("nobarrier", OptionValue::Toggle, "Skips cache_flush commands if storage guarantees order."),
	opt!("checkpoint", OptionValue::Enum(&["enable", "disable"]), "Enables or disables checkpointing (disable/enable).", "enable"),
	opt!("checkpoint_merge", OptionValue::Toggle, "Merges concurrent checkpoint requests via a daemon."),
	opt!("nocheckpoint_merge", OptionValue::Toggle, "Disables checkpoint merge feature."),
	opt!("compress_algorithm", OptionValue::Enum(&["lzo", "lz4", "zstd", "lzo-rle"]), "Selects compression algorithm (lzo, lz4, zstd).", "lz4"),
	opt!("compress_cache", OptionValue::Toggle, "Caches compressed blocks to improve read hit ratio."),
	opt!("compress_chksum", OptionValue::Toggle, "Verifies checksums of compressed clusters."),
	opt!("compress_extension", OptionValue::String, "Adds extensions whose files are auto-compressed."),
	opt!("compress_log_size", OptionValue::Integer, "Sets compress cluster size in 4KB units.", "2"),
	opt!("compress_mode", OptionValue::Enum(&["fs", "user"]), "Sets compression mode: fs (auto) or user (manual).", "fs"),
	opt!("data_flush", OptionValue::Toggle, "Flushes data before checkpoint to persist regular files."),
	opt!("disable_ext_identify", OptionValue::Toggle, "Ignores mkfs extension list for cold-file detection."),
	opt!("disable_roll_forward", OptionValue::Toggle, "Disables the roll-forward recovery routine."),
	opt!("discard", OptionValue::Toggle, "Enables real-time discard/TRIM commands."),
	opt!("nodiscard", OptionValue::Toggle, "Disables real-time discard."),
	opt!("discard_unit", OptionValue::Enum(&["block", "segment", "section"]), "Sets discard alignment unit: block, segment, or section.", "block"),
	opt!("errors", OptionValue::Enum(&["panic", "continue", "remount-ro"]), "Sets behavior on critical errors: panic, continue, remount-ro.", "continue"),
	opt!("extent_cache", OptionValue::Toggle, "Enables the rb-tree extent cache (default)."),
	opt!("noextent_cache", OptionValue::Toggle, "Disables the extent cache."),
	opt!("fastboot", OptionValue::Toggle, "Reduces mount time at the cost of normal performance."),
	opt!("fault_injection", OptionValue::Integer, "Enables fault injection at a given rate (testing).", "0"),
	opt!("fault_type", OptionValue::Integer, "Configures which fault types to inject (testing).", "0"),
	opt!("flush_merge", OptionValue::Toggle, "Merges concurrent cache_flush commands."),
	opt!("fsync_mode", OptionValue::Enum(&["posix", "strict", "nobarrier"]), "Sets fsync policy: posix, strict, or nobarrier.", "posix"),
	opt!("gc_merge", OptionValue::Toggle, "Handles foreground GC in the background GC thread."),
	opt!("nogc_merge", OptionValue::Toggle, "Disables GC merge feature."),
	opt!("grpjquota", OptionValue::String, "Points to group journaled quota file."),
	opt!("grpquota", OptionValue::Toggle, "Enables group disk quota accounting."),
	opt!("heap", OptionValue::Toggle, "Allocates segments heap-based from disk end; deprecated."),
	opt!("no_heap", OptionValue::Toggle, "Disables heap-based allocation; deprecated."),
	opt!("inline_data", OptionValue::Toggle, "Stores small file data inside the inode block."),
	opt!("inline_dentry", OptionValue::Toggle, "Stores directory entries inside the inode block."),
	opt!("noinline_dentry", OptionValue::Toggle, "Disables the inline dentry feature."),
	opt!("inline_xattr", OptionValue::Toggle, "Enables inline extended attributes."),
	opt!("noinline_xattr", OptionValue::Toggle, "Disables inline extended attributes."),
	opt!("inline_xattr_size", OptionValue::Integer, "Sets the inline xattr size.", "50"),
	opt!("inlinecrypt", OptionValue::Toggle, "Uses blk-crypto for inline encryption hardware."),
	opt!("jqfmt", OptionValue::Enum(&["vfsold", "vfsv0", "vfsv1"]), "Sets quota format: vfsold, vfsv0, or vfsv1."),
	opt!("lookup_mode", OptionValue::Enum(&["perf", "compat", "auto"]), "Sets casefold directory lookup: perf, compat, or auto.", "perf"),
	opt!("memory", OptionValue::Enum(&["normal", "low"]), "Sets memory mode: normal or low.", "normal"),
	opt!("mode", OptionValue::Enum(&["adaptive", "lfs", "fragment:segment", "fragment:block"]), "Sets block allocation mode: adaptive or lfs.", "adaptive"),
	opt!("nat_bits", OptionValue::Toggle, "Enables faster full/empty NAT block access."),
	opt!("noacl", OptionValue::Toggle, "Disables POSIX access control lists."),
	opt!("noinline_data", OptionValue::Toggle, "Disables the inline data feature."),
	opt!("norecovery", OptionValue::Toggle, "Disables roll-forward recovery and mounts read-only."),
	opt!("nouser_xattr", OptionValue::Toggle, "Disables user extended attributes."),
	opt!("noquota", OptionValue::Toggle, "Disables all plain disk quota options."),
	opt!("prjjquota", OptionValue::String, "Points to project journaled quota file."),
	opt!("prjquota", OptionValue::Toggle, "Enables project quota accounting."),
	opt!("quota", OptionValue::Toggle, "Enables plain user disk quota accounting."),
	opt!("reserve_node", OptionValue::Integer, "Sets reserved nodes for privileged-user allocation.", "0"),
	opt!("reserve_root", OptionValue::Integer, "Sets reserved space (%) for privileged-user allocation.", "0"),
	opt!("resgid", OptionValue::Integer, "Group ID with access to reserved blocks and nodes."),
	opt!("resuid", OptionValue::Integer, "User ID with access to reserved blocks and nodes."),
	opt!("test_dummy_encryption", OptionValue::Enum(&["v1", "v2"]), "Enables dummy fscrypt context for testing.", "v2"),
	opt!("usrjquota", OptionValue::String, "Points to user journaled quota file."),
	opt!("usrquota", OptionValue::Toggle, "Enables user disk quota accounting."),
];

#[rustfmt::skip]
pub const NTFS3_OPTIONS: &[FsOption] = &[
	opt!("acl", OptionValue::Toggle, "Enables POSIX Access Control List support."),
	opt!("discard", OptionValue::Toggle, "Enables TRIM support for SSD delete performance."),
	opt!("dmask", OptionValue::Octal, "Permission mask for directories."),
	opt!("fmask", OptionValue::Octal, "Permission mask for files."),
	opt!("force", OptionValue::Toggle, "Forces mounting even if the volume is dirty (not recommended)."),
	opt!("gid", OptionValue::Integer, "Default group ID for created files and directories."),
	opt!("hide_dot_files", OptionValue::Toggle, "Sets the HIDDEN attribute for names starting with a dot."),
	opt!("iocharset", OptionValue::String, "Charset used to translate path strings to Unicode.", "utf8"),
	opt!("nohidden", OptionValue::Toggle, "Hides files with the Windows HIDDEN attribute."),
	opt!("prealloc", OptionValue::Toggle, "Preallocates space to reduce write fragmentation."),
	opt!("showmeta", OptionValue::Toggle, "Shows all NTFS meta-files (system files)."),
	opt!("sparse", OptionValue::Toggle, "Creates new files as sparse."),
	opt!("sys_immutable", OptionValue::Toggle, "Marks files with the SYSTEM attribute as immutable."),
	opt!("uid", OptionValue::Integer, "Default user ID for created files and directories."),
	opt!("umask", OptionValue::Octal, "Default permission mask for files and directories."),
	opt!("windows_names", OptionValue::Toggle, "Rejects filenames not allowed by Windows."),
];

#[rustfmt::skip]
pub const VFAT_OPTIONS: &[FsOption] = &[
	opt!("allow_utime", OptionValue::Integer, "Relaxes utime() permission checks for changing timestamps."),
	opt!("blocksize", OptionValue::Enum(&["512", "1024", "2048"]), "Sets the block size; obsolete."),
	opt!("check", OptionValue::Enum(&["r", "s", "n", "relaxed", "strict", "normal"]), "Sets case-sensitivity checking (strict, relaxed, or normal).", "normal"),
	opt!("codepage", OptionValue::Integer, "Sets codepage for converting to shortname characters.", "437"),
	opt!("conv", OptionValue::String, "Obsolete; may fail or be ignored."),
	opt!("cvf_format", OptionValue::String, "Obsolete; forces use of a CVF compressed-volume module."),
	opt!("cvf_option", OptionValue::String, "Obsolete; passes an option to the CVF module."),
	opt!("debug", OptionValue::Toggle, "Enables debug output; unused by current implementation."),
	opt!("discard", OptionValue::Toggle, "Issues discard/TRIM commands when blocks are freed."),
	opt!("dmask", OptionValue::Octal, "Permission mask for directories."),
	opt!("dos1xfloppy", OptionValue::Toggle, "Uses DOS 1.x BIOS Parameter Block defaults for small floppies."),
	opt!("dots", OptionValue::Toggle, "Forces DOS dot conventions on filenames (obsolete)."),
	opt!("nodots", OptionValue::Toggle, "Forces Unix-style names without DOS dots (obsolete)."),
	opt!("dotsOK", OptionValue::Bool(BoolType::OneZero), "Allows dots in 8.3 short names (obsolete).", "0"),
	opt!("errors", OptionValue::Enum(&["panic", "continue", "remount-ro"]), "Sets FAT behavior on errors: panic, continue, or remount-ro.", "remount-ro"),
	opt!("fat", OptionValue::Enum(&["12", "16", "32"]), "Forces FAT type 12, 16, or 32, overriding auto-detection."),
	opt!("flush", OptionValue::Toggle, "Flushes data to disk earlier than normal."),
	opt!("fmask", OptionValue::Octal, "Permission mask for regular files."),
	opt!("gid", OptionValue::Integer, "Group ID applied to all files."),
	opt!("iocharset", OptionValue::String, "Charset converting filenames between disk and Unicode.", "iso8859-1"),
	opt!("nfs", OptionValue::Enum(&["stale_rw", "nostale_ro"]), "Enables NFS export support (stale_rw or nostale_ro)."),
	opt!("nocase", OptionValue::Toggle, "Deprecated; use shortname=win95 instead."),
	opt!("nonumtail", OptionValue::Bool(BoolType::OneZero), "Skips the ~number tail in 8.3 aliases when possible.", "0"),
	opt!("posix", OptionValue::Toggle, "Obsolete; allowed files differing only in case."),
	opt!("quiet", OptionValue::Toggle, "Suppresses warning messages and chmod/chown errors."),
	opt!("rodir", OptionValue::Toggle, "Treats the FAT read-only attribute as read-only for directories."),
	opt!("shortname", OptionValue::Enum(&["lower", "win95", "winnt", "mixed"]), "Sets 8.3 short-name display/create mode (lower, win95, winnt, mixed).", "mixed"),
	opt!("showexec", OptionValue::Toggle, "Allows execute permission only for .EXE, .COM, or .BAT files."),
	opt!("sys_immutable", OptionValue::Toggle, "Treats the FAT system attribute as the immutable flag."),
	opt!("time_offset", OptionValue::Integer, "Sets minute offset converting FAT local time to UTC.", "0"),
	opt!("tz", OptionValue::Enum(&["UTC"]), "Interprets timestamps as UTC instead of local time."),
	opt!("uid", OptionValue::Integer, "Owner user ID applied to all files."),
	opt!("umask", OptionValue::Octal, "Permission mask for files and directories."),
	opt!("uni_xlate", OptionValue::Bool(BoolType::OneZero), "Escapes unhandled Unicode characters as :XXXX sequences.", "0"),
	opt!("usefree", OptionValue::Toggle, "Uses the free-cluster count stored in FSINFO."),
	opt!("utf8", OptionValue::Bool(BoolType::OneZero), "Use UTF-8 to encode long file names.", "0"),
];

#[rustfmt::skip]
pub const EXFAT_OPTIONS: &[FsOption] = &[
	opt!("allow_utime", OptionValue::Integer, "Relaxes utime() permission checks for changing timestamps."),
	opt!("codepage", OptionValue::Integer, "Deprecated; accepted but has no effect."),
	opt!("debug", OptionValue::Toggle, "Deprecated; accepted but has no effect."),
	opt!("discard", OptionValue::Toggle, "Issues discard/TRIM commands when blocks are freed."),
	opt!("dmask", OptionValue::Octal, "Permission mask for directories."),
	opt!("errors", OptionValue::Enum(&["continue", "panic", "remount-ro"]), "Sets behavior on errors: panic, continue, or remount-ro.", "remount-ro"),
	opt!("fmask", OptionValue::Octal, "Permission mask for regular files."),
	opt!("gid", OptionValue::Integer, "Default group ID applied to all files."),
	opt!("iocharset", OptionValue::String, "Charset converting filenames between disk and Unicode.", "utf8"),
	opt!("keep_last_dots", OptionValue::Toggle, "Keeps trailing periods in path components and filenames."),
	opt!("namecase", OptionValue::Integer, "Deprecated; accepted but has no effect."),
	opt!("sys_tz", OptionValue::Toggle, "Uses the system timezone for timestamps lacking a valid offset."),
	opt!("time_offset", OptionValue::IntegerRange(-1440, 1440), "Sets minute offset converting timestamps to UTC.", "0"),
	opt!("uid", OptionValue::Integer, "Default user ID applied to all files."),
	opt!("umask", OptionValue::Octal, "Default permission mask for files and directories."),
	opt!("utf8", OptionValue::Toggle, "Deprecated; accepted but has no effect."),
	opt!("zero_size_dir", OptionValue::Toggle, "Creates directories with zero size, allocating no cluster."),
];

#[rustfmt::skip]
pub const SWAP_OPTIONS: &[FsOption] = &[
	opt!("discard", OptionValue::Toggle, "Enables swap discard; supports discard=once or discard=pages."),
	opt!("pri", OptionValue::IntegerRange(0, 32767), "Sets the swap device priority (0-32767).", "-1"),
];

#[rustfmt::skip]
pub const CIFS_OPTIONS: &[FsOption] = &[
	opt!("acdirmax", OptionValue::Integer, "Maximum seconds to cache directory attributes.", "1"),
	opt!("acregmax", OptionValue::Integer, "Maximum seconds to cache regular file attributes.", "1"),
	opt!("actimeo", OptionValue::Integer, "Seconds to cache file/directory attributes before re-checking.", "1"),
	opt!("addr", OptionValue::String, "Sets the destination IP address of the server."),
	opt!("backupgid", OptionValue::String, "Members of this group open files with backup intent."),
	opt!("backupuid", OptionValue::String, "This user opens files with backup intent."),
	opt!("bsize", OptionValue::Size, "Overrides the default block size reported on SMB3 files.", "1048576"),
	opt!("cache", OptionValue::Enum(&["none", "strict", "loose"]), "Cache mode for file data: none, strict or loose.", "strict"),
	opt!("cifsacl", OptionValue::Toggle, "Maps CIFS/NTFS ACLs and SIDs to/from Linux permissions."),
	opt!("closetimeo", OptionValue::Integer, "Max seconds to defer the final SMB3 file close.", "1"),
	opt!("compress", OptionValue::Toggle, "Enables experimental SMB 3.1.1 message compression."),
	opt!("cred", OptionValue::String, "Reads username/password from a credentials file."),
	opt!("credentials", OptionValue::String, "Reads username/password from a credentials file."),
	opt!("cruid", OptionValue::Integer, "Uid owning the credentials cache (useful with krb5)."),
	opt!("dir_mode", OptionValue::String, "Overrides the default mode for directories.", "0755"),
	opt!("dom", OptionValue::String, "Sets the user's domain (workgroup)."),
	opt!("domain", OptionValue::String, "Sets the user's domain (workgroup)."),
	opt!("domainauto", OptionValue::Toggle, "Auto-guesses the server domain from the NTLM challenge."),
	opt!("dynperm", OptionValue::Toggle, "Keeps unsupported permissions in memory only; unreliable."),
	opt!("echo_interval", OptionValue::Integer, "Seconds between keepalive echo requests.", "60"),
	opt!("esize", OptionValue::Integer, "Min encrypted-read size before offloading decryption.", "0"),
	opt!("file_mode", OptionValue::String, "Overrides the default mode for regular files.", "0755"),
	opt!("forcegid", OptionValue::Toggle, "Ignores server gid and always uses the gid value."),
	opt!("forceuid", OptionValue::Toggle, "Ignores server uid and always uses the uid value."),
	opt!("forcemandatorylock", OptionValue::Toggle, "Always uses CIFS-style mandatory locks."),
	opt!("fsc", OptionValue::Toggle, "Enables local disk caching via FS-Cache."),
	opt!("gid", OptionValue::String, "Default gid owning files when the server gives none.", "0"),
	opt!("guest", OptionValue::Toggle, "Don't prompt for a password; mount as guest."),
	opt!("handlecache", OptionValue::Toggle, "Keeps the share root directory handle cached."),
	opt!("handletimeout", OptionValue::Integer, "Ms the server reserves handles after failover.", "0"),
	opt!("hard", OptionValue::Toggle, "Hangs when the server crashes instead of erroring."),
	opt!("idsfromsid", OptionValue::Toggle, "Extracts uid/gid from a special SID instead of mapping."),
	opt!("ignorecase", OptionValue::Toggle, "Synonym for nocase; case-insensitive matching."),
	opt!("intr", OptionValue::Toggle, "Allow interrupts while hung; currently unimplemented."),
	opt!("iocharset", OptionValue::String, "Charset for converting path names to/from Unicode."),
	opt!("ip", OptionValue::String, "Sets the destination IP address of the server."),
	opt!("linux", OptionValue::Toggle, "Enables Unix Extensions (synonym for posix/unix)."),
	opt!("locallease", OptionValue::Toggle, "Checks cached leases locally instead of querying server."),
	opt!("mapchars", OptionValue::Toggle, "Translates reserved Windows filename characters."),
	opt!("mapposix", OptionValue::Toggle, "Maps reserved characters per Microsoft Services For Mac."),
	opt!("max_cached_dirs", OptionValue::Integer, "Maximum number of cached directories per share.", "16"),
	opt!("max_channels", OptionValue::Integer, "Number of multichannel transport connections (max 16).", "1"),
	opt!("max_credits", OptionValue::Integer, "Maximum SMB2 credits the client can hold.", "32000"),
	opt!("mfsymlinks", OptionValue::Toggle, "Enables Minshall+French symlink support."),
	opt!("multichannel", OptionValue::Toggle, "Uses multiple transport connections for one session."),
	opt!("multiuser", OptionValue::Toggle, "Maps each user's access to individual credentials."),
	opt!("netbiosname", OptionValue::String, "Client netbios name used for port 139 connections."),
	opt!("noacl", OptionValue::Toggle, "Disables POSIX ACL operations."),
	opt!("noautotune", OptionValue::Toggle, "Uses fixed kernel socket buffer sizes."),
	opt!("nobrl", OptionValue::Toggle, "Don't send byte-range lock requests to the server."),
	opt!("nocase", OptionValue::Toggle, "Requests case-insensitive path name matching."),
	opt!("nodfs", OptionValue::Toggle, "Don't follow Distributed FileSystem referrals."),
	opt!("nohandlecache", OptionValue::Toggle, "Disables caching of the share root handle."),
	opt!("nointr", OptionValue::Toggle, "Don't allow interrupts; currently unimplemented."),
	opt!("nolease", OptionValue::Toggle, "Don't request leases/oplocks when opening files."),
	opt!("nolinux", OptionValue::Toggle, "Disables Unix Extensions (synonym for noposix)."),
	opt!("nomapchars", OptionValue::Toggle, "Don't translate reserved filename characters."),
	opt!("noperm", OptionValue::Toggle, "Disables client-side permission checking."),
	opt!("nopersistenthandles", OptionValue::Toggle, "Disables persistent handles."),
	opt!("noposix", OptionValue::Toggle, "Disables Unix Extensions for this mount."),
	opt!("noposixpaths", OptionValue::Toggle, "Don't negotiate POSIX-style pathnames."),
	opt!("noresilienthandles", OptionValue::Toggle, "Disables resilient handles."),
	opt!("noserverino", OptionValue::Toggle, "Client generates inode numbers locally."),
	opt!("nosetuids", OptionValue::Toggle, "Lets the server set uid/gid on new files."),
	opt!("nosharesock", OptionValue::Toggle, "Never reuses existing sockets for new mounts."),
	opt!("nostrictsync", OptionValue::Toggle, "Don't flush to the server on fsync()."),
	opt!("nounix", OptionValue::Toggle, "Disables Unix Extensions (synonym for noposix)."),
	opt!("nouser_xattr", OptionValue::Toggle, "Disables getfattr/setfattr xattr operations."),
	opt!("pass", OptionValue::String, "Specifies the CIFS password (alias of password)."),
	opt!("pass2", OptionValue::String, "Specifies an alternate password for password rotation."),
	opt!("password", OptionValue::String, "Specifies the CIFS password."),
	opt!("password2", OptionValue::String, "Specifies an alternate password for password rotation."),
	opt!("perm", OptionValue::Toggle, "Enables client-side permission checking (default)."),
	opt!("persistenthandles", OptionValue::Toggle, "Keeps opened files across reconnections."),
	opt!("port", OptionValue::Integer, "Port to connect to the CIFS server.", "445"),
	opt!("posix", OptionValue::Toggle, "Enables Unix Extensions for this mount."),
	opt!("posixpaths", OptionValue::Toggle, "Allows POSIX-style pathnames (inverse of noposixpaths)."),
	opt!("rdma", OptionValue::Toggle, "Uses SMB Direct over an RDMA adapter."),
	opt!("resilienthandles", OptionValue::Toggle, "Keeps opened files across reconnections."),
	opt!("rsize", OptionValue::Size, "Maximum bytes requested in each read request.", "4194304"),
	opt!("rwpidforward", OptionValue::Toggle, "Forwards the opening pid on read/write operations."),
	opt!("seal", OptionValue::Toggle, "Encrypts traffic at the SMB layer (SMB3+)."),
	opt!(
		"sec",
		OptionValue::Enum(&["none", "krb5", "krb5i", "ntlm", "ntlmi", "ntlmv2", "ntlmv2i", "ntlmssp", "ntlmsspi"]),
		"Sets the session security mode (none, krb5, krb5i, ntlm, ntlmi, ntlmv2, ntlmv2i, ntlmssp, ntlmsspi).",
		"ntlmssp"),
	opt!("servern", OptionValue::String, "Server netbios name for old port 139 servers."),
	opt!("serverino", OptionValue::Toggle, "Uses inode numbers returned by the server."),
	opt!("setuids", OptionValue::Toggle, "Sets uid/gid of processes on newly created files."),
	opt!("sfu", OptionValue::Toggle, "Creates device/fifo files in Services for Unix format."),
	opt!("sloppy", OptionValue::Toggle, "Ignores unrecognized mount options following it."),
	opt!("snapshot", OptionValue::String, "Mounts a specific snapshot of the remote share."),
	opt!("soft", OptionValue::Toggle, "Returns errors instead of hanging when server crashes."),
	opt!("uid", OptionValue::String, "Default uid owning files when the server gives none.", "0"),
	opt!("unix", OptionValue::Toggle, "Enables Unix Extensions (synonym for posix)."),
	opt!("upcall_target", OptionValue::Enum(&["mount", "app"]), "Namespace in which kernel upcalls are handled.", "app"),
	opt!("user", OptionValue::String, "Specifies the SMB username (alias of username)."),
	opt!("username", OptionValue::String, "Specifies the SMB username to connect as."),
	opt!("vers", OptionValue::Enum(&["1.0", "2.0", "2.1", "3.0", "3.02", "3.0.2", "3.1.1", "3.11", "3", "default"]), "SMB protocol version (e.g. 1.0, 2.1, 3.1.1).", "default"),
	opt!("workgroup", OptionValue::String, "Sets the user's domain (workgroup)."),
	opt!("wsize", OptionValue::Size, "Maximum bytes sent in each write request.", "4194304"),
];

#[rustfmt::skip]
pub const NFS_OPTIONS: &[FsOption] = &[
	opt!("ac", OptionValue::Toggle, "Enables attribute caching (default)."),
	opt!("noac", OptionValue::Toggle, "Disables attribute caching; makes writes synchronous."),
	opt!("acdirmax", OptionValue::Integer, "Max seconds directory attributes are cached.", "60"),
	opt!("acdirmin", OptionValue::Integer, "Min seconds directory attributes are cached.", "30"),
	opt!("acregmax", OptionValue::Integer, "Max seconds regular file attributes are cached.", "60"),
	opt!("acregmin", OptionValue::Integer, "Min seconds regular file attributes are cached.", "3"),
	opt!("actimeo", OptionValue::Integer, "Sets all attribute-cache timeouts to this value."),
	opt!("acl", OptionValue::Toggle, "Enables the NFSACL sideband protocol."),
	opt!("noacl", OptionValue::Toggle, "Disables the NFSACL sideband protocol."),
	opt!("bg", OptionValue::Toggle, "Retries a failed mount in the background."),
	opt!("fg", OptionValue::Toggle, "Fails the mount in the foreground (default)."),
	opt!("clientaddr", OptionValue::String, "Client IP advertised for NFSv4.0 callbacks."),
	opt!("cto", OptionValue::Toggle, "Uses close-to-open cache coherence semantics."),
	opt!("nocto", OptionValue::Toggle, "Disables close-to-open cache coherence."),
	opt!("fsc", OptionValue::Toggle, "Caches read-only pages on local disk via FS-Cache."),
	opt!("nofsc", OptionValue::Toggle, "Disables FS-Cache local disk caching."),
	opt!("hard", OptionValue::Toggle, "Retries requests indefinitely after server timeouts."),
	opt!("intr", OptionValue::Toggle, "Allow interrupts while hung; ignored after kernel 2.6.25."),
	opt!("nointr", OptionValue::Toggle, "Don't allow interrupts; ignored after kernel 2.6.25."),
	opt!("local_lock", OptionValue::Enum(&["all", "flock", "posix", "none"]), "Treats flock/POSIX locks as local only.", "none"),
	opt!("lock", OptionValue::Toggle, "Uses the NLM protocol to lock files on the server."),
	opt!("nolock", OptionValue::Toggle, "Don't use NLM locking; locks are local only."),
	opt!("lookupcache", OptionValue::Enum(&["all", "none", "pos", "positive"]), "Directory-entry cache mode: all, none, pos or positive.", "all"),
	opt!("max_connect", OptionValue::Integer, "Max connections to IPs of one NFSv4.1+ server.", "1"),
	opt!("migration", OptionValue::Toggle, "Uses TSM-compatible ID for NFSv4 migration."),
	opt!("nomigration", OptionValue::Toggle, "Uses legacy client ID string (default)."),
	opt!("minorversion", OptionValue::Integer, "NFSv4 protocol minor version number.", "0"),
	opt!("mounthost", OptionValue::String, "Hostname running the mountd service."),
	opt!("mountport", OptionValue::Integer, "Port of the server's mountd service."),
	opt!("mountproto", OptionValue::Enum(&["udp", "tcp", "udp6", "tcp6"]), "Transport used to contact mountd (udp/tcp).", "udp"),
	opt!("mountvers", OptionValue::Integer, "RPC version used to contact mountd."),
	opt!("namlen", OptionValue::Integer, "Maximum length of a pathname component.", "255"),
	opt!("nconnect", OptionValue::Integer, "Number of connections to establish to the server.", "1"),
	opt!("nfsvers", OptionValue::String, "NFS protocol version (synonym of vers).", "4.2"),
	opt!("noalignwrite", OptionValue::Toggle, "Disables rounding buffered writes to page boundaries."),
	opt!("nordirplus", OptionValue::Toggle, "Disables READDIRPLUS requests."),
	opt!("port", OptionValue::Integer, "Port of the server's NFS service.", "2049"),
	opt!("proto", OptionValue::Enum(&["udp", "udp6", "tcp", "tcp6", "rdma", "rdma6"]), "Transport protocol: udp, tcp or rdma.", "tcp"),
	opt!("rdma", OptionValue::Toggle, "Uses RDMA transport (synonym for proto=rdma)."),
	opt!("rdirplus", OptionValue::Enum(&["none", "force"]), "Enables READDIRPLUS requests."),
	opt!("resvport", OptionValue::Toggle, "Uses privileged source ports (default)."),
	opt!("noresvport", OptionValue::Toggle, "Uses non-privileged source ports."),
	opt!("retrans", OptionValue::Integer, "Number of retries before further recovery action.", "2"),
	opt!("retry", OptionValue::Integer, "Minutes to retry a failed mount before giving up.", "2"),
	opt!("rsize", OptionValue::Integer, "Maximum bytes per network read request."),
	opt!("wsize", OptionValue::Integer, "Maximum bytes per network write request."),
	opt!("sec", OptionValue::Enum(&["none", "sys", "krb5", "krb5i", "krb5p"]), "Security flavors: none, sys, krb5, krb5i or krb5p.", "sys"),
	opt!("sharecache", OptionValue::Toggle, "Shares one data/attribute cache across mounts."),
	opt!("nosharecache", OptionValue::Toggle, "Gives each mount its own cache."),
	opt!("sloppy", OptionValue::Toggle, "Ignores unrecognized mount options."),
	opt!("soft", OptionValue::Toggle, "Returns EIO after retrans retries."),
	opt!("softerr", OptionValue::Toggle, "Returns ETIMEDOUT after retrans retries."),
	opt!("softreval", OptionValue::Toggle, "Serves cached data when revalidation times out."),
	opt!("nosoftreval", OptionValue::Toggle, "Don't fall back to cache when revalidation fails."),
	opt!("tcp", OptionValue::Toggle, "Uses TCP transport (synonym for proto=tcp)."),
	opt!("timeo", OptionValue::Integer, "Deciseconds to wait before retrying a request.", "600"),
	opt!("trunkdiscovery", OptionValue::Toggle, "Probes NFSv4.1 session trunking on new mounts."),
	opt!("notrunkdiscovery", OptionValue::Toggle, "Disables session trunking discovery (default)."),
	opt!("udp", OptionValue::Toggle, "Uses UDP transport (synonym for proto=udp)."),
	opt!("vers", OptionValue::String, "NFS protocol version (alternative to nfsvers).", "4.2"),
	opt!("xprtsec", OptionValue::Enum(&["none", "tls", "mtls"]), "Transport security policy: none, tls or mtls.", "none"),
];

#[rustfmt::skip]
pub const SSHFS_OPTIONS: &[FsOption] = &[
	opt!("reconnect", OptionValue::Toggle, "Automatically reconnects if the connection is interrupted."),
	opt!("delay_connect", OptionValue::Toggle, "Delays connecting until the mountpoint is first accessed."),
	opt!("sshfs_sync", OptionValue::Toggle, "Synchronous writes; slower but more reliable."),
	opt!("no_readahead", OptionValue::Toggle, "Disables speculative read-ahead beyond requested data."),
	opt!("sync_readdir", OptionValue::Toggle, "Synchronous readdir; slower but more reliable."),
	opt!("workaround", OptionValue::Enum(&["rename", "renamexdev", "truncate", "fstat", "buflimit", "createmode"]), "Enables workarounds for broken SFTP servers."),
	opt!("idmap", OptionValue::Enum(&["none", "user", "file"]), "UID/GID mapping mode: none, user or file.", "none"),
	opt!("uidfile", OptionValue::String, "File of username:uid mappings for idmap=file."),
	opt!("gidfile", OptionValue::String, "File of groupname:gid mappings for idmap=file."),
	opt!("nomap", OptionValue::Enum(&["ignore", "error"]), "Handles missing idmap entries: ignore or error.", "error"),
	opt!("ssh_command", OptionValue::String, "Command to run instead of ssh."),
	opt!("ssh_protocol", OptionValue::Integer, "SSH protocol version to use (default 2).", "2"),
	opt!("sftp_server", OptionValue::String, "Path to the sftp server or subsystem.", "sftp"),
	opt!("directport", OptionValue::Integer, "Connects directly to a port, bypassing SSH."),
	opt!("vsock", OptionValue::String, "Connects via vsock to CID:PORT, bypassing SSH."),
	opt!("passive", OptionValue::Toggle, "Communicates over stdin/stdout, bypassing the network."),
	opt!("disable_hardlink", OptionValue::Toggle, "Makes link(2) fail with ENOSYS."),
	opt!("transform_symlinks", OptionValue::Toggle, "Turns absolute symlinks relative on the client."),
	opt!("follow_symlinks", OptionValue::Toggle, "Presents server symlinks as regular files."),
	opt!("no_check_root", OptionValue::Toggle, "Don't check that the remote directory exists."),
	opt!("password_stdin", OptionValue::Toggle, "Reads the password from stdin (for pam_mount only)."),
	opt!("dir_cache", OptionValue::Bool(BoolType::YesNo), "Enables or disables the SSHFS directory cache.", "yes"),
	opt!("dcache_max_size", OptionValue::Integer, "Maximum size of the directory cache.", "10000"),
	opt!("dcache_timeout", OptionValue::Integer, "Timeout in seconds for the directory cache.", "20"),
	opt!("dcache_stat_timeout", OptionValue::Integer, "Timeout for cached attributes.", "20"),
	opt!("dcache_link_timeout", OptionValue::Integer, "Timeout for cached symlinks.", "20"),
	opt!("dcache_dir_timeout", OptionValue::Integer, "Timeout for cached directory names.", "20"),
	opt!("dcache_clean_interval", OptionValue::Integer, "Interval for cleaning the directory cache.", "60"),
	opt!("dcache_min_clean_interval", OptionValue::Integer, "Interval for forced cleaning when full.", "5"),
	opt!("direct_io", OptionValue::Toggle, "Disables the kernel page cache for file content."),
	opt!("max_conns", OptionValue::Integer, "Maximum number of simultaneous SSH connections.", "1"),
	opt!("allow_other", OptionValue::Toggle, "Allows all users to access the mount."),
	opt!("allow_root", OptionValue::Toggle, "Allows root access in addition to the mounter."),
	opt!("auto_unmount", OptionValue::Toggle, "Unmounts automatically when the process exits."),
	opt!("default_permissions", OptionValue::Toggle, "Lets the kernel enforce permission checks."),
	opt!("kernel_cache", OptionValue::Toggle, "Caches file data in kernel without invalidation on open."),
	opt!("auto_cache", OptionValue::Toggle, "Invalidates kernel data cache when file mtime changes."),
	opt!("use_ino", OptionValue::Toggle, "Uses filesystem-provided inode numbers."),
	opt!("nonempty", OptionValue::Toggle, "Allows mounting over non-empty directories (obsolete, now default)."),
	opt!("fsname", OptionValue::String, "Source string shown in /proc/mounts and mtab."),
	opt!("max_read", OptionValue::Integer, "Maximum size of read requests (bytes).", "32768"),
	opt!("max_write", OptionValue::Integer, "Maximum size of write requests (bytes).", "32768"),
];

#[rustfmt::skip]
pub const ISO9660_OPTIONS: &[FsOption] = &[
	opt!("block", OptionValue::Enum(&["512", "1024", "2048"]), "Sets the block size to 512, 1024, or 2048.", "1024"),
	opt!("check", OptionValue::Enum(&["relaxed", "strict", "r", "s"]), "Sets filename case checking (relaxed or strict).", "strict"),
	opt!("conv", OptionValue::String, "Obsolete; may fail or be ignored."),
	opt!("cruft", OptionValue::Toggle, "Ignores high-order bits of file lengths (files limited to 16 MB)."),
	opt!("gid", OptionValue::Integer, "Group ID given to all files.", "0"),
	opt!("iocharset", OptionValue::String, "Charset converting Joliet Unicode names to 8-bit characters.", "iso8859-1"),
	opt!("map", OptionValue::Enum(&["normal", "off", "acorn", "n", "o", "a"]), "Sets non-Rock Ridge name translation (normal, off, or acorn).", "normal"),
	opt!("mode", OptionValue::Octal, "Sets default permission mode for non-Rock Ridge files.", "0555"),
	opt!("nojoliet", OptionValue::Toggle, "Disables Microsoft Joliet extensions."),
	opt!("norock", OptionValue::Toggle, "Disables Rock Ridge extensions."),
	opt!("sbsector", OptionValue::Integer, "Sets the sector where the session begins."),
	opt!("session", OptionValue::Integer, "Selects the session on a multisession CD."),
	opt!("uid", OptionValue::Integer, "User ID given to all files.", "0"),
	opt!("unhide", OptionValue::Toggle, "Shows hidden and associated files."),
	opt!("utf8", OptionValue::Toggle, "Converts 16-bit Unicode CD names to UTF-8."),
];

#[rustfmt::skip]
pub const UDF_OPTIONS: &[FsOption] = &[
	opt!("adinicb", OptionValue::Toggle, "Embeds file data in the inode (default)."),
	opt!("noadinicb", OptionValue::Toggle, "Disables embedding file data in the inode."),
	opt!("anchor", OptionValue::Integer, "Overrides the standard anchor location (default 256).", "256"),
	opt!("bs", OptionValue::Integer, "Sets the block size.", "2048"),
	opt!("dmode", OptionValue::Octal, "Sets default permission mode for directories."),
	opt!("fileset", OptionValue::String, "Unimplemented and ignored."),
	opt!("gid", OptionValue::String, "Sets the default group for all files.", "65534"),
	opt!("iocharset", OptionValue::String, "Sets the NLS character set for filenames.", "utf8"),
	opt!("lastblock", OptionValue::Integer, "Sets the last block of the filesystem."),
	opt!("longad", OptionValue::Toggle, "Uses long UDF address descriptors (default)."),
	opt!("mode", OptionValue::Octal, "Sets default permission mode for non-directory files."),
	opt!("nostrict", OptionValue::Toggle, "Unsets strict UDF conformance."),
	opt!("novrs", OptionValue::Toggle, "Skips volume recognition sequence and mounts anyway."),
	opt!("partition", OptionValue::String, "Unimplemented and ignored."),
	opt!("rootdir", OptionValue::String, "Unimplemented and ignored."),
	opt!("session", OptionValue::Integer, "Selects the session on multisession optical media."),
	opt!("shortad", OptionValue::Toggle, "Uses short UDF address descriptors."),
	opt!("uid", OptionValue::String, "Sets the default user for all files.", "65534"),
	opt!("umask", OptionValue::Octal, "Masks out permissions from all inodes read.", "0"),
	opt!("undelete", OptionValue::Toggle, "Shows deleted files in listings."),
	opt!("unhide", OptionValue::Toggle, "Shows otherwise hidden files."),
	opt!("utf8", OptionValue::Toggle, "Uses the UTF-8 character set."),
	opt!("volume", OptionValue::String, "Unimplemented and ignored."),
];

#[rustfmt::skip]
pub const TMPFS_OPTIONS: &[FsOption] = &[
	opt!("gid", OptionValue::Integer, "Sets initial group ID of the root directory.", "0"),
	opt!("huge", OptionValue::Enum(&["never", "always", "within_size", "advise", "deny", "force"]), "Huge-page policy for files: never, always, within_size, advise.", "never"),
	opt!("mode", OptionValue::Octal, "Sets initial permissions of the root directory.", "01777"),
	opt!("mpol", OptionValue::String, "Sets NUMA memory allocation policy for all files."),
	opt!("noswap", OptionValue::Toggle, "Disables swap; files cannot be swapped out."),
	opt!("nr_blocks", OptionValue::Size, "Same as size, but expressed in blocks of PAGE_SIZE."),
	opt!("nr_inodes", OptionValue::Size, "Maximum number of inodes for this instance."),
	opt!("size", OptionValue::Size, "Upper size limit, e.g. 8m or 50%; 0 removes the limit."),
	opt!("uid", OptionValue::Integer, "Sets initial user ID of the root directory.", "0"),
];

#[rustfmt::skip]
pub const PROC_OPTIONS: &[FsOption] = &[
	opt!("gid", OptionValue::Integer, "Group whose members bypass hidepid access restrictions.", "0"),
	opt!("hidepid", OptionValue::Enum(&["off", "0", "noaccess", "1", "invisible", "2", "ptraceable", "4"]), "Hides /proc/pid entries: 0 off, 1 own dirs, 2 other PIDs invisible.", "0"),
	opt!("pidns", OptionValue::String, "Selects the PID namespace used to translate PIDs (new in 6.16)."),
	opt!("subset", OptionValue::Enum(&["pid"]), "Shows only the pid subset, hiding other top-level proc files."),
];

pub const SYSFS_OPTIONS: &[FsOption] = &[];

#[rustfmt::skip]
pub const DEVPTS_OPTIONS: &[FsOption] = &[
	opt!("gid", OptionValue::Integer, "Sets group of newly created pseudo-terminals."),
	opt!("max", OptionValue::IntegerRange(0, 1048576), "Limits the number of pseudo-terminals in this instance.", "1048576"),
	opt!("mode", OptionValue::Octal, "Sets permissions of newly created pseudo-terminals.", "0600"),
	opt!("newinstance", OptionValue::Toggle, "Creates a private instance with independent pty index space."),
	opt!("ptmxmode", OptionValue::Octal, "Sets the mode of the instance's ptmx device node.", "0000"),
	opt!("uid", OptionValue::Integer, "Sets owner of newly created pseudo-terminals."),
];

#[rustfmt::skip]
pub const CGROUP2_OPTIONS: &[FsOption] = &[
	opt!("favordynmods", OptionValue::Toggle, "Favors dynamic cgroup changes over hot-path fork/exit costs."),
	opt!("memory_hugetlb_accounting", OptionValue::Toggle, "Counts HugeTLB usage in memory controller accounting."),
	opt!("memory_localevents", OptionValue::Toggle, "Reports only local memory.events, excluding subtree counts."),
	opt!("memory_recursiveprot", OptionValue::Toggle, "Applies memory.min/low protection recursively to subtrees."),
	opt!("nsdelegate", OptionValue::Toggle, "Treats cgroup namespaces as delegation boundaries."),
	opt!("pids_localevents", OptionValue::Toggle, "Counts only local fork failures in pids.events.max."),
];

pub const SECURITYFS_OPTIONS: &[FsOption] = &[];

#[rustfmt::skip]
pub const DEBUGFS_OPTIONS: &[FsOption] = &[
	opt!("gid", OptionValue::Integer, "Sets the group of the debugfs mount.", "0"),
	opt!("mode", OptionValue::Octal, "Sets permissions of the mountpoint.", "0700"),
	opt!("uid", OptionValue::Integer, "Sets the owner of the debugfs mount.", "0"),
];

#[rustfmt::skip]
pub const TRACEFS_OPTIONS: &[FsOption] = &[
	opt!("gid", OptionValue::Integer, "Sets the group of the tracefs mount.", "0"),
	opt!("mode", OptionValue::Octal, "Sets permissions of the mountpoint.", "0700"),
	opt!("uid", OptionValue::Integer, "Sets the owner of the tracefs mount.", "0"),
];

pub const CONFIGFS_OPTIONS: &[FsOption] = &[];

pub const MQUEUE_OPTIONS: &[FsOption] = &[];

#[rustfmt::skip]
pub const HUGETLBFS_OPTIONS: &[FsOption] = &[
	opt!("gid", OptionValue::Integer, "Sets group of the filesystem root.", "0"),
	opt!("min_size", OptionValue::Size, "Reserves a minimum of huge-page memory; mount fails if short."),
	opt!("mode", OptionValue::Octal, "Sets permissions of the filesystem root.", "0755"),
	opt!("nr_inodes", OptionValue::Size, "Maximum number of inodes the filesystem can use."),
	opt!("pagesize", OptionValue::Size, "Uses the given huge-page size for this mount."),
	opt!("size", OptionValue::Size, "Maximum huge-page memory the filesystem may use."),
	opt!("uid", OptionValue::Integer, "Sets owner of the filesystem root.", "0"),
];

#[rustfmt::skip]
pub const P9_OPTIONS: &[FsOption] = &[
	opt!("access", OptionValue::Enum(&["user", "any", "client"]), "Access mode: user, <uid>, any or client.", "user"),
	opt!("afid", OptionValue::String, "Authentication fid used by Plan 9 security."),
	opt!("aname", OptionValue::String, "File tree to access on the server."),
	opt!("cache", OptionValue::Enum(&["none", "readahead", "mmap", "loose", "fscache"]), "Caching policy: none, readahead, mmap, loose or fscache.", "none"),
	opt!("cachetag", OptionValue::String, "Tag for the persistent fscache."),
	opt!("debug", OptionValue::Integer, "Debug level as a bitmask.", "0"),
	opt!("dfltgid", OptionValue::String, "Attempts to mount with this gid."),
	opt!("dfltuid", OptionValue::String, "Attempts to mount as this uid."),
	opt!("directio", OptionValue::Toggle, "Bypasses the page cache on all reads/writes."),
	opt!("ignoreqv", OptionValue::Toggle, "Ignores qid.version==0 as a cache marker."),
	opt!("msize", OptionValue::Integer, "Bytes used for 9p packet payload.", "131096"),
	opt!("negtimeout", OptionValue::Integer, "Ms to retain negative dentries in cache.", "0"),
	opt!("noextend", OptionValue::Toggle, "Forces legacy 9p2000 mode (no extensions)."),
	opt!("nodevmap", OptionValue::Toggle, "Don't map special files; shows them as normal files."),
	opt!("noxattr", OptionValue::Toggle, "Don't offer xattr functions on this mount."),
	opt!("port", OptionValue::Integer, "Port to connect to on the remote server.", "564"),
	opt!("rfdno", OptionValue::Integer, "File descriptor for reading with trans=fd."),
	opt!("trans", OptionValue::Enum(&["unix", "tcp", "fd", "virtio", "rdma", "usbg"]), "Transport: unix, tcp, fd, virtio, rdma or usbg.", "virtio"),
	opt!("uname", OptionValue::String, "User name to mount as on the server.", "nobody"),
	opt!("version", OptionValue::Enum(&["9p2000", "9p2000.u", "9p2000.L"]), "9p protocol version: 9p2000, 9p2000.u or 9p2000.L.", "9p2000.L"),
	opt!("wfdno", OptionValue::Integer, "File descriptor for writing with trans=fd."),
];

#[rustfmt::skip]
pub const OVERLAY_OPTIONS: &[FsOption] = &[
	opt!("datadir+", OptionValue::String, "Appends a data-only lower layer directory (new mount API)."),
	opt!("fsync", OptionValue::Enum(&["auto", "strict", "volatile"]), "Controls fsync during copy-up: auto, strict, or volatile.", "auto"),
	opt!("index", OptionValue::Enum(&["on", "off"]), "Use an index to avoid inode collisions: on or off.", "off"),
	opt!("lowerdir", OptionValue::String, "Colon-separated list of lower-layer directories."),
	opt!("metacopy", OptionValue::Enum(&["on", "off"]), "Copy up only metadata first, deferring data copy: on or off.", "off"),
	opt!("nfs_export", OptionValue::Enum(&["on", "off"]), "Make the overlay mountable via NFS: on or off.", "off"),
	opt!("override_creds", OptionValue::Toggle, "Record the caller's credentials for permission checks."),
	opt!("redirect_dir", OptionValue::Enum(&["on", "off", "follow", "nofollow"]), "Redirect directories on rename: on, off, follow, or nofollow.", "off"),
	opt!("upperdir", OptionValue::String, "Directory used as the upper (writable) layer."),
	opt!("userxattr", OptionValue::Toggle, "Use the user.overlay.* xattr namespace instead of trusted.overlay.*."),
	opt!("uuid", OptionValue::Enum(&["null", "off", "on", "auto"]), "Control the overlay UUID and fsid: null, off, on, or auto.", "auto"),
	opt!("verity", OptionValue::Enum(&["off", "on", "require"]), "Verify metacopy digests with fs-verity: off, on, or require.", "off"),
	opt!("volatile", OptionValue::Toggle, "Prefer performance over durability; data not crash-safe."),
	opt!("workdir", OptionValue::String, "Scratch directory on the same filesystem as upperdir."),
	opt!("workdir+", OptionValue::String, "Appends an additional work directory (new mount API)."),
	opt!("xino", OptionValue::Enum(&["on", "off", "auto"]), "Compose unique inode numbers across layers: on, off, or auto.", "auto"),
];

#[rustfmt::skip]
pub const ZFS_OPTIONS: &[FsOption] = &[
	opt!("acl", OptionValue::Toggle, "Enable access control lists (legacy alias for acltype)."),
	opt!("noacl", OptionValue::Toggle, "Disable ACLs (alias for acltype=off)."),
	opt!("acltype", OptionValue::Enum(&["off", "noacl", "nfsv4", "posix", "posixacl"]), "Select ACL type: off, nfsv4, or posix.", "off"),
	opt!("mand", OptionValue::Toggle, "Enable mandatory locking (alias for nbmand=on)."),
	opt!("nomand", OptionValue::Toggle, "Disable mandatory locking (alias for nbmand=off)."),
	opt!("nbmand", OptionValue::Enum(&["on", "off"]), "Mount with non-blocking mandatory locks.", "off"),
	opt!("overlay", OptionValue::Enum(&["on", "off"]), "Allow mounting over a non-empty or busy directory.", "on"),
	opt!("posixacl", OptionValue::Toggle, "Enable POSIX ACLs (alias for acltype=posix)."),
	opt!("relatime", OptionValue::Toggle, "Update access times relative to modify or change time."),
	opt!("norelatime", OptionValue::Toggle, "Disable relative access-time updates (plain atime)."),
	opt!("xattr", OptionValue::Enum(&["sa", "dir", "on", "off"]), "Enable extended attributes.", "sa"),
	opt!("noxattr", OptionValue::Toggle, "Disable extended attributes."),
	opt!("zfsutil", OptionValue::Toggle, "Private flag marking the mount as managed by ZFS."),
];

#[rustfmt::skip]
pub const BCACHEFS_OPTIONS: &[FsOption] = &[
	opt!("acl", OptionValue::Toggle, "Enables POSIX access control lists."),
	opt!("noacl", OptionValue::Toggle, "Disables POSIX access control lists."),
	opt!("background_compression", OptionValue::Enum(&["none", "lz4", "gzip", "zstd"]), "Compression type used for background (rebalance) writes.", "none"),
	opt!("background_target", OptionValue::String, "Target device/label to move data to in the background."),
	opt!("compression", OptionValue::Enum(&["none", "lz4", "gzip", "zstd"]), "Compression type used for foreground writes.", "none"),
	opt!("data_checksum", OptionValue::Enum(&["none", "crc32c", "crc64"]), "Checksum type used for data writes.", "crc32c"),
	opt!("data_replicas", OptionValue::Integer, "Number of replicas to keep for user data.", "1"),
	opt!("degraded", OptionValue::Toggle, "Allows mounting with data degraded (missing a replica).") ,
	opt!("very_degraded", OptionValue::Toggle, "Allows mounting even when data would be missing."),
	opt!("discard", OptionValue::Toggle, "Enables discard/TRIM support on member devices."),
	opt!("erasure_code", OptionValue::Toggle, "Enables erasure coding."),
	opt!("errors", OptionValue::Enum(&["continue", "fix_safe", "panic", "ro"]), "Action to take on filesystem inconsistency: continue, fix_safe, panic, or ro.", "fix_safe"),
	opt!("fix_errors", OptionValue::Toggle, "Fixes fsck errors without prompting during mount."),
	opt!("foreground_target", OptionValue::String, "Preferred target device/label for foreground writes."),
	opt!("fsck", OptionValue::Toggle, "Runs fsck during mount."),
	opt!("inline_data", OptionValue::Toggle, "Enables inline data extents for small files (default on)."),
	opt!("inodes_32bit", OptionValue::Toggle, "Restricts new inode numbers to 32 bits."),
	opt!("journal_flush_delay", OptionValue::Integer, "Milliseconds before an automatic journal commit (default 1000).", "1000"),
	opt!("journal_flush_disabled", OptionValue::Toggle, "Disables journal flush on sync/fsync."),
	opt!("journal_reclaim_delay", OptionValue::Integer, "Milliseconds before automatic journal reclaim.", "100"),
	opt!("metadata_checksum", OptionValue::Enum(&["none", "crc32c", "crc64"]), "Checksum type used for metadata writes.", "crc32c"),
	opt!("metadata_replicas", OptionValue::Integer, "Number of replicas to keep for metadata (journal and btree).", "1"),
	opt!("metadata_target", OptionValue::String, "Preferred target device/label for metadata writes."),
	opt!("nochanges", OptionValue::Toggle, "Issues no writes at all, even for journal replay (super read-only mode)."),
	opt!("noexcl", OptionValue::Toggle, "Doesn't open member devices in exclusive mode."),
	opt!("norecovery", OptionValue::Toggle, "Does not replay the journal on mount (not recommended)."),
	opt!("promote_target", OptionValue::String, "Target device/label data is copied to on read."),
	opt!("ratelimit_errors", OptionValue::Toggle, "Rate-limits error messages during fsck."),
	opt!("read_only", OptionValue::Toggle, "Mounts the filesystem in read-only mode."),
	opt!("shard_inode_numbers", OptionValue::Toggle, "Uses the CPU id for the high bits of new inode numbers."),
	opt!("str_hash", OptionValue::Enum(&["crc32c", "crc64", "siphash"]), "Hash function used for string hash tables (directories and xattrs).", "siphash"),
	opt!("verbose", OptionValue::Toggle, "Prints extra debugging info during mount/recovery."),
	opt!("version_upgrade", OptionValue::Toggle, "Upgrades the on-disk format to the latest version on mount."),
	opt!("wide_macs", OptionValue::Toggle, "Stores full 128-bit cryptographic MACs instead of the default 80-bit."),
	opt!("grpquota", OptionValue::Toggle, "Enables group quotas."),
	opt!("prjquota", OptionValue::Toggle, "Enables project quotas."),
	opt!("usrquota", OptionValue::Toggle, "Enables user quotas."),
];

/// Filesystem-specific options for `fs_type` (without the generic options).
pub fn specific_options(fs_type: &FsType) -> Vec<FsOption> {
	let mut options: Vec<FsOption> = Vec::new();
	match fs_type {
		FsType::Ext2 => options.extend_from_slice(EXT2_OPTIONS),
		FsType::Ext3 => {
			options.extend_from_slice(EXT2_OPTIONS);
			options.extend_from_slice(EXT3_OPTIONS);
		}
		FsType::Ext4 => {
			options.extend_from_slice(EXT2_OPTIONS);
			options.extend_from_slice(EXT3_OPTIONS);
			options.extend_from_slice(EXT4_OPTIONS);
		}
		FsType::Btrfs => options.extend_from_slice(BTRFS_OPTIONS),
		FsType::Xfs => options.extend_from_slice(XFS_OPTIONS),
		FsType::F2fs => options.extend_from_slice(F2FS_OPTIONS),
		FsType::Ntfs3 => options.extend_from_slice(NTFS3_OPTIONS),
		FsType::Vfat => options.extend_from_slice(VFAT_OPTIONS),
		FsType::Exfat => options.extend_from_slice(EXFAT_OPTIONS),
		FsType::Swap => options.extend_from_slice(SWAP_OPTIONS),
		FsType::Cifs | FsType::Smb3 => options.extend_from_slice(CIFS_OPTIONS),
		FsType::Nfs | FsType::Nfs4 => options.extend_from_slice(NFS_OPTIONS),
		FsType::FuseSshfs => options.extend_from_slice(SSHFS_OPTIONS),
		FsType::Iso9660 => options.extend_from_slice(ISO9660_OPTIONS),
		FsType::Udf => options.extend_from_slice(UDF_OPTIONS),
		FsType::Tmpfs | FsType::Devtmpfs => options.extend_from_slice(TMPFS_OPTIONS),
		FsType::Proc => options.extend_from_slice(PROC_OPTIONS),
		FsType::Sysfs => options.extend_from_slice(SYSFS_OPTIONS),
		FsType::Devpts => options.extend_from_slice(DEVPTS_OPTIONS),
		FsType::Cgroup2 => options.extend_from_slice(CGROUP2_OPTIONS),
		FsType::Securityfs => options.extend_from_slice(SECURITYFS_OPTIONS),
		FsType::Debugfs => options.extend_from_slice(DEBUGFS_OPTIONS),
		FsType::Tracefs => options.extend_from_slice(TRACEFS_OPTIONS),
		FsType::Configfs => options.extend_from_slice(CONFIGFS_OPTIONS),
		FsType::Mqueue => options.extend_from_slice(MQUEUE_OPTIONS),
		FsType::Hugetlbfs => options.extend_from_slice(HUGETLBFS_OPTIONS),
		FsType::P9 => options.extend_from_slice(P9_OPTIONS),
		FsType::Overlay => options.extend_from_slice(OVERLAY_OPTIONS),
		FsType::Zfs => options.extend_from_slice(ZFS_OPTIONS),
		FsType::Bcachefs => options.extend_from_slice(BCACHEFS_OPTIONS),
		FsType::Other(_) => {}
	}
	options
}

pub fn options_for(fs_type: &FsType) -> Vec<FsOption> {
	let mut specific = specific_options(fs_type);
	specific.sort_unstable_by_key(|e| e.name.to_ascii_lowercase());
	let mut options: Vec<FsOption> = GENERIC_OPTIONS
		.iter()
		.filter(|generic| !specific.iter().any(|s| s.name == generic.name))
		.copied()
		.collect();
	options.sort_unstable_by_key(|e| e.name.to_ascii_lowercase());
	specific.extend(options);
	specific
}

pub fn lookup(fs_type: &FsType, name: &str) -> Option<FsOption> {
	options_for(fs_type).into_iter().find(|o| o.name == name)
}

#[cfg(test)]
mod tests {
	use super::*;
	use strum::IntoEnumIterator;

	#[test]
	fn lookup_finds_toggle() {
		assert_eq!(
			lookup(&FsType::Ext4, "nofail"),
			Some(FsOption {
				name: "nofail",
				description: "Do not report errors if the device does not exist.",
				value: OptionValue::Toggle,
				default: None
			})
		);
	}

	#[test]
	fn lookup_finds_fs_specific() {
		assert_eq!(
			lookup(&FsType::Ext4, "data"),
			Some(FsOption {
				name: "data",
				description: "Sets data journaling mode: journal, ordered, or writeback.",
				value: OptionValue::Enum(&["journal", "ordered", "writeback"]),
				default: Some("ordered")
			})
		);
		assert_eq!(
			lookup(&FsType::Btrfs, "subvol"),
			Some(FsOption {
				name: "subvol",
				description: "Mounts a subvolume at the given path, not the toplevel.",
				value: OptionValue::String,
				default: Some("@")
			})
		);
	}

	#[test]
	fn no_duplicate_names_per_fs() {
		for fs in FsType::iter() {
			let opts = options_for(&fs);
			let mut names: Vec<&str> = opts.iter().map(|o| o.name).collect();
			names.sort();
			let dupes: Vec<_> = names.windows(2).filter(|w| w[0] == w[1]).collect();
			assert!(dupes.is_empty(), "{fs}: duplicate option names: {dupes:?}");
		}
	}

	#[test]
	fn bool_type_parse() {
		assert_eq!(BoolType::YesNo.parse("yes"), Some(true));
		assert_eq!(BoolType::YesNo.parse("NO"), Some(false));
		assert_eq!(BoolType::TrueFalse.parse("TRUE"), Some(true));
		assert_eq!(BoolType::OneZero.parse("0"), Some(false));
		assert_eq!(BoolType::OneZero.parse("garbage"), None);
	}
}

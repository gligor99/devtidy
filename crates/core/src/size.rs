//! Honest size measurement.
//!
//! Three things make naive `st_size` sums lie on modern filesystems:
//! sparse files (Docker's VM disk), hard links (pnpm and bun stores) and
//! copy-on-write clones (APFS). We count allocated blocks, count each inode
//! once, and separately estimate what would actually be freed. Clones cannot
//! be detected cheaply, so the cleaner verifies real numbers with `statfs`.

use std::collections::HashMap;
use std::fs::Metadata;
use std::io;
use std::path::Path;

use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Measurement {
    /// Bytes allocated on disk, each inode counted once.
    #[cfg_attr(feature = "specta", specta(type = specta_typescript::Number))]
    pub allocated: u64,
    /// Bytes that removal is expected to free. Lower than `allocated` when
    /// files are hard-linked from somewhere outside the measured paths.
    #[cfg_attr(feature = "specta", specta(type = specta_typescript::Number))]
    pub reclaimable: u64,
    /// Number of entries visited (files, directories, symlinks).
    #[cfg_attr(feature = "specta", specta(type = specta_typescript::Number))]
    pub entries: u64,
    /// Number of entries we were not allowed to read.
    #[cfg_attr(feature = "specta", specta(type = specta_typescript::Number))]
    pub denied: u64,
}

impl Measurement {
    pub fn add(&mut self, other: Self) {
        self.allocated += other.allocated;
        self.reclaimable += other.reclaimable;
        self.entries += other.entries;
        self.denied += other.denied;
    }
}

/// Measures every root together, so a file hard-linked between two roots of
/// the same rule is counted once and treated as reclaimable.
#[must_use]
pub fn measure<P: AsRef<Path>>(roots: &[P]) -> Measurement {
    let mut total = Measurement::default();
    let mut links: HashMap<(u64, u64), LinkedInode> = HashMap::new();

    for root in roots {
        // Never follow symlinks and never cross into another mount.
        let walker = WalkDir::new(root.as_ref())
            .follow_links(false)
            .same_file_system(true);
        for entry in walker {
            let meta = match entry
                .map_err(io::Error::from)
                .and_then(|e| e.metadata().map_err(io::Error::from))
            {
                Ok(meta) => meta,
                Err(err) if err.kind() == io::ErrorKind::PermissionDenied => {
                    total.denied += 1;
                    continue;
                }
                // Vanished mid-walk or unreadable for another reason: skip.
                Err(_) => continue,
            };
            total.entries += 1;
            let bytes = allocated_bytes(&meta);
            if let Some(key) = link_key(&meta) {
                let seen = links.entry(key).or_insert(LinkedInode {
                    bytes,
                    nlink: nlink(&meta),
                    seen: 0,
                });
                seen.seen += 1;
            } else {
                total.allocated += bytes;
                total.reclaimable += bytes;
            }
        }
    }

    for inode in links.values() {
        total.allocated += inode.bytes;
        if inode.seen >= inode.nlink {
            total.reclaimable += inode.bytes;
        }
    }
    total
}

struct LinkedInode {
    bytes: u64,
    nlink: u64,
    seen: u64,
}

#[cfg(unix)]
fn allocated_bytes(meta: &Metadata) -> u64 {
    use std::os::unix::fs::MetadataExt;
    // st_blocks is always in 512-byte units, regardless of the fs block size.
    meta.blocks() * 512
}

#[cfg(not(unix))]
fn allocated_bytes(meta: &Metadata) -> u64 {
    meta.len()
}

#[cfg(unix)]
fn nlink(meta: &Metadata) -> u64 {
    use std::os::unix::fs::MetadataExt;
    meta.nlink()
}

#[cfg(not(unix))]
fn nlink(_meta: &Metadata) -> u64 {
    1
}

/// Only regular files with more than one link need inode tracking.
#[cfg(unix)]
fn link_key(meta: &Metadata) -> Option<(u64, u64)> {
    use std::os::unix::fs::MetadataExt;
    (meta.is_file() && meta.nlink() > 1).then(|| (meta.dev(), meta.ino()))
}

#[cfg(not(unix))]
fn link_key(_meta: &Metadata) -> Option<(u64, u64)> {
    None
}

#[cfg(all(test, unix))]
mod tests {
    use std::fs::{self, File};
    use std::io::Write;
    use std::process::Command;

    use super::*;

    const MIB: u64 = 1024 * 1024;

    fn write_file(path: &Path, bytes: u64) {
        let mut file = File::create(path).unwrap();
        file.write_all(&vec![7u8; usize::try_from(bytes).unwrap()])
            .unwrap();
        file.sync_all().unwrap();
    }

    #[test]
    fn sparse_files_count_allocated_blocks_not_length() {
        let dir = tempfile::tempdir().unwrap();
        let file = File::create(dir.path().join("disk.raw")).unwrap();
        file.set_len(512 * MIB).unwrap();

        let m = measure(&[dir.path()]);
        assert!(
            m.allocated < MIB,
            "sparse file reported as {} bytes",
            m.allocated
        );
    }

    #[test]
    fn hard_links_inside_the_item_are_counted_once_and_reclaimable() {
        let dir = tempfile::tempdir().unwrap();
        write_file(&dir.path().join("a"), 4 * MIB);
        fs::hard_link(dir.path().join("a"), dir.path().join("b")).unwrap();

        let m = measure(&[dir.path()]);
        assert!(m.allocated >= 4 * MIB && m.allocated < 5 * MIB);
        assert_eq!(m.allocated, m.reclaimable);
    }

    #[test]
    fn hard_links_from_outside_are_not_reclaimable() {
        let root = tempfile::tempdir().unwrap();
        let store = root.path().join("store");
        let project = root.path().join("project/node_modules");
        fs::create_dir_all(&store).unwrap();
        fs::create_dir_all(&project).unwrap();
        write_file(&store.join("pkg"), 4 * MIB);
        fs::hard_link(store.join("pkg"), project.join("pkg")).unwrap();

        let m = measure(&[&project]);
        assert!(m.allocated >= 4 * MIB);
        assert!(
            m.reclaimable < MIB,
            "linked file reported reclaimable: {}",
            m.reclaimable
        );
    }

    #[test]
    fn symlinks_are_not_followed() {
        let root = tempfile::tempdir().unwrap();
        let outside = root.path().join("outside");
        let item = root.path().join("item");
        fs::create_dir_all(&outside).unwrap();
        fs::create_dir_all(&item).unwrap();
        write_file(&outside.join("big"), 8 * MIB);
        std::os::unix::fs::symlink(&outside, item.join("link")).unwrap();

        assert!(measure(&[&item]).allocated < MIB);
    }

    #[test]
    fn matches_du() {
        let dir = tempfile::tempdir().unwrap();
        for i in 0..20 {
            let sub = dir.path().join(format!("d{}", i % 4));
            fs::create_dir_all(&sub).unwrap();
            write_file(&sub.join(format!("f{i}")), (i + 1) * 37 * 1024);
        }
        fs::hard_link(dir.path().join("d0/f0"), dir.path().join("d1/link")).unwrap();

        let out = Command::new("du")
            .arg("-sk")
            .arg(dir.path())
            .output()
            .unwrap();
        let du_kib: u64 = String::from_utf8(out.stdout)
            .unwrap()
            .split_whitespace()
            .next()
            .unwrap()
            .parse()
            .unwrap();

        assert_eq!(measure(&[dir.path()]).allocated / 1024, du_kib);
    }
}

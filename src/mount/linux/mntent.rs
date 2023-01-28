//! Iterate over mtab/fstab

use crate::{NixPath, Result};
use libc::{endmntent, getmntent, mntent, setmntent, FILE};
use std::ffi::{CStr, CString};

#[derive(Debug)]
/// A wrapper for `libc::mntent`, an iterator for `MountEntry`
pub struct MountEntries {
    file: *mut FILE,
}

impl MountEntries {
    /// Creates a new `MountEntry iterator, opening the given mtab/fstab
    ///
    /// # Arguments
    /// - `path` - Path to mtab/fstab, e.g. `/etc/mtab`.
    /// - `mode` - Mode as for `fopen(3)`, e.g. `"r"` or `"a+"`.
    ///
    /// # See Also
    /// [`getmntent`](https://www.man7.org/linux/man-pages/man3/getmntent.3.html)
    /// [`fopen`](https://www.man7.org/linux/man-pages/man3/fopen.3.html)
    pub fn new<P: ?Sized + NixPath>(path: &P, mode: String) -> Result<Self> {
        let mode = CString::new(mode).unwrap();
        let file = path.with_nix_path(|cstr| unsafe {
            setmntent(cstr.as_ptr(), mode.as_ptr())
        })?;

        if file.is_null() {
            panic!("The setmntent call returned NULL");
        }

        Ok(MountEntries { file })
    }
}

impl Drop for MountEntries {
    fn drop(&mut self) {
        unsafe { endmntent(self.file) };
    }
}

/// Represents an entry in mtab/fstab.
#[derive(Debug, PartialEq)]
pub struct MountEntry {
    /// 1. name of the filesystem (e.g. the device)
    pub fs_name: String,
    /// 2. path to the mounted directory
    pub mount_dir: String,
    /// 3. type of the filesystem
    pub fs_type: String,
    /// 4. options passed to `mount`
    pub options: String,
    /// 5. option for `dump(8)`
    pub dump_freq: i32,
    /// 6. option for `fsck(8)`
    pub pass_no: i32,
}

impl From<&mntent> for MountEntry {
    fn from(value: &mntent) -> Self {
        unsafe {
            MountEntry {
                fs_name: CStr::from_ptr(value.mnt_fsname)
                    .to_string_lossy()
                    .into_owned(),
                mount_dir: CStr::from_ptr(value.mnt_dir)
                    .to_string_lossy()
                    .into_owned(),
                fs_type: CStr::from_ptr(value.mnt_type)
                    .to_string_lossy()
                    .into_owned(),
                options: CStr::from_ptr(value.mnt_opts)
                    .to_string_lossy()
                    .into_owned(),
                dump_freq: value.mnt_freq,
                pass_no: value.mnt_passno,
            }
        }
    }
}

impl Iterator for MountEntries {
    type Item = MountEntry;

    fn next(&mut self) -> Option<Self::Item> {
        unsafe { getmntent(self.file).as_ref().map(MountEntry::from) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::prelude::*;
    use tempfile::{self, NamedTempFile};

    #[test]
    fn test_iterate_mtab() {
        const CONTENTS: &[u8] = concat!(
            "devtmpfs /dev devtmpfs rw,nosuid,mode=755 0 0\n",
            "tmpfs /dev/shm tmpfs rw,nosuid,nodev 0 0\n",
        )
        .as_bytes();
        let mut tmp = NamedTempFile::new().unwrap();
        tmp.write_all(CONTENTS).unwrap();

        let mut mount_entries =
            MountEntries::new(tmp.path(), "r".to_string()).unwrap();

        assert_eq!(
            mount_entries.next(),
            Some(MountEntry {
                fs_name: "devtmpfs".to_string(),
                mount_dir: "/dev".to_string(),
                fs_type: "devtmpfs".to_string(),
                options: "rw,nosuid,mode=755".to_string(),
                dump_freq: 0,
                pass_no: 0
            })
        );

        assert_eq!(
            mount_entries.next(),
            Some(MountEntry {
                fs_name: "tmpfs".to_string(),
                mount_dir: "/dev/shm".to_string(),
                fs_type: "tmpfs".to_string(),
                options: "rw,nosuid,nodev".to_string(),
                dump_freq: 0,
                pass_no: 0
            })
        );

        assert_eq!(mount_entries.next(), None);
    }
}

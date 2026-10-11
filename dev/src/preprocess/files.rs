// Translated from preprocess/source/source_files.cpp (host identity). See NOTICE.
//! Canonical host file identity shared by pragma registration and future inclusion.
/// Host identity follows stat, including symlinks; only regular files qualify.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct FileIdentity {
    device: u64,
    inode: u64,
}
impl FileIdentity {
    /// Construct the identity reported by the host file boundary.
    #[must_use]
    pub fn new(device: u64, inode: u64) -> Self {
        Self { device, inode }
    }
}

/// Demand-only host service; a missing or nonregular file returns None.
pub type FileIdentityQuery = fn(&[u8]) -> Option<FileIdentity>;

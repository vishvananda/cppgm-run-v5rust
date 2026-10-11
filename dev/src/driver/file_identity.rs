// Translated from preprocess/source/source_files.cpp (GetFileIdentity). See NOTICE.
//! Host stat boundary; preprocessing receives only typed identity facts.
use crate::preprocess::files::FileIdentity;
use std::{
    ffi::OsStr,
    io::Read,
    os::unix::{ffi::OsStrExt, fs::MetadataExt},
};

pub(super) fn read(path: &[u8]) -> Option<Vec<u8>> {
    let end = path.iter().position(|&c| c == 0).unwrap_or(path.len());
    let mut file = std::fs::File::open(OsStr::from_bytes(&path[..end])).ok()?;
    // ReadFrom reserves the image plus a possible final newline, and keeps bytes
    // delivered before an input failure. Only failure to open is a missing file.
    let capacity = file
        .metadata()
        .ok()
        .and_then(|metadata| usize::try_from(metadata.len()).ok())
        .and_then(|size| size.checked_add(1));
    let mut bytes = capacity.map_or_else(Vec::new, Vec::with_capacity);
    let _read_status = file.read_to_end(&mut bytes);
    Some(bytes)
}

pub(super) fn query(path: &[u8]) -> Option<FileIdentity> {
    // The frozen stat call receives a C string, including presumed names from #line.
    let end = path.iter().position(|&c| c == 0).unwrap_or(path.len());
    let metadata = std::fs::metadata(OsStr::from_bytes(&path[..end])).ok()?;
    metadata
        .is_file()
        .then_some(FileIdentity::new(metadata.dev(), metadata.ino()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_preserves_aliases_nul_termination_and_regular_file_requirement() {
        let path = file!();
        let identity = query(path.as_bytes()).unwrap();
        assert!(query(format!("./{path}").as_bytes()) == Some(identity));
        assert!(query(format!("{path}\0suffix").as_bytes()) == Some(identity));
        assert_eq!(
            read(path.as_bytes()),
            read(format!("{path}\0suffix").as_bytes())
        );
        assert!(read(b"").is_none());
        assert!(query(b"src/driver").is_none());
        assert!(query(b"").is_none());
    }
}

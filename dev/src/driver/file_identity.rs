// Translated from preprocess/source/source_files.cpp (GetFileIdentity). See NOTICE.
//! Host source-file boundary; preprocessing receives typed identity and read results.
use crate::preprocess::files::FileIdentity;
use crate::support::source::SourceReadError;
use std::{
    ffi::OsStr,
    io::{Read, Seek, SeekFrom},
    os::unix::{ffi::OsStrExt, fs::MetadataExt},
};

/// Read a source after its regular-file identity has been checked.
/// Missing or inaccessible files return `None`.
/// # Errors
/// Returns an opened file's seek, allocation or read failure.
pub fn read(path: &[u8]) -> Result<Option<Vec<u8>>, SourceReadError> {
    let end = path.iter().position(|&c| c == 0).unwrap_or(path.len());
    let Ok(mut file) = std::fs::File::open(OsStr::from_bytes(&path[..end])) else {
        return Ok(None);
    };
    let start = file.stream_position();
    let end = file.seek(SeekFrom::End(0));
    let capacity = if let (Ok(start), Ok(end)) = (start, end)
        && end >= start
    {
        if file.seek(SeekFrom::Start(start)).ok() != Some(start) {
            return Err(SourceReadError::Rewind);
        }
        Some(
            usize::try_from(end - start)
                .ok()
                .and_then(|size| size.checked_add(1))
                .ok_or(SourceReadError::Capacity)?,
        )
    } else {
        None
    };
    read_image(&mut file, capacity).map(Some)
}

fn read_image(reader: &mut impl Read, capacity: Option<usize>) -> Result<Vec<u8>, SourceReadError> {
    let mut bytes = Vec::new();
    if let Some(capacity) = capacity {
        if isize::try_from(capacity).is_err() {
            return Err(SourceReadError::Capacity);
        }
        bytes
            .try_reserve_exact(capacity)
            .map_err(|_| SourceReadError::Allocation)?;
    }
    reader.read_to_end(&mut bytes).map_err(|error| {
        if error.kind() == std::io::ErrorKind::OutOfMemory && error.raw_os_error().is_none() {
            SourceReadError::Allocation
        } else {
            SourceReadError::Read(error)
        }
    })?;
    Ok(bytes)
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
            read(path.as_bytes()).unwrap(),
            read(format!("{path}\0suffix").as_bytes()).unwrap()
        );
        assert!(read(b"").unwrap().is_none());
        assert!(query(b"src/driver").is_none());
        assert!(query(b"").is_none());
    }

    #[test]
    fn opened_read_failure_cannot_publish_a_partial_image() {
        struct FailsAfterPrefix(bool);
        impl Read for FailsAfterPrefix {
            fn read(&mut self, bytes: &mut [u8]) -> std::io::Result<usize> {
                if bytes.is_empty() {
                    return Ok(0);
                }
                if self.0 {
                    return Err(std::io::Error::from_raw_os_error(5));
                }
                self.0 = true;
                bytes[0] = b'x';
                Ok(1)
            }
        }
        let error = read_image(&mut FailsAfterPrefix(false), Some(2)).unwrap_err();
        assert!(matches!(error, SourceReadError::Read(_)));
        assert_eq!(
            error.to_string(),
            "basic_filebuf::xsgetn error reading the file: Input/output error"
        );
        assert!(matches!(
            read_image(&mut std::io::empty(), Some(usize::MAX)),
            Err(SourceReadError::Capacity)
        ));
    }
}

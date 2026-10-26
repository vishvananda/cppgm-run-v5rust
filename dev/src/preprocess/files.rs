// Translated from preprocess/source/source_files.cpp. See NOTICE.
//! Unit-owned physical files, cached paths/identities and whole-file guard facts.
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

/// A missing file is distinct from a failure while reading an opened image.
pub type FileReadResult = Result<Option<Vec<u8>>, crate::support::source::SourceReadError>;

/// Host source-file operations supplied explicitly at the driver boundary.
#[derive(Clone, Copy)]
pub struct FileServices {
    pub identity: FileIdentityQuery,
    pub read: fn(&[u8]) -> FileReadResult,
}

use super::error::PreprocessError;
use crate::{
    lex::identifiers::NameId,
    support::source::{SourceBuffer, SourceImage},
};
use std::{collections::HashMap, rc::Rc};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct FileId(u32);
struct SourceFile<'a> {
    image: SourceImage<'a>,
    identity: Option<FileIdentity>,
    guard: Option<NameId>,
}
/// Unit-owned physical images. Path misses and successful identity aliases are cached.
pub(crate) struct SourceFiles<'a> {
    files: Vec<SourceFile<'a>>,
    paths: HashMap<Vec<u8>, Option<FileId>>,
    identities: HashMap<FileIdentity, FileId>,
    services: FileServices,
}
impl<'a> SourceFiles<'a> {
    pub(crate) fn counts(&self) -> (usize, usize) {
        (
            self.files.len(),
            self.files.iter().map(|file| file.image.bytes().len()).sum(),
        )
    }
    pub(crate) fn new(source: &'a SourceBuffer, name: &[u8], services: FileServices) -> Self {
        let identity = (services.identity)(name);
        let id = FileId(0);
        Self {
            files: vec![SourceFile {
                image: SourceImage::Borrowed(source),
                identity,
                guard: None,
            }],
            paths: HashMap::from([(name.to_vec(), Some(id))]),
            identities: identity
                .map_or_else(HashMap::new, |identity| HashMap::from([(identity, id)])),
            services,
        }
    }
    pub(crate) fn primary() -> FileId {
        FileId(0)
    }
    pub(crate) fn add_text(&mut self, bytes: Vec<u8>) -> Result<FileId, PreprocessError> {
        let id = FileId(
            u32::try_from(self.files.len())
                .map_err(|_| PreprocessError::Limit("too many source files"))?,
        );
        self.files.push(SourceFile {
            image: SourceImage::Shared(Rc::new(source_image(bytes)?)),
            identity: None,
            guard: None,
        });
        Ok(id)
    }
    pub(crate) fn image(&self, id: FileId) -> SourceImage<'a> {
        self.files[id.0 as usize].image.clone()
    }
    pub(crate) fn identity(&self, id: FileId) -> Option<FileIdentity> {
        self.files[id.0 as usize].identity
    }
    pub(crate) fn guard(&self, id: FileId) -> Option<NameId> {
        self.files[id.0 as usize].guard
    }
    pub(crate) fn set_guard(&mut self, id: FileId, guard: NameId) {
        self.files[id.0 as usize].guard = Some(guard);
    }
    pub(crate) fn open(&mut self, path: &[u8]) -> Result<Option<FileId>, PreprocessError> {
        if let Some(&id) = self.paths.get(path) {
            return Ok(id);
        }
        let id = self.find_or_read(path)?;
        self.paths.insert(path.to_vec(), id);
        Ok(id)
    }
    fn find_or_read(&mut self, path: &[u8]) -> Result<Option<FileId>, PreprocessError> {
        let Some(identity) = (self.services.identity)(path) else {
            return Ok(None);
        };
        if let Some(&id) = self.identities.get(&identity) {
            return Ok(Some(id));
        }
        let Some(bytes) = (self.services.read)(path).map_err(PreprocessError::Read)? else {
            return Ok(None);
        };
        let image = SourceImage::Shared(Rc::new(source_image(bytes)?));
        let id = FileId(
            u32::try_from(self.files.len())
                .map_err(|_| PreprocessError::Limit("too many source files"))?,
        );
        self.files.push(SourceFile {
            image,
            identity: Some(identity),
            guard: None,
        });
        self.identities.insert(identity, id);
        Ok(Some(id))
    }
}

fn source_image(bytes: Vec<u8>) -> Result<SourceBuffer, PreprocessError> {
    SourceBuffer::new(bytes).map_err(|message| {
        PreprocessError::Read(crate::support::source::SourceReadError::Image(message))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn read_failures_publish_neither_images_nor_cached_misses() {
        static READS: AtomicUsize = AtomicUsize::new(0);
        let source = SourceBuffer::new(Vec::new()).unwrap();
        let mut files = SourceFiles::new(
            &source,
            b"primary",
            FileServices {
                identity: |path| Some(FileIdentity::new(1, u64::from(path != b"primary") + 1)),
                read: |_| {
                    if READS.fetch_add(1, Ordering::Relaxed) == 0 {
                        Err(crate::support::source::SourceReadError::Read(
                            std::io::Error::from_raw_os_error(5),
                        ))
                    } else {
                        Ok(Some(b"header\n".to_vec()))
                    }
                },
            },
        );
        assert!(matches!(
            files.open(b"header"),
            Err(PreprocessError::Read(_))
        ));
        assert_eq!(files.counts().0, 1);
        let alias = files.open(b"alias").unwrap().unwrap();
        assert!(files.open(b"header").unwrap() == Some(alias));
        assert_eq!(READS.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn aliases_share_images_and_line_indices_and_misses_stay_cached() {
        static QUERIES: AtomicUsize = AtomicUsize::new(0);
        static READS: AtomicUsize = AtomicUsize::new(0);
        fn identity(path: &[u8]) -> Option<FileIdentity> {
            QUERIES.fetch_add(1, Ordering::Relaxed);
            match path {
                b"primary" => Some(FileIdentity::new(1, 1)),
                b"header" | b"alias" => Some(FileIdentity::new(1, 2)),
                _ => None,
            }
        }
        fn read(path: &[u8]) -> Option<Vec<u8>> {
            READS.fetch_add(1, Ordering::Relaxed);
            (path == b"header").then(|| b"first\nsecond\n".to_vec())
        }
        let primary = SourceBuffer::new(Vec::new()).unwrap();
        let mut files = SourceFiles::new(
            &primary,
            b"primary",
            FileServices {
                identity,
                read: |path| Ok(read(path)),
            },
        );
        let first = files.open(b"header").unwrap().unwrap();
        let alias = files.open(b"alias").unwrap().unwrap();
        assert!(first == alias);
        assert!(files.open(b"header").unwrap() == Some(first));
        assert!(files.open(b"missing").unwrap().is_none());
        assert!(files.open(b"missing").unwrap().is_none());
        assert_eq!(QUERIES.load(Ordering::Relaxed), 4);
        assert_eq!(READS.load(Ordering::Relaxed), 1);
        let image = files.image(first);
        assert!(!image.has_line_index());
        let index = image.line_starts();
        assert!(std::ptr::eq(
            index.as_ptr(),
            files.image(alias).line_starts().as_ptr()
        ));
        assert_eq!(files.files.len(), 2);
    }
}

//! Host file creation for bounded serialized views. See NOTICE.
use std::{
    fs::{File, OpenOptions},
    io,
    os::unix::fs::OpenOptionsExt,
};

pub(super) fn anonymous() -> io::Result<File> {
    // Linux x86-64 O_TMPFILE includes O_DIRECTORY. As with the frozen tmpfile(),
    // the owned descriptor names no directory entry and closing it reclaims it.
    const ANONYMOUS_FILE: i32 = 0o20_200_000;
    OpenOptions::new()
        .read(true)
        .write(true)
        .mode(0o600)
        .custom_flags(ANONYMOUS_FILE)
        .open("/tmp")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lowir::text_storage::{BUFFER_BYTES, DefinitionBytes};
    use std::io::Write;

    #[test]
    fn spill_failure_is_reported_after_lowering_at_the_output_boundary() {
        fn unavailable() -> io::Result<File> {
            Err(io::Error::from(io::ErrorKind::StorageFull))
        }
        let mut storage = DefinitionBytes::default();
        storage.set_spill_factory(unavailable);
        let text = vec![1; BUFFER_BYTES + 7];
        storage.write_all(&text).unwrap();
        storage.write_all(b"later definition").unwrap();
        assert_eq!(storage.len(), text.len() + b"later definition".len());
        assert!(
            storage
                .copy_range(0..storage.len(), &mut io::sink())
                .is_err()
        );
    }

    #[test]
    fn reordered_ranges_cross_spill_and_live_buffer_without_retaining_text() {
        let source: Vec<_> = (0..BUFFER_BYTES * 2 + 31)
            .map(|index| u8::try_from(index % 251).unwrap())
            .collect();
        let mut storage = DefinitionBytes::default();
        storage.set_spill_factory(anonymous);
        let split = BUFFER_BYTES / 2 + 13;
        storage.write_all(&source[..split]).unwrap();
        storage.write_all(&source[split..]).unwrap();
        assert!(storage.retained_capacity() <= BUFFER_BYTES);
        assert_eq!(storage.len(), source.len());
        for range in [
            BUFFER_BYTES * 2 - 7..source.len(),
            0..BUFFER_BYTES + 1,
            9..9,
            BUFFER_BYTES - 3..BUFFER_BYTES + 3,
        ] {
            let mut output = Vec::new();
            storage.copy_range(range.clone(), &mut output).unwrap();
            assert_eq!(output, source[range]);
        }
        assert!(
            storage
                .copy_range(0..source.len() + 1, &mut io::sink())
                .is_err()
        );
        let tail = vec![19; BUFFER_BYTES];
        storage.write_all(&tail).unwrap();
        let mut appended = Vec::new();
        storage
            .copy_range(source.len()..storage.len(), &mut appended)
            .unwrap();
        assert_eq!(appended, tail);
    }
}

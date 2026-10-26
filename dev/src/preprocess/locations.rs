// Translated from source/source_files.cpp and preprocessor.cpp. See NOTICE.
//! Physical line index and presumed-location history for one source inclusion.
use super::error::PreprocessError;
use crate::support::source::{SourceBuffer, SourceError, SourceImage, SourceOffset};
use std::rc::Rc;

pub(crate) trait SourcePositions {
    fn source_file(&self, location: SourceOffset) -> Rc<[u8]>;
    fn source_column(&self, location: SourceOffset) -> u32;
    fn source_line(&self, location: SourceOffset) -> Result<i64, PreprocessError>;
    fn in_system_header(&self, location: SourceOffset) -> bool;
}

struct LineState {
    delta: i64,
    name: Rc<[u8]>,
}

struct EarlierState {
    until: SourceOffset,
    state: LineState,
}
pub(crate) struct Locations<'a> {
    source: SourceImage<'a>,
    current: LineState,
    earlier: Vec<EarlierState>,
    file_literal: Option<Rc<[u8]>>,
}
impl<'a> Locations<'a> {
    pub(crate) fn new(source: &'a SourceBuffer, name: &[u8]) -> Self {
        Self::with_image(SourceImage::Borrowed(source), name)
    }
    pub(crate) fn with_image(source: SourceImage<'a>, name: &[u8]) -> Self {
        Self {
            source,
            current: LineState {
                delta: 0,
                name: name.into(),
            },
            earlier: Vec::new(),
            file_literal: None,
        }
    }
    fn state_at(&self, location: SourceOffset) -> &LineState {
        self.earlier
            .get(
                self.earlier
                    .partition_point(|state| state.until <= location),
            )
            .map_or(&self.current, |earlier| &earlier.state)
    }
    pub(crate) fn name_at(&self, location: SourceOffset) -> Rc<[u8]> {
        self.state_at(location).name.clone()
    }
    pub(crate) fn column_at(&self, location: SourceOffset) -> u32 {
        let start = self.starts()[self
            .starts()
            .partition_point(|&start| start <= location.0)
            .saturating_sub(1)];
        location.0 - start + 1
    }
    pub(crate) fn line_at(&self, location: SourceOffset) -> Result<i64, PreprocessError> {
        Ok((self.physical_line(location)? + self.state_at(location).delta).max(1))
    }
    pub(crate) fn line_and_name(
        &self,
        location: SourceOffset,
    ) -> Result<(Rc<[u8]>, i64), PreprocessError> {
        Ok((self.name_at(location), self.line_at(location)?))
    }
    fn starts(&self) -> &[u32] {
        self.source.line_starts()
    }
    fn physical_line(&self, location: SourceOffset) -> Result<i64, PreprocessError> {
        i64::try_from(self.starts().partition_point(|&start| start <= location.0))
            .map_err(|_| PreprocessError::Invariant("physical line index exceeds source bounds"))
    }
    pub(crate) fn presumed_line(&self, location: SourceOffset) -> Result<i64, PreprocessError> {
        Ok((self.physical_line(location)? + self.current.delta).max(1))
    }
    pub(crate) fn file_literal(&mut self) -> Rc<[u8]> {
        self.file_literal
            .get_or_insert_with(|| {
                let mut text = vec![b'"'];
                for &byte in self.current.name.iter() {
                    match byte {
                        b'"' | b'\\' => {
                            text.push(b'\\');
                            text.push(byte);
                        }
                        0..=0x1f | 0x7f => text.extend_from_slice(&[
                            b'\\',
                            b'0' + (byte >> 6),
                            b'0' + ((byte >> 3) & 7),
                            b'0' + (byte & 7),
                        ]),
                        _ => text.push(byte),
                    }
                }
                text.push(b'"');
                text.into()
            })
            .clone()
    }
    pub(crate) fn presumed_name(&self) -> &[u8] {
        &self.current.name
    }
    pub(crate) fn set_line(
        &mut self,
        newline: SourceOffset,
        number: u32,
        name: Option<Vec<u8>>,
    ) -> Result<(), PreprocessError> {
        let next_physical = self.physical_line(newline)? + 1;
        if name.is_some() {
            self.file_literal = None;
        }
        let new_name = name.map_or_else(|| self.current.name.clone(), Rc::from);
        let previous = std::mem::replace(
            &mut self.current,
            LineState {
                delta: i64::from(number) - next_physical,
                name: new_name,
            },
        );
        self.earlier.push(EarlierState {
            until: newline,
            state: previous,
        });
        Ok(())
    }
    pub(crate) fn describe(&self, error: &SourceError) -> Result<Vec<u8>, PreprocessError> {
        let offset = error
            .offset
            .index()
            .clamp(self.source.begin().index(), self.source.bytes().len());
        let location =
            SourceOffset(u32::try_from(offset).map_err(|_| {
                PreprocessError::Invariant("diagnostic offset exceeds source bounds")
            })?);
        let physical = self.physical_line(location)?;
        let start = self.starts()[self
            .starts()
            .partition_point(|&start| start <= location.0)
            .saturating_sub(1)];
        let state = self
            .earlier
            .get(
                self.earlier
                    .partition_point(|state| state.until <= location),
            )
            .map_or(&self.current, |earlier| &earlier.state);
        let mut text = state.name.to_vec();
        text.extend_from_slice(
            format!(
                ":{}:{}: {}",
                (physical + state.delta).max(1),
                offset - start as usize + 1,
                error.message
            )
            .as_bytes(),
        );
        Ok(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filename_queries_do_not_demand_the_physical_line_index() {
        let source = SourceBuffer::new(b"first\nsecond\n".to_vec()).unwrap();
        let locations = Locations::new(&source, b"file.cpp");
        assert!(!source.has_line_index());
        assert_eq!(&*locations.name_at(SourceOffset(7)), b"file.cpp");
        assert!(!source.has_line_index());
        assert_eq!(locations.line_at(SourceOffset(7)).unwrap(), 2);
        assert!(source.has_line_index());
    }

    #[test]
    fn line_index_is_lazy_and_history_preserves_original_diagnostic_state() {
        let source = SourceBuffer::new(b"a\nb\nc\nd\n".to_vec()).unwrap();
        let mut locations = Locations::new(&source, b"physical");
        assert!(!source.has_line_index());
        let initial_literal = locations.file_literal();
        assert!(Rc::ptr_eq(&initial_literal, &locations.file_literal()));
        assert!(!source.has_line_index());
        locations
            .set_line(SourceOffset(3), 50, Some(b"first".to_vec()))
            .unwrap();
        assert_eq!(locations.starts(), [0, 2, 4, 6, 8]);
        assert_eq!(locations.presumed_line(SourceOffset(0)).unwrap(), 48);
        assert_eq!(
            locations
                .describe(&SourceError::new(SourceOffset(0), "early"))
                .unwrap(),
            b"physical:1:1: early"
        );
        locations
            .set_line(SourceOffset(5), 2, Some(b"second".to_vec()))
            .unwrap();
        assert_eq!(
            locations
                .describe(&SourceError::new(SourceOffset(4), "middle"))
                .unwrap(),
            b"first:50:1: middle"
        );
        assert_eq!(
            locations
                .describe(&SourceError::new(SourceOffset(5), "boundary"))
                .unwrap(),
            b"second:1:2: boundary"
        );
        assert_eq!(
            locations
                .describe(&SourceError::new(SourceOffset(6), "latest"))
                .unwrap(),
            b"second:2:1: latest"
        );
        assert_eq!(locations.presumed_line(SourceOffset(0)).unwrap(), 1);
        let literal = locations.file_literal();
        locations.set_line(SourceOffset(7), 8, None).unwrap();
        assert!(Rc::ptr_eq(&literal, &locations.file_literal()));
        assert_eq!(locations.earlier.len(), 3);
    }

    #[test]
    fn bom_columns_and_filename_bytes_are_preserved_without_utf8_conversion() {
        let source = SourceBuffer::new(b"\xef\xbb\xbfa\n".to_vec()).unwrap();
        let mut locations = Locations::new(&source, b"a\"\\\n\t\x7f\xff");
        assert_eq!(
            &*locations.file_literal(),
            b"\"a\\\"\\\\\\012\\011\\177\xff\""
        );
        assert_eq!(locations.presumed_line(SourceOffset(3)).unwrap(), 1);
        assert_eq!(
            locations
                .describe(&SourceError::new(SourceOffset(0), "bom"))
                .unwrap(),
            b"a\"\\\n\t\x7f\xff:1:1: bom"
        );
    }
}

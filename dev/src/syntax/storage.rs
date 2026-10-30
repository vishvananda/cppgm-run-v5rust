//! Translation-unit slabs and reusable nested list scratch. See NOTICE.
use super::ParseError;
use std::marker::PhantomData;

#[derive(Debug)]
pub struct Id<T>(u32, PhantomData<fn() -> T>);
impl<T> Id<T> {
    #[inline]
    #[must_use]
    pub fn index(self) -> usize {
        self.0 as usize
    }
}
impl<T> Copy for Id<T> {}
impl<T> PartialEq for Id<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}
impl<T> Eq for Id<T> {}
impl<T> std::hash::Hash for Id<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::hash::Hash::hash(&self.0, state);
    }
}
impl<T> Clone for Id<T> {
    fn clone(&self) -> Self {
        *self
    }
}
#[derive(Debug)]
pub struct List<T> {
    begin: u32,
    end: u32,
    marker: PhantomData<fn() -> T>,
}
impl<T> PartialEq for List<T> {
    fn eq(&self, other: &Self) -> bool {
        self.begin == other.begin && self.end == other.end
    }
}
impl<T> Eq for List<T> {}
impl<T> std::hash::Hash for List<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::hash::Hash::hash(&(self.begin, self.end), state);
    }
}
impl<T> Copy for List<T> {}
impl<T> Clone for List<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> Default for List<T> {
    fn default() -> Self {
        Self {
            begin: 0,
            end: 0,
            marker: PhantomData,
        }
    }
}
impl<T> List<T> {
    /// Borrow a checked range of an existing immutable slab list.
    pub fn range(self, range: std::ops::Range<usize>) -> Result<Self, ParseError> {
        let count = usize::try_from(self.end - self.begin)
            .map_err(|_| ParseError::Limit("syntax list range"))?;
        if range.start > range.end || range.end > count {
            return Err(ParseError::Invariant("syntax list range"));
        }
        let begin = self
            .begin
            .checked_add(
                u32::try_from(range.start).map_err(|_| ParseError::Limit("syntax list range"))?,
            )
            .ok_or(ParseError::Limit("syntax list range"))?;
        let end = self
            .begin
            .checked_add(
                u32::try_from(range.end).map_err(|_| ParseError::Limit("syntax list range"))?,
            )
            .ok_or(ParseError::Limit("syntax list range"))?;
        Ok(Self {
            begin,
            end,
            marker: PhantomData,
        })
    }
    #[inline]
    #[must_use]
    pub fn is_empty(self) -> bool {
        self.begin == self.end
    }
}

pub struct Store<T> {
    data: Vec<T>,
}
impl<T> Default for Store<T> {
    fn default() -> Self {
        Self { data: Vec::new() }
    }
}
impl<T> Store<T> {
    #[cfg(feature = "telemetry")]
    #[inline]
    #[must_use]
    pub fn reserved_bytes(&self) -> usize {
        self.data
            .capacity()
            .saturating_mul(std::mem::size_of::<T>())
    }

    #[inline]
    pub fn add(&mut self, value: T) -> Result<Id<T>, ParseError> {
        let index =
            u32::try_from(self.data.len()).map_err(|_| ParseError::Limit("syntax store limit"))?;
        self.data.push(value);
        Ok(Id(index, PhantomData))
    }
    #[inline]
    #[must_use]
    pub fn get(&self, id: Id<T>) -> &T {
        &self.data[id.0 as usize]
    }
    #[inline]
    pub fn replace(&mut self, id: Id<T>, value: T) {
        self.data[id.0 as usize] = value;
    }
    #[inline]
    #[must_use]
    pub fn mark(&self) -> usize {
        self.data.len()
    }
    #[inline]
    pub fn rewind(&mut self, mark: usize) {
        self.data.truncate(mark);
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ListMark(usize, usize);
pub struct Lists<T> {
    data: Vec<T>,
    scratch: Vec<T>,
}
impl<T> Default for Lists<T> {
    fn default() -> Self {
        Self {
            data: Vec::new(),
            scratch: Vec::new(),
        }
    }
}
impl<T: Copy> Lists<T> {
    #[cfg(feature = "telemetry")]
    #[inline]
    #[must_use]
    pub fn reserved_bytes(&self) -> usize {
        self.data
            .capacity()
            .saturating_add(self.scratch.capacity())
            .saturating_mul(std::mem::size_of::<T>())
    }

    #[inline]
    #[must_use]
    pub fn begin(&self) -> usize {
        self.scratch.len()
    }
    #[inline]
    pub fn push(&mut self, value: T) {
        self.scratch.push(value);
    }
    pub fn finish(&mut self, start: usize) -> Result<List<T>, ParseError> {
        let begin =
            u32::try_from(self.data.len()).map_err(|_| ParseError::Limit("syntax list limit"))?;
        let count = u32::try_from(self.scratch.len() - start)
            .map_err(|_| ParseError::Limit("syntax list limit"))?;
        let end = begin
            .checked_add(count)
            .ok_or(ParseError::Limit("syntax list limit"))?;
        self.data.extend_from_slice(&self.scratch[start..]);
        self.scratch.truncate(start);
        Ok(List {
            begin,
            end,
            marker: PhantomData,
        })
    }
    #[inline]
    pub fn extend(&mut self, list: List<T>) {
        self.scratch
            .extend_from_slice(&self.data[list.begin as usize..list.end as usize]);
    }
    #[inline]
    #[must_use]
    pub fn get(&self, list: List<T>) -> &[T] {
        &self.data[list.begin as usize..list.end as usize]
    }
    #[inline]
    pub fn replace(&mut self, list: List<T>, index: usize, value: T) {
        self.data[list.begin as usize..list.end as usize][index] = value;
    }
    #[inline]
    #[must_use]
    pub fn mark(&self) -> ListMark {
        ListMark(self.data.len(), self.scratch.len())
    }
    #[inline]
    pub fn rewind(&mut self, mark: ListMark) {
        self.data.truncate(mark.0);
        self.scratch.truncate(mark.1);
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Text {
    begin: u32,
    end: u32,
    block: Option<std::num::NonZeroU32>,
}
#[derive(Default)]
pub struct TextStore {
    pub(super) literals: Store<super::ast::LiteralData>,
    bytes: Vec<u8>,
    strings: Vec<Box<[u8]>>,
    used: usize,
    string_bytes: usize,
}
impl TextStore {
    #[inline]
    pub fn literal(
        &mut self,
        data: super::ast::LiteralData,
    ) -> Result<super::ast::Literal, ParseError> {
        self.literals.add(data)
    }

    /// Source string sequences outlive the tokenizer scratch. The observable
    /// spelling arena retains the frozen 16-KiB/8-MiB block growth policy.
    /// A boxed-slice handle is the 16-byte block record; scalar/data copies
    /// live in the separate typed syntax backing store.
    pub fn copy_string(&mut self, bytes: &[u8]) -> Result<Text, ParseError> {
        const HEADER: usize = std::mem::size_of::<Box<[u8]>>();
        let count = bytes
            .len()
            .checked_add(1)
            .ok_or(ParseError::Limit("syntax spelling limit"))?;
        if self
            .strings
            .last()
            .is_none_or(|block| count > block.len() - self.used)
        {
            let growth = if self.string_bytes == 0 {
                16 * 1024
            } else {
                self.string_bytes.min(8 * 1024 * 1024)
            };
            let needed = HEADER
                .checked_add(count)
                .and_then(|n| n.checked_add(1))
                .ok_or(ParseError::Limit("syntax spelling limit"))?;
            let size = growth.max(needed);
            self.string_bytes = self
                .string_bytes
                .checked_add(size)
                .ok_or(ParseError::Limit("syntax spelling limit"))?;
            self.strings.push(vec![0; size - HEADER].into_boxed_slice());
            self.used = 0;
        }
        let block = std::num::NonZeroU32::new(
            u32::try_from(self.strings.len())
                .map_err(|_| ParseError::Limit("syntax spelling limit"))?,
        )
        .ok_or(ParseError::Invariant("missing spelling block"))?;
        let begin =
            u32::try_from(self.used).map_err(|_| ParseError::Limit("syntax spelling limit"))?;
        let end = self
            .used
            .checked_add(bytes.len())
            .ok_or(ParseError::Limit("syntax spelling limit"))?;
        self.strings[(block.get() - 1) as usize][self.used..end].copy_from_slice(bytes);
        self.used += count;
        Ok(Text {
            begin,
            end: u32::try_from(end).map_err(|_| ParseError::Limit("syntax spelling limit"))?,
            block: Some(block),
        })
    }
    #[cfg(feature = "telemetry")]
    #[inline]
    #[must_use]
    pub fn spelling_bytes(&self) -> usize {
        self.string_bytes
    }
    #[cfg(feature = "telemetry")]
    #[must_use]
    pub fn reserved_bytes(&self) -> usize {
        self.literals
            .reserved_bytes()
            .saturating_add(self.bytes.capacity())
            .saturating_add(
                self.strings
                    .capacity()
                    .saturating_mul(std::mem::size_of::<Box<[u8]>>()),
            )
    }

    pub fn add(&mut self, bytes: &[u8]) -> Result<Text, ParseError> {
        let begin = u32::try_from(self.bytes.len())
            .map_err(|_| ParseError::Limit("syntax spelling limit"))?;
        let count =
            u32::try_from(bytes.len()).map_err(|_| ParseError::Limit("syntax spelling limit"))?;
        let end = begin
            .checked_add(count)
            .ok_or(ParseError::Limit("syntax spelling limit"))?;
        self.bytes.extend_from_slice(bytes);
        Ok(Text {
            begin,
            end,
            block: None,
        })
    }
    #[must_use]
    pub fn unquote(text: Text) -> Text {
        Text {
            block: text.block,
            begin: text.begin.saturating_add(1).min(text.end),
            end: text
                .end
                .saturating_sub(1)
                .max(text.begin.saturating_add(1).min(text.end)),
        }
    }
    #[inline]
    #[must_use]
    pub fn get(&self, text: Text) -> &[u8] {
        let bytes = text.block.map_or(self.bytes.as_slice(), |block| {
            self.strings[(block.get() - 1) as usize].as_ref()
        });
        &bytes[text.begin as usize..text.end as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_spelling_blocks_preserve_views_across_growth() -> Result<(), ParseError> {
        let mut store = TextStore::default();
        let scalar = store.add(b"123")?;
        assert_eq!(store.spelling_bytes(), 0);
        let first = store.copy_string(b"\"first\"")?;
        let first_reservation = store.spelling_bytes();
        let larger = vec![b'x'; first_reservation];
        let second = store.copy_string(&larger)?;
        assert!(store.spelling_bytes() > first_reservation);
        assert_eq!(store.get(scalar), b"123");
        assert_eq!(store.get(first), b"\"first\"");
        assert_eq!(store.get(TextStore::unquote(first)), b"first");
        assert_eq!(store.get(second), larger);
        let owned: usize = store
            .strings
            .iter()
            .map(|block| block.len() + std::mem::size_of::<Box<[u8]>>())
            .sum();
        assert_eq!(store.spelling_bytes(), owned);
        Ok(())
    }
}

//! Translation-unit slabs and reusable nested list scratch. See NOTICE.
use super::ParseError;
use std::marker::PhantomData;

#[derive(Debug)]
pub(super) struct Id<T>(u32, PhantomData<fn() -> T>);
impl<T> Copy for Id<T> {}
impl<T> Clone for Id<T> {
    fn clone(&self) -> Self {
        *self
    }
}
#[derive(Debug)]
pub(super) struct List<T> {
    begin: u32,
    end: u32,
    marker: PhantomData<fn() -> T>,
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
    pub(super) fn is_empty(self) -> bool {
        self.begin == self.end
    }
}

pub(super) struct Store<T> {
    data: Vec<T>,
}
impl<T> Default for Store<T> {
    fn default() -> Self {
        Self { data: Vec::new() }
    }
}
impl<T> Store<T> {
    pub(super) fn add(&mut self, value: T) -> Result<Id<T>, ParseError> {
        let index =
            u32::try_from(self.data.len()).map_err(|_| ParseError::Limit("syntax store limit"))?;
        self.data.push(value);
        Ok(Id(index, PhantomData))
    }
    pub(super) fn get(&self, id: Id<T>) -> &T {
        &self.data[id.0 as usize]
    }
    pub(super) fn replace(&mut self, id: Id<T>, value: T) {
        self.data[id.0 as usize] = value;
    }
    pub(super) fn mark(&self) -> usize {
        self.data.len()
    }
    pub(super) fn rewind(&mut self, mark: usize) {
        self.data.truncate(mark);
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct ListMark(usize, usize);
pub(super) struct Lists<T> {
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
    pub(super) fn begin(&self) -> usize {
        self.scratch.len()
    }
    pub(super) fn push(&mut self, value: T) {
        self.scratch.push(value);
    }
    pub(super) fn finish(&mut self, start: usize) -> Result<List<T>, ParseError> {
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
    pub(super) fn get(&self, list: List<T>) -> &[T] {
        &self.data[list.begin as usize..list.end as usize]
    }
    pub(super) fn mark(&self) -> ListMark {
        ListMark(self.data.len(), self.scratch.len())
    }
    pub(super) fn rewind(&mut self, mark: ListMark) {
        self.data.truncate(mark.0);
        self.scratch.truncate(mark.1);
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct Text {
    begin: u32,
    end: u32,
}
#[derive(Default)]
pub(super) struct TextStore {
    bytes: Vec<u8>,
}
impl TextStore {
    pub(super) fn add(&mut self, bytes: &[u8]) -> Result<Text, ParseError> {
        let begin = u32::try_from(self.bytes.len())
            .map_err(|_| ParseError::Limit("syntax spelling limit"))?;
        let count =
            u32::try_from(bytes.len()).map_err(|_| ParseError::Limit("syntax spelling limit"))?;
        let end = begin
            .checked_add(count)
            .ok_or(ParseError::Limit("syntax spelling limit"))?;
        self.bytes.extend_from_slice(bytes);
        Ok(Text { begin, end })
    }
    pub(super) fn unquote(text: Text) -> Text {
        Text {
            begin: text.begin.saturating_add(1).min(text.end),
            end: text
                .end
                .saturating_sub(1)
                .max(text.begin.saturating_add(1).min(text.end)),
        }
    }
    pub(super) fn get(&self, text: Text) -> &[u8] {
        &self.bytes[text.begin as usize..text.end as usize]
    }
}

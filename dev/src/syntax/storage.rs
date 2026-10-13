//! Translation-unit slabs and reusable nested list scratch. See NOTICE.
use super::ParseError;
use std::marker::PhantomData;

#[derive(Debug)]
pub(crate) struct Id<T>(u32, PhantomData<fn() -> T>);
impl<T> Id<T> {
    pub(crate) fn index(self) -> usize {
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
pub(crate) struct List<T> {
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
    pub(crate) fn is_empty(self) -> bool {
        self.begin == self.end
    }
}

pub(crate) struct Store<T> {
    data: Vec<T>,
}
impl<T> Default for Store<T> {
    fn default() -> Self {
        Self { data: Vec::new() }
    }
}
impl<T> Store<T> {
    pub(crate) fn add(&mut self, value: T) -> Result<Id<T>, ParseError> {
        let index =
            u32::try_from(self.data.len()).map_err(|_| ParseError::Limit("syntax store limit"))?;
        self.data.push(value);
        Ok(Id(index, PhantomData))
    }
    pub(crate) fn get(&self, id: Id<T>) -> &T {
        &self.data[id.0 as usize]
    }
    pub(crate) fn replace(&mut self, id: Id<T>, value: T) {
        self.data[id.0 as usize] = value;
    }
    pub(crate) fn mark(&self) -> usize {
        self.data.len()
    }
    pub(crate) fn rewind(&mut self, mark: usize) {
        self.data.truncate(mark);
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ListMark(usize, usize);
pub(crate) struct Lists<T> {
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
    pub(crate) fn begin(&self) -> usize {
        self.scratch.len()
    }
    pub(crate) fn push(&mut self, value: T) {
        self.scratch.push(value);
    }
    pub(crate) fn finish(&mut self, start: usize) -> Result<List<T>, ParseError> {
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
    pub(crate) fn extend(&mut self, list: List<T>) {
        self.scratch
            .extend_from_slice(&self.data[list.begin as usize..list.end as usize]);
    }
    pub(crate) fn get(&self, list: List<T>) -> &[T] {
        &self.data[list.begin as usize..list.end as usize]
    }
    pub(crate) fn mark(&self) -> ListMark {
        ListMark(self.data.len(), self.scratch.len())
    }
    pub(crate) fn rewind(&mut self, mark: ListMark) {
        self.data.truncate(mark.0);
        self.scratch.truncate(mark.1);
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Text {
    begin: u32,
    end: u32,
}
#[derive(Default)]
pub(crate) struct TextStore {
    bytes: Vec<u8>,
}
impl TextStore {
    pub(crate) fn add(&mut self, bytes: &[u8]) -> Result<Text, ParseError> {
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
    pub(crate) fn unquote(text: Text) -> Text {
        Text {
            begin: text.begin.saturating_add(1).min(text.end),
            end: text
                .end
                .saturating_sub(1)
                .max(text.begin.saturating_add(1).min(text.end)),
        }
    }
    pub(crate) fn get(&self, text: Text) -> &[u8] {
        &self.bytes[text.begin as usize..text.end as usize]
    }
}

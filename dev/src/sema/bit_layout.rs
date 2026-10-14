// Translated from sema/layout.cpp::assign_bit_units/segment/fields. See NOTICE.
//! Reusable class-local runs assign exact bounded storage windows, including split units.
use super::{
    Error,
    bit_fields::{BitFieldId, BitFields, Storage},
    layout::round_up,
};
use crate::support::source::SourceOffset;

struct Field {
    id: BitFieldId,
    bit: u64,
    width: u64,
    natural: u64,
    packed: bool,
    storage: Storage,
}
struct Run {
    first: usize,
    last: usize,
    lo: u64,
    hi: Option<u64>,
}
#[derive(Clone, Copy)]
struct Segment {
    first: usize,
    last: usize,
    begin: u64,
    bytes: u64,
}
#[derive(Default)]
pub(super) struct BitLayout {
    fields: Vec<Field>,
    runs: Vec<Run>,
    segments: Vec<Segment>,
}
#[derive(Clone, Copy)]
pub(super) struct Placement {
    pub data_bits: u64,
    pub natural: u64,
    pub requested: u64,
    pub packed: bool,
    pub pack: u64,
    pub union: bool,
    pub named: bool,
    pub location: SourceOffset,
}
pub(super) struct Placed {
    pub data_bits: u64,
    pub size: u64,
    pub alignment: u64,
}
// The narrowest integer window holding [first,last) within [lo,hi).
fn narrow_window(first: u64, last: u64, lo: u64, hi: u64) -> Option<(u64, u64)> {
    for width in [1, 2, 4, 8] {
        if width < last.wrapping_sub(first) || width > hi.wrapping_sub(lo) {
            continue;
        }
        let aligned = first / width * width;
        let begin = if aligned >= lo && aligned + width <= hi && aligned + width >= last {
            aligned
        } else if first + width <= hi {
            first
        } else if hi - width >= lo && hi - width <= first {
            hi - width
        } else {
            continue;
        };
        return Some((begin, width));
    }
    None
}
impl BitLayout {
    pub(super) fn clear(&mut self) {
        self.fields.clear();
        self.runs.clear();
        self.segments.clear();
    }
    pub(super) fn end_run(&mut self, limit: u64) {
        if let Some(run) = self.runs.last_mut()
            && run.hi.is_none()
        {
            run.hi = Some(limit);
        }
    }
    pub(super) fn place_member(
        &mut self,
        store: &mut BitFields,
        id: BitFieldId,
        place: Placement,
    ) -> Result<Placed, Error> {
        let width = store.width(id);
        let wide = width > place.natural * 8;
        let mut alignment = if wide {
            let mut alignment = 8;
            while alignment * 8 > width {
                alignment /= 2;
            }
            alignment
        } else {
            place.natural
        };
        alignment = if place.packed {
            place.requested.max(1)
        } else {
            alignment.max(place.requested)
        };
        if place.pack != 0 {
            alignment = alignment.min(place.pack);
        }
        let mut class_alignment = if place.pack != 0 {
            place.natural.min(place.pack)
        } else if place.packed {
            1
        } else {
            place.natural
        };
        if wide {
            class_alignment = class_alignment.max(alignment);
        } else if !place.named {
            class_alignment = 1;
        }
        let (data_bits, size) = if place.union {
            store.publish(
                id,
                Storage {
                    bit: 0,
                    offset: 0,
                    bytes: u8::try_from(place.natural)
                        .map_err(|_| Error::Invariant("bit-field type too wide"))?,
                    split: 0,
                    shared: false,
                },
            );
            (place.data_bits, round_up(width, 8) / 8)
        } else {
            let end = self.place(
                store,
                id,
                place.data_bits,
                place.natural,
                alignment,
                place.packed || place.pack != 0,
            )?;
            (end, round_up(end, 8) / 8)
        };
        if size >> 60 != 0 {
            return Err(Error::at(
                place.location,
                "a class too large (its size reaches 2^60 bytes)",
            ));
        }
        Ok(Placed {
            data_bits,
            size,
            alignment: class_alignment,
        })
    }
    fn place(
        &mut self,
        store: &mut BitFields,
        id: BitFieldId,
        data_bits: u64,
        natural: u64,
        alignment: u64,
        packed: bool,
    ) -> Result<u64, Error> {
        let width = store.width(id);
        let unit = natural * 8;
        let wide = width > unit;
        let mut start = data_bits;
        if width == 0 {
            start = round_up(start, unit);
        } else if wide {
            start = round_up(start, alignment * 8);
        } else if !packed && start / unit != start.wrapping_add(width - 1) / unit {
            start = round_up(start, unit);
        }
        let end = start.wrapping_add(width);
        if width == 0 {
            self.end_run(start / 8);
            store.publish(
                id,
                Storage {
                    bit: start,
                    offset: 0,
                    bytes: 0,
                    split: 0,
                    shared: false,
                },
            );
        } else {
            if self.runs.last().is_none_or(|run| run.hi.is_some()) {
                self.runs.push(Run {
                    first: self.fields.len(),
                    last: self.fields.len(),
                    lo: round_up(data_bits, 8) / 8,
                    hi: None,
                });
            }
            self.fields.push(Field {
                id,
                bit: start,
                width: width.min(unit),
                natural,
                packed,
                storage: Storage {
                    bit: start,
                    offset: 0,
                    bytes: 0,
                    split: 0,
                    shared: false,
                },
            });
            self.runs
                .last_mut()
                .ok_or(Error::Invariant("bit-field without run"))?
                .last = self.fields.len();
        }
        Ok(end)
    }
    pub(super) fn finish(
        &mut self,
        store: &mut BitFields,
        limit: u64,
        location: SourceOffset,
    ) -> Result<(), Error> {
        self.end_run(limit);
        for r in 0..self.runs.len() {
            let run = &self.runs[r];
            let (first, last, lo, hi) = (
                run.first,
                run.last,
                run.lo,
                run.hi.ok_or(Error::Invariant("open bit-field run"))?,
            );
            if first == last {
                continue;
            }
            if self.fields[first].packed {
                self.assign_fields(first, last, lo, hi, location)?;
                continue;
            }
            self.segments.clear();
            for i in first..last {
                let bytes = self.fields[i].natural;
                let begin = self.fields[i].bit / 8 / bytes * bytes;
                let mut from = i;
                while self
                    .segments
                    .last()
                    .is_some_and(|s| begin <= s.begin && s.begin + s.bytes <= begin + bytes)
                {
                    from = self
                        .segments
                        .pop()
                        .ok_or(Error::Invariant("missing bit-field segment"))?
                        .first;
                }
                if let Some(s) = self.segments.last_mut()
                    && s.begin <= begin
                    && begin + bytes <= s.begin + s.bytes
                {
                    s.last = i + 1;
                } else {
                    self.segments.push(Segment {
                        first: from,
                        last: i + 1,
                        begin,
                        bytes,
                    });
                }
            }
            for i in 0..self.segments.len() {
                self.assign_segment(self.segments[i], lo..hi, location)?;
            }
        }
        for field in &self.fields {
            store.publish(field.id, field.storage);
        }
        Ok(())
    }
    fn assign_segment(
        &mut self,
        segment: Segment,
        range: std::ops::Range<u64>,
        location: SourceOffset,
    ) -> Result<(), Error> {
        let Segment {
            first,
            last,
            begin,
            bytes,
        } = segment;
        let (lo, hi) = (range.start, range.end);
        let mut window = (begin, bytes);
        if begin < lo || begin + bytes > hi {
            let lo = lo.max(begin);
            let hi = hi.min(begin + bytes);
            let mut data_first = u64::MAX;
            let mut data_last = 0;
            for field in &self.fields[first..last] {
                data_first = data_first.min(field.bit / 8);
                data_last = data_last.max(round_up(field.bit.wrapping_add(field.width), 8) / 8);
            }
            let Some(narrow) = narrow_window(data_first, data_last, lo, hi) else {
                return self.assign_fields(first, last, lo, hi, location);
            };
            window = narrow;
        }
        for field in &mut self.fields[first..last] {
            field.storage.offset = window.0;
            field.storage.bytes = u8::try_from(window.1)
                .map_err(|_| Error::Invariant("bit-field storage exceeds target width"))?;
        }
        Ok(())
    }
    fn assign_fields(
        &mut self,
        first: usize,
        last: usize,
        lo: u64,
        hi: u64,
        location: SourceOffset,
    ) -> Result<(), Error> {
        for field in &mut self.fields[first..last] {
            let begin = field.bit / 8;
            let end = round_up(field.bit.wrapping_add(field.width), 8) / 8;
            let (offset, bytes) = if let Some(window) = narrow_window(begin, end, lo, hi) {
                window
            } else if field.packed {
                if end - begin > 16 {
                    return Err(Error::at(
                        location,
                        "a packed bit-field spanning more than 16 bytes",
                    ));
                }
                field.storage.split = u8::try_from(end - begin)
                    .map_err(|_| Error::Invariant("bit-field split exceeds target width"))?;
                field.storage.shared = true;
                (begin, if end - begin <= 8 { 8 } else { 16 })
            } else {
                field.storage.shared = true;
                (begin / field.natural * field.natural, field.natural)
            };
            field.storage.offset = offset;
            field.storage.bytes = u8::try_from(bytes)
                .map_err(|_| Error::Invariant("bit-field storage exceeds target width"))?;
        }
        for i in first..last.saturating_sub(1) {
            let left = self.fields[i].storage;
            let right = self.fields[i + 1].storage;
            if (left.offset != right.offset || left.bytes != right.bytes)
                && right.offset < left.offset + u64::from(left.bytes)
            {
                self.fields[i].storage.shared = true;
                self.fields[i + 1].storage.shared = true;
            }
        }
        Ok(())
    }
}

//! Exact LSDA call-site/action/type serialization (`object/host_unwind.cpp`). See NOTICE.
use super::{
    host_unwind::{Call, Frame, uleb},
    region_flow::Tables,
};
use crate::lowir::{Error, model::SymbolId};
use std::collections::HashMap;

#[derive(Default)]
pub(super) struct Lsda {
    pub(super) bytes: Vec<u8>,
    pub(super) types: Vec<(u32, SymbolId)>,
}
fn uleb_size(mut value: u64) -> usize {
    let mut n = 1;
    while value >= 128 {
        n += 1;
        value >>= 7;
    }
    n
}
fn sleb_size(mut value: i64) -> usize {
    let mut n = 1;
    while !(-64..=63).contains(&value) {
        n += 1;
        value >>= 7;
    }
    n
}
fn sleb(out: &mut Vec<u8>, mut value: i64, padding: usize) {
    let mut n = 0;
    loop {
        let byte = (value & 127).to_le_bytes()[0];
        value >>= 7;
        n += 1;
        let done = value == 0 && byte & 64 == 0 || value == -1 && byte & 64 != 0;
        if done && n >= padding {
            out.push(byte);
            break;
        }
        out.push(byte | 128);
    }
}
fn padded_uleb(out: &mut Vec<u8>, mut value: u64, padding: usize) {
    let mut n = 0;
    loop {
        let byte = (value & 127).to_le_bytes()[0];
        value >>= 7;
        n += 1;
        out.push(byte | if value != 0 || n < padding { 128 } else { 0 });
        if value == 0 && n >= padding {
            break;
        }
    }
}
#[derive(Clone, Copy)]
struct Record {
    filter: i32,
    next: Option<usize>,
}
fn displacement(records: &[Record], positions: &[usize], r: usize) -> Result<i64, Error> {
    let Some(next) = records[r].next else {
        return Ok(0);
    };
    Ok(i64::try_from(positions[next]).map_err(|_| Error::Limit)?
        - i64::try_from(positions[r] + sleb_size(i64::from(records[r].filter)))
            .map_err(|_| Error::Limit)?)
}
fn site(
    out: &mut Vec<u8>,
    call: Call,
    end: u32,
    pads: &[u32],
    first: &[Option<usize>],
    positions: &[usize],
) -> Result<(), Error> {
    uleb(out, call.begin.into());
    uleb(
        out,
        u64::from(end.checked_sub(call.begin).ok_or(Error::Limit)?),
    );
    uleb(out, call.landing.map_or(0, |l| u64::from(pads[l.index()])));
    let action = call
        .landing
        .and_then(|l| first[l.index()])
        .map_or(Ok(0), |r| {
            u64::try_from(positions[r] + 1).map_err(|_| Error::Limit)
        })?;
    uleb(out, action);
    Ok(())
}
pub(super) fn encode(frame: &Frame, tables: &Tables) -> Result<Lsda, Error> {
    let mut out = Lsda::default();
    let mut used = vec![false; tables.landings.len()];
    for call in &frame.calls {
        if let Some(index) = call.landing {
            used[index.index()] = true;
        }
    }
    if !used.iter().any(|&used| used) {
        return Ok(out);
    }
    let mut records = Vec::<Record>::new();
    let mut index = HashMap::new();
    let mut first = vec![None; tables.landings.len()];
    let mut typed = false;
    for (l, landing) in tables.landings.iter().enumerate() {
        let filters = &tables.filters[landing.filters.clone()];
        if !used[l] || filters.is_empty() || filters == [0] {
            continue;
        }
        let mut next = None;
        for &filter in filters.iter().rev() {
            typed |= filter != 0;
            let key = (filter, next);
            if let Some(&known) = index.get(&key) {
                next = Some(known);
            } else {
                let id = records.len();
                records.push(Record { filter, next });
                index.insert(key, id);
                next = Some(id);
            }
        }
        first[l] = next;
    }
    let mut positions = vec![0_usize; records.len()];
    let mut widths: Vec<_> = records
        .iter()
        .map(|r| sleb_size(i64::from(r.filter)) + 1)
        .collect();
    // Displacement widths can only grow, at most ten bytes per record.
    loop {
        let mut at = 0_usize;
        for (r, &size) in widths.iter().enumerate() {
            positions[r] = at;
            at = at.checked_add(size).ok_or(Error::Limit)?;
        }
        let mut changed = false;
        for r in 0..records.len() {
            let needed = sleb_size(i64::from(records[r].filter))
                + sleb_size(displacement(&records, &positions, r)?);
            if needed > widths[r] {
                widths[r] = needed;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    let mut sites = Vec::new();
    let mut open: Option<Call> = None;
    let mut end = 0;
    for &call in &frame.calls {
        if open.is_some_and(|previous| previous.landing == call.landing) {
            end = call.end;
            continue;
        }
        if let Some(previous) = open {
            site(&mut sites, previous, end, &frame.pads, &first, &positions)?;
        }
        open = Some(call);
        end = call.end;
    }
    if let Some(previous) = open {
        site(&mut sites, previous, end, &frame.pads, &first, &positions)?;
    }
    let mut rest = vec![1];
    uleb(
        &mut rest,
        u64::try_from(sites.len()).map_err(|_| Error::Limit)?,
    );
    rest.extend(sites);
    for (r, record) in records.iter().enumerate() {
        sleb(&mut rest, i64::from(record.filter), 0);
        sleb(
            &mut rest,
            displacement(&records, &positions, r)?,
            widths[r] - sleb_size(i64::from(record.filter)),
        );
    }
    let entries = if typed { tables.types.len() } else { 0 };
    out.bytes.push(255);
    if entries == 0 && (!typed || tables.specs.is_empty()) {
        out.bytes.push(255);
        out.bytes.extend(rest);
        return Ok(out);
    }
    let mut base_bytes = 1_usize;
    let (base, padding) = loop {
        let before = 2 + base_bytes + rest.len();
        let padding = (4 - before % 4) % 4;
        let base = rest
            .len()
            .checked_add(padding)
            .and_then(|n| {
                entries
                    .checked_mul(4)
                    .and_then(|bytes| n.checked_add(bytes))
            })
            .ok_or(Error::Limit)?;
        let needed = uleb_size(u64::try_from(base).map_err(|_| Error::Limit)?);
        if needed <= base_bytes {
            break (base, padding);
        }
        base_bytes = needed;
    };
    out.bytes.push(0x9b);
    padded_uleb(
        &mut out.bytes,
        u64::try_from(base).map_err(|_| Error::Limit)?,
        base_bytes,
    );
    out.bytes.extend(rest);
    out.bytes.resize(out.bytes.len() + padding, 0);
    let table = out.bytes.len();
    out.bytes.resize(table + 4 * entries, 0);
    out.bytes.extend_from_slice(&tables.specs);
    for (k, &ty) in tables.types[..entries].iter().enumerate() {
        if let Some(ty) = ty {
            out.types.push((
                u32::try_from(table + 4 * (entries - 1 - k)).map_err(|_| Error::Limit)?,
                ty,
            ));
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::region_flow::{Landing, LandingId};

    #[test]
    fn shared_action_suffixes_keep_unprotected_calls_as_coalescing_barriers() -> Result<(), Error> {
        let a = LandingId::new(0)?;
        let b = LandingId::new(1)?;
        let tables = Tables {
            landings: vec![
                Landing {
                    filters: 0..2,
                    ..Landing::default()
                },
                Landing {
                    filters: 2..4,
                    ..Landing::default()
                },
            ],
            filters: vec![1, 0, 2, 0],
            types: vec![None, None],
            ..Tables::default()
        };
        let frame = Frame {
            pads: vec![100, 200],
            calls: [
                (10, 15, Some(a)),
                (20, 25, Some(a)),
                (30, 35, None),
                (40, 45, Some(b)),
                (50, 55, Some(a)),
            ]
            .into_iter()
            .map(|(begin, end, landing)| Call {
                begin,
                end,
                landing,
                args: 0,
            })
            .collect(),
            ..Frame::default()
        };
        let encoded = encode(&frame, &tables)?;
        // One shared cleanup action, two distinct typed heads; table entries
        // retain the unprotected interval and the return to the first landing.
        assert_eq!(
            encoded.bytes,
            [
                0xff, 0x9b, 33, 1, 17, 10, 15, 100, 3, 30, 5, 0, 0, 40, 5, 0xc8, 1, 5, 50, 5, 100,
                3, 0, 0, 1, 0x7d, 2, 0x7b, 0, 0, 0, 0, 0, 0, 0, 0,
            ]
        );
        assert!(encoded.types.is_empty());
        let no_calls = encode(&Frame::default(), &tables)?;
        assert!(no_calls.bytes.is_empty());
        Ok(())
    }

    fn decode_signed(bytes: &[u8]) -> i64 {
        let mut bits = 0_i64;
        let mut shift = 0;
        for &byte in bytes {
            bits |= i64::from(byte & 127) << shift;
            shift += 7;
            if byte & 128 == 0 {
                return if byte & 64 != 0 {
                    bits | (-1_i64 << shift)
                } else {
                    bits
                };
            }
        }
        0
    }

    #[test]
    fn signed_displacements_keep_padding_when_a_record_width_grows() {
        for value in [-8193, -8192, -65, -64, -1, 0, 63, 64, 8191, 8192] {
            let mut minimal = Vec::new();
            sleb(&mut minimal, value, 0);
            assert_eq!(minimal.len(), sleb_size(value));
            let mut padded = Vec::new();
            sleb(&mut padded, value, minimal.len() + 2);
            assert_eq!(padded.len(), minimal.len() + 2);

            assert_eq!(decode_signed(&minimal), value);
            assert_eq!(decode_signed(&padded), value);
        }
    }
}

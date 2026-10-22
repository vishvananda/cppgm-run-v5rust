//! CFI programs for kept, omitted and repeated frame setups. See NOTICE.
use super::host_unwind::{Frame, Prologue, advance, uleb};
use crate::lowir::Error;
const DWARF: [u8; 16] = [0, 2, 1, 3, 7, 6, 4, 5, 8, 9, 10, 11, 12, 13, 14, 15];
struct Cursor {
    bytes: Vec<u8>,
    at: u32,
}
impl Cursor {
    fn new() -> Self {
        Self {
            bytes: Vec::new(),
            at: 0,
        }
    }
    fn advance(&mut self, to: u32) -> Result<(), Error> {
        advance(&mut self.bytes, &mut self.at, to)
    }
    fn offset(&mut self, offset: u32) {
        self.bytes.push(0x0e);
        uleb(&mut self.bytes, offset.into());
    }
    fn args(&mut self, args: u32) {
        self.bytes.push(0x2e);
        uleb(&mut self.bytes, args.into());
    }
}
fn kept_epilogue(
    c: &mut Cursor,
    frame: &Frame,
    leave: u32,
    ret: u32,
    more: bool,
) -> Result<(), Error> {
    c.advance(leave)?;
    if more {
        c.bytes.push(0x0a);
    }
    c.bytes.extend_from_slice(&[0x0c, 7, 8, 0xc6]);
    for &(_, reg, _) in &frame.saves {
        c.bytes.push(0xc0 | DWARF[usize::from(reg.code())]);
    }
    if more {
        c.advance(ret)?;
        c.bytes.push(0x0b);
    }
    Ok(())
}
fn kept(frame: &Frame, lsda: bool) -> Result<Vec<u8>, Error> {
    let mut c = Cursor::new();
    if frame.established_end == 0 {
        return Ok(c.bytes);
    }
    c.advance(frame.push_end)?;
    c.bytes.extend_from_slice(&[0x0e, 16, 0x86, 2]);
    c.advance(frame.established_end)?;
    c.bytes.extend_from_slice(&[0x0d, 6]);
    for &(end, reg, offset) in &frame.saves {
        c.advance(end)?;
        c.bytes.push(0x80 | DWARF[usize::from(reg.code())]);
        uleb(
            &mut c.bytes,
            u64::from(16_u32.wrapping_sub(offset.cast_unsigned()) / 8),
        );
    }
    let mut args = 0;
    let mut next_return = 0;
    for call in &frame.calls {
        while frame
            .returns
            .get(next_return)
            .is_some_and(|r| r.released <= call.begin)
        {
            let r = frame.returns[next_return];
            kept_epilogue(&mut c, frame, r.released, r.returned, true)?;
            next_return += 1;
        }
        if !lsda || call.landing.is_none() || call.args == args || call.begin >= frame.ret_end {
            continue;
        }
        c.advance(call.begin)?;
        c.args(call.args);
        args = call.args;
    }
    for r in &frame.returns[next_return..] {
        kept_epilogue(&mut c, frame, r.released, r.returned, true)?;
    }
    kept_epilogue(
        &mut c,
        frame,
        frame.leave_end,
        frame.ret_end,
        frame.size > frame.ret_end,
    )?;
    Ok(c.bytes)
}
fn prologues(c: &mut Cursor, f: &Frame) -> Result<u32, Error> {
    let mut only = Prologue {
        reserve_end: f.reserve_end,
        ..Prologue::default()
    };
    for (slot, &(end, _, _)) in only.save_end.iter_mut().zip(&f.saves) {
        *slot = end;
    }
    let setups = if f.prologues.is_empty() {
        std::slice::from_ref(&only)
    } else {
        &f.prologues
    };
    let mut below = 8_u32;
    for (index, p) in setups.iter().enumerate() {
        if index != 0 {
            c.advance(p.start)?;
            c.offset(8);
            for &(_, reg, _) in &f.saves {
                c.bytes.push(0xc0 | DWARF[usize::from(reg.code())]);
            }
        }
        below = 8_u32.wrapping_add(f.reserve);
        if f.reserve != 0 {
            c.advance(p.reserve_end)?;
            c.offset(below);
        }
        for (&end, &(_, reg, offset)) in p.save_end.iter().zip(&f.saves) {
            c.advance(end)?;
            below = below.wrapping_add(8);
            c.offset(below);
            c.bytes.push(0x80 | DWARF[usize::from(reg.code())]);
            uleb(
                &mut c.bytes,
                u64::from(16_u32.wrapping_sub(offset.cast_unsigned()) / 8),
            );
        }
    }
    Ok(below)
}
struct Events {
    adjust: usize,
    call: usize,
    args: u32,
}
impl Events {
    fn before(
        &mut self,
        c: &mut Cursor,
        f: &Frame,
        body: u32,
        before: u32,
        lsda: bool,
    ) -> Result<(), Error> {
        loop {
            while let Some(call) = f.calls.get(self.call) {
                if lsda
                    && call.landing.is_some()
                    && call.args != self.args
                    && call.begin < f.ret_end
                {
                    break;
                }
                self.call += 1;
            }
            let adjust = f.adjusts.get(self.adjust).map_or(u32::MAX, |a| a.end);
            let call = f.calls.get(self.call).map_or(u32::MAX, |a| a.begin);
            if adjust >= before && call >= before {
                return Ok(());
            }
            if adjust <= call {
                c.advance(adjust)?;
                c.offset(body.wrapping_add(f.adjusts[self.adjust].args));
                self.adjust += 1;
            } else {
                c.advance(call)?;
                c.args(f.calls[self.call].args);
                self.args = f.calls[self.call].args;
                self.call += 1;
            }
        }
    }
}
fn omitted_epilogue(
    c: &mut Cursor,
    f: &Frame,
    body: u32,
    epilogue: super::host_unwind::Epilogue,
    more: bool,
) -> Result<(), Error> {
    let super::host_unwind::Epilogue {
        released: leave,
        returned: ret,
        pops,
    } = epilogue;
    if body == 8 {
        return Ok(());
    }
    if more {
        c.bytes.push(0x0a);
    }
    let mut at = body;
    for (&end, &(_, reg, _)) in pops.iter().zip(f.saves.iter().rev()) {
        c.advance(end)?;
        at = at.wrapping_sub(8);
        c.offset(at);
        c.bytes.push(0xc0 | DWARF[usize::from(reg.code())]);
    }
    if f.reserve != 0 {
        c.advance(leave)?;
        c.offset(8);
    }
    if more {
        c.advance(ret)?;
        c.bytes.push(0x0b);
    }
    Ok(())
}
fn omitted(f: &Frame, lsda: bool) -> Result<Vec<u8>, Error> {
    let mut c = Cursor::new();
    let body = prologues(&mut c, f)?;
    let mut events = Events {
        adjust: 0,
        call: 0,
        args: 0,
    };
    for index in 0..=f.returns.len() {
        let shared = index == f.returns.len();
        let (pops, leave, ret) = if shared {
            (&f.pop_end, f.leave_end, f.ret_end)
        } else {
            let r = &f.returns[index];
            (&r.pops, r.released, r.returned)
        };
        events.before(
            &mut c,
            f,
            body,
            if f.saves.is_empty() { leave } else { pops[0] },
            lsda,
        )?;
        omitted_epilogue(
            &mut c,
            f,
            body,
            super::host_unwind::Epilogue {
                pops: *pops,
                released: leave,
                returned: ret,
            },
            !shared || f.size > f.ret_end,
        )?;
    }
    Ok(c.bytes)
}
pub(super) fn encode(frame: &Frame, lsda: bool) -> Result<Vec<u8>, Error> {
    if frame.frameless {
        omitted(frame, lsda)
    } else {
        kept(frame, lsda)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::{
        host_unwind::{Call, Epilogue},
        region_flow::LandingId,
    };
    #[test]
    fn kept_frames_merge_call_rules_with_duplicated_epilogues_in_address_order() {
        let frame = Frame {
            push_end: 1,
            established_end: 4,
            returns: vec![Epilogue {
                released: 10,
                returned: 11,
                ..Epilogue::default()
            }],
            calls: vec![Call {
                begin: 16,
                end: 19,
                landing: Some(LandingId::new(0).unwrap()),
                args: 16,
            }],
            leave_end: 20,
            ret_end: 21,
            size: 30,
            ..Frame::default()
        };
        assert_eq!(
            encode(&frame, true).unwrap(),
            [
                0x41, 0x0e, 16, 0x86, 2, 0x43, 0x0d, 6, 0x46, 0x0a, 0x0c, 7, 8, 0xc6, 0x41, 0x0b,
                0x45, 0x2e, 16, 0x44, 0x0a, 0x0c, 7, 8, 0xc6, 0x41, 0x0b
            ]
        );
    }
    #[test]
    fn target_cfi_cursor_retains_unsigned_wrap_for_reordered_recorded_events() {
        let mut c = Cursor::new();
        c.advance(100).unwrap();
        c.advance(4).unwrap();
        assert_eq!(c.bytes, [2, 100, 4, 0xa0, 0xff, 0xff, 0xff]);
    }
}

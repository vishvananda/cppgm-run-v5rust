//! Host frame rules from final encoder offsets (`object/host_unwind.cpp`). See NOTICE.
use super::x86::Register;
use crate::lowir::Error;

#[derive(Default)]
pub(super) struct Frame {
    pub(super) push_end: u32,
    pub(super) established_end: u32,
    pub(super) saves: Vec<(u32, Register, i32)>,
    pub(super) leave_end: u32,
    pub(super) ret_end: u32,
}
fn uleb(out: &mut Vec<u8>, mut value: u64) {
    loop {
        let byte = (value & 127) as u8;
        value >>= 7;
        out.push(byte | if value == 0 { 0 } else { 128 });
        if value == 0 {
            break;
        }
    }
}
fn finish(out: &mut Vec<u8>, start: usize) -> Result<(), Error> {
    while !(out.len() - start).is_multiple_of(8) {
        out.push(0);
    }
    let length = u32::try_from(out.len() - start - 4).map_err(|_| Error::Limit)?;
    out[start..start + 4].copy_from_slice(&length.to_le_bytes());
    Ok(())
}
fn advance(out: &mut Vec<u8>, at: &mut u32, to: u32) -> Result<(), Error> {
    let delta = to
        .checked_sub(*at)
        .ok_or_else(|| Error::Construction("unordered frame offsets".into()))?;
    match delta {
        0 => {}
        1..64 => out.push(0x40 | u8::try_from(delta).map_err(|_| Error::Limit)?),
        64..256 => {
            out.push(2);
            out.push(u8::try_from(delta).map_err(|_| Error::Limit)?);
        }
        256..65536 => {
            out.push(3);
            out.extend_from_slice(
                &u16::try_from(delta)
                    .map_err(|_| Error::Limit)?
                    .to_le_bytes(),
            );
        }
        _ => {
            out.push(4);
            out.extend_from_slice(&delta.to_le_bytes());
        }
    }
    *at = to;
    Ok(())
}
impl Frame {
    pub(super) fn encode(&self) -> Result<Vec<u8>, Error> {
        const DWARF: [u8; 16] = [0, 2, 1, 3, 7, 6, 4, 5, 8, 9, 10, 11, 12, 13, 14, 15];
        let mut out = Vec::new();
        if self.established_end == 0 {
            return Ok(out);
        }
        let mut at = 0;
        advance(&mut out, &mut at, self.push_end)?;
        out.extend_from_slice(&[0x0e, 16, 0x86, 2]);
        advance(&mut out, &mut at, self.established_end)?;
        out.extend_from_slice(&[0x0d, 6]);
        for &(end, reg, offset) in &self.saves {
            advance(&mut out, &mut at, end)?;
            out.push(0x80 | DWARF[usize::from(reg.code())]);
            uleb(
                &mut out,
                u64::from(16_u32.wrapping_sub(offset.cast_unsigned()) / 8),
            );
        }
        advance(&mut out, &mut at, self.leave_end)?;
        out.extend_from_slice(&[0x0c, 7, 8, 0xc6]);
        for &(_, reg, _) in &self.saves {
            out.push(0xc0 | DWARF[usize::from(reg.code())]);
        }
        Ok(out)
    }
}
pub(super) fn cie(out: &mut Vec<u8>) -> Result<(), Error> {
    let start = out.len();
    out.extend_from_slice(&[
        0, 0, 0, 0, 0, 0, 0, 0, 1, b'z', b'R', 0, 1, 0x78, 16, 1, 0x1b, 0x0c, 7, 8, 0x90, 1,
    ]);
    finish(out, start)
}
pub(super) fn fde(out: &mut Vec<u8>, size: u32, cfi: &[u8]) -> Result<u32, Error> {
    let start = out.len();
    out.extend_from_slice(&[0; 4]);
    let cie = u32::try_from(out.len()).map_err(|_| Error::Limit)?;
    out.extend_from_slice(&cie.to_le_bytes());
    let field = u32::try_from(out.len()).map_err(|_| Error::Limit)?;
    out.extend_from_slice(&[0; 4]);
    out.extend_from_slice(&size.to_le_bytes());
    out.push(0);
    out.extend_from_slice(cfi);
    finish(out, start)?;
    Ok(field)
}

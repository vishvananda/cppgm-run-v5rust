//! Host frame rules from final encoder offsets (`object/host_unwind.cpp`). See NOTICE.
use super::x86::Register;
use crate::lowir::Error;

#[derive(Default)]
pub(super) struct Frame {
    pub(super) frameless: bool,
    pub(super) reserve: u32,
    pub(super) reserve_end: u32,
    pub(super) pop_end: [u32; 6],
    pub(super) returns: Vec<Epilogue>,
    pub(super) prologues: Vec<Prologue>,
    pub(super) adjusts: Vec<Adjust>,
    pub(super) push_end: u32,
    pub(super) established_end: u32,
    pub(super) saves: Vec<(u32, Register, i32)>,
    pub(super) leave_end: u32,
    pub(super) ret_end: u32,
    pub(super) size: u32,
    pub(super) calls: Vec<Call>,
    pub(super) pads: Vec<u32>,
}
#[derive(Default, Clone, Copy)]
pub(super) struct Epilogue {
    pub(super) released: u32,
    pub(super) returned: u32,
    pub(super) pops: [u32; 6],
}
#[derive(Default, Clone, Copy)]
pub(super) struct Prologue {
    pub(super) start: u32,
    pub(super) reserve_end: u32,
    pub(super) save_end: [u32; 6],
}
#[derive(Clone, Copy)]
pub(super) struct Adjust {
    pub(super) end: u32,
    pub(super) args: u32,
}
#[derive(Clone, Copy)]
pub(super) struct Call {
    pub(super) begin: u32,
    pub(super) end: u32,
    pub(super) landing: Option<super::region_flow::LandingId>,
    pub(super) args: u32,
}
pub(super) fn uleb(out: &mut Vec<u8>, mut value: u64) {
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
pub(super) fn advance(out: &mut Vec<u8>, at: &mut u32, to: u32) -> Result<(), Error> {
    // The frozen CFI cursor uses target u32 arithmetic, including when
    // separately recorded prologue/body events revisit an earlier offset.
    let delta = to.wrapping_sub(*at);
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
    pub(super) fn encode(&self, lsda: bool) -> Result<Vec<u8>, Error> {
        super::unwind_rules::encode(self, lsda)
    }
}
pub(super) fn cie(out: &mut Vec<u8>, personality: bool) -> Result<Option<u32>, Error> {
    let start = out.len();
    out.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0, 0, 1]);
    out.extend_from_slice(if personality { b"zPLR\0" } else { b"zR\0" });
    out.extend_from_slice(&[1, 0x78, 16]);
    let field = if personality {
        out.extend_from_slice(&[7, 0x9b]);
        let field = u32::try_from(out.len()).map_err(|_| Error::Limit)?;
        out.extend_from_slice(&[0, 0, 0, 0, 0x1b, 0x1b]);
        Some(field)
    } else {
        out.extend_from_slice(&[1, 0x1b]);
        None
    };
    out.extend_from_slice(&[0x0c, 7, 8, 0x90, 1]);
    finish(out, start)?;
    Ok(field)
}
pub(super) fn fde(
    out: &mut Vec<u8>,
    size: u32,
    cfi: &[u8],
    cie_offset: u32,
    lsda: bool,
) -> Result<(u32, Option<u32>), Error> {
    let start = out.len();
    out.extend_from_slice(&[0; 4]);
    let cie = u32::try_from(out.len())
        .map_err(|_| Error::Limit)?
        .checked_sub(cie_offset)
        .ok_or(Error::Limit)?;
    out.extend_from_slice(&cie.to_le_bytes());
    let field = u32::try_from(out.len()).map_err(|_| Error::Limit)?;
    out.extend_from_slice(&[0; 4]);
    out.extend_from_slice(&size.to_le_bytes());
    out.push(if lsda { 4 } else { 0 });
    let lsda_field = if lsda {
        let field = u32::try_from(out.len()).map_err(|_| Error::Limit)?;
        out.extend_from_slice(&[0; 4]);
        Some(field)
    } else {
        None
    };
    out.extend_from_slice(cfi);
    finish(out, start)?;
    Ok((field, lsda_field))
}

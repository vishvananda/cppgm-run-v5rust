//! Static Linux ELF layout and explicit little-endian serialization. See NOTICE.
use super::image::{Image, align_up};
use crate::lowir::Error;
use std::{
    io::{self, Write},
    path::Path,
};
const BASE: u64 = 0x0040_0000;
const PAGE: u64 = 4096;
pub(super) struct Layout {
    text_offset: u64,
    pub(super) text_address: u64,
    rodata_offset: u64,
    pub(super) rodata_address: u64,
    data_offset: u64,
    pub(super) data_address: u64,
    pub(super) bss_address: u64,
    headers: u16,
    has_rodata: bool,
    has_data: bool,
}
impl Layout {
    pub(super) fn new(image: &Image) -> Result<Self, Error> {
        Self::with_alignment(image, [1; 4])
    }
    pub(super) fn with_alignment(image: &Image, align: [u64; 4]) -> Result<Self, Error> {
        let has_rodata = !image.rodata.is_empty();
        let has_data = !image.data.is_empty() || image.bss != 0;
        let headers = 2 + u16::from(has_rodata) + u16::from(has_data);
        let text_offset = align_up(64 + 56 * u64::from(headers), align[0].max(16))?;
        let text_end = text_offset
            .checked_add(u64::try_from(image.text.len()).map_err(|_| Error::Limit)?)
            .ok_or(Error::Limit)?;
        let rodata_offset = align_up(text_end, align[1].max(PAGE))?;
        let rodata_end = rodata_offset
            .checked_add(u64::try_from(image.rodata.len()).map_err(|_| Error::Limit)?)
            .ok_or(Error::Limit)?;
        let data_offset = align_up(
            if has_rodata { rodata_end } else { text_end },
            align[2].max(PAGE),
        )?;
        let data_address = BASE + data_offset;
        let bss_address = align_up(
            data_address
                .checked_add(u64::try_from(image.data.len()).map_err(|_| Error::Limit)?)
                .ok_or(Error::Limit)?,
            align[3].max(64),
        )?;
        Ok(Self {
            text_offset,
            text_address: BASE + text_offset,
            rodata_offset,
            rodata_address: BASE + rodata_offset,
            data_offset,
            data_address,
            bss_address,
            headers,
            has_rodata,
            has_data,
        })
    }
}
fn put16(out: &mut Vec<u8>, value: u16) {
    out.extend(value.to_le_bytes());
}
fn put32(out: &mut Vec<u8>, value: u32) {
    out.extend(value.to_le_bytes());
}
fn put64(out: &mut Vec<u8>, value: u64) {
    out.extend(value.to_le_bytes());
}
#[expect(
    clippy::too_many_arguments,
    reason = "ELF program-header fields are independent serialized values"
)]
fn program_header(
    out: &mut Vec<u8>,
    ty: u32,
    flags: u32,
    offset: u64,
    address: u64,
    file: u64,
    memory: u64,
    align: u64,
) {
    put32(out, ty);
    put32(out, flags);
    put64(out, offset);
    put64(out, address);
    put64(out, address);
    put64(out, file);
    put64(out, memory);
    put64(out, align);
}
pub(super) fn write(
    path: &Path,
    layout: &Layout,
    image: &Image,
    entry: u64,
    permissions: Option<&std::fs::Permissions>,
) -> Result<(), Error> {
    let data_file = image
        .data
        .iter()
        .rposition(|&byte| byte != 0)
        .map_or(0, |index| index + 1);
    let mut header = vec![0x7f, b'E', b'L', b'F', 2, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    put16(&mut header, 2);
    put16(&mut header, 62);
    put32(&mut header, 1);
    put64(&mut header, entry);
    put64(&mut header, 64);
    put64(&mut header, 0);
    put32(&mut header, 0);
    for value in [64, 56, layout.headers, 0, 0, 0] {
        put16(&mut header, value);
    }
    let text_end =
        layout.text_offset + u64::try_from(image.text.len()).map_err(|_| Error::Limit)?;
    program_header(&mut header, 1, 5, 0, BASE, text_end, text_end, PAGE);
    if layout.has_rodata {
        let bytes = u64::try_from(image.rodata.len()).map_err(|_| Error::Limit)?;
        program_header(
            &mut header,
            1,
            4,
            layout.rodata_offset,
            layout.rodata_address,
            bytes,
            bytes,
            PAGE,
        );
    }
    if layout.has_data {
        program_header(
            &mut header,
            1,
            6,
            layout.data_offset,
            layout.data_address,
            u64::try_from(data_file).map_err(|_| Error::Limit)?,
            layout.bss_address + image.bss - layout.data_address,
            PAGE,
        );
    }
    program_header(&mut header, 0x6474_e551, 6, 0, 0, 0, 0, 16);
    header.resize(
        usize::try_from(layout.text_offset).map_err(|_| Error::Limit)?,
        0,
    );
    write_sections(path, layout, image, &header, data_file)
        .map_err(|_| Error::argument("cannot write ", path.as_os_str()))?;
    // The frozen writer ignores chmod failure after a successful close.
    if let Some(permissions) = permissions {
        let _ = std::fs::set_permissions(path, permissions.clone());
    }
    Ok(())
}
fn write_sections(
    path: &Path,
    layout: &Layout,
    image: &Image,
    header: &[u8],
    data_file: usize,
) -> io::Result<()> {
    let mut out = std::io::BufWriter::new(std::fs::File::create(path)?);
    out.write_all(header)?;
    out.write_all(&image.text)?;
    let mut position =
        layout.text_offset + u64::try_from(image.text.len()).map_err(io::Error::other)?;
    for (present, offset, bytes) in [
        (
            layout.has_rodata,
            layout.rodata_offset,
            image.rodata.as_slice(),
        ),
        (
            layout.has_data,
            layout.data_offset,
            &image.data[..data_file],
        ),
    ] {
        if !present {
            continue;
        }
        while position < offset {
            let n = usize::try_from((offset - position).min(256)).map_err(io::Error::other)?;
            out.write_all(&[0; 256][..n])?;
            position += u64::try_from(n).map_err(io::Error::other)?;
        }
        out.write_all(bytes)?;
        position += u64::try_from(bytes.len()).map_err(io::Error::other)?;
    }
    out.flush()
}

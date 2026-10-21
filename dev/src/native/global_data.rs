//! Global spans, target bytes and typed address fixups in definition order. See NOTICE.
use super::{
    abi::{value_align, value_bytes},
    image::Image,
};
use crate::{
    lowir::{
        Error,
        literals::{self, Constant, FloatKind},
        model::{DataItem, Global, GlobalInit, Program},
        vocabulary::Type,
    },
    types::floating::FloatingScanner,
};
fn item_type(item: DataItem) -> Option<Type> {
    match item {
        DataItem::Zero(_) => None,
        DataItem::Constant(ty, _) => Some(ty),
        DataItem::Address(..) => Some(Type::Ptr),
    }
}

fn target_align(value: u64, align: u64) -> u64 {
    value.wrapping_add(align - 1) & !(align - 1)
}

pub(super) fn layout(global: &Global) -> Result<(u64, u32), Error> {
    let GlobalInit::Data(items) = &global.init else {
        let ty = global.ty.unwrap_or(Type::Ptr);
        let bytes = value_bytes(ty)?;
        return Ok((
            u64::from(if bytes == 0 { 8 } else { bytes }),
            value_align(ty)?.min(16),
        ));
    };
    let mut size = 0;
    let mut align = 1;
    let mut typed = false;
    for &item in items {
        if let Some(ty) = item_type(item) {
            let a = value_align(ty)?.min(16);
            typed = true;
            align = align.max(a);
            size = target_align(size, u64::from(a)).wrapping_add(u64::from(value_bytes(ty)?));
        } else if let DataItem::Zero(amount) = item {
            size = size.wrapping_add(amount.cast_unsigned());
        }
    }
    if let Some(Type::Object { bytes, align: log }) = global.ty {
        align = align.max(1_u32.wrapping_shl(u32::from(log)));
        size = size.max(bytes);
    } else if !typed {
        align = 16;
    }
    Ok((size.max(1), align))
}
pub(super) fn constant_bytes(
    scanner: &mut FloatingScanner,
    ty: Type,
    value: Constant,
) -> Result<[u8; 16], Error> {
    let bits = if let Some(kind) = FloatKind::from_type(ty) {
        literals::native_float(scanner, value, kind)?
    } else {
        match value {
            Constant::Integer(bits) | Constant::Float(_, bits) => bits,
        }
    };
    let mut bytes = bits.to_le_bytes();
    // x87 payload occupies ten bytes; the ABI's remaining six stay zero.
    if ty == Type::F80 {
        bytes[10..].fill(0);
    }
    Ok(bytes)
}
pub(super) fn emit(program: &Program, image: &mut Image) -> Result<(), Error> {
    let mut scanner = FloatingScanner::default();
    let mut layouts = Vec::new();
    let mut reserve = 0_u64;
    for global in program.globals_in_publication_order() {
        if matches!(global.init, GlobalInit::Declaration) {
            continue;
        }
        let (bytes, align) = layout(global)?;
        reserve = reserve
            .checked_add(bytes)
            .and_then(|n| n.checked_add(u64::from(align)))
            .ok_or(Error::Limit)?;
        layouts.push((global, bytes, align));
    }
    image
        .data
        .try_reserve(usize::try_from(reserve).map_err(|_| Error::Limit)?)
        .map_err(|_| Error::Limit)?;
    let mut addresses = Vec::new();
    for (global, bytes, align) in layouts {
        let symbol = program.find_symbol(global.name).ok_or(Error::Limit)?;
        let base = image.reserve_data(symbol, bytes, u64::from(align))?;
        addresses.clear();
        write_global(global, &mut image.data, base, &mut addresses, &mut scanner)?;
        for address in &addresses {
            image.address_item(
                base.checked_add(usize::try_from(address.offset).map_err(|_| Error::Limit)?)
                    .ok_or(Error::Limit)?,
                address.target,
                address.addend,
            );
        }
    }
    Ok(())
}
pub(super) struct Address {
    pub(super) offset: u64,
    pub(super) target: crate::lowir::model::SymbolId,
    pub(super) addend: i64,
}
pub(super) fn write_global(
    global: &Global,
    out: &mut [u8],
    base: usize,
    addresses: &mut Vec<Address>,
    scanner: &mut FloatingScanner,
) -> Result<(), Error> {
    match &global.init {
        GlobalInit::Constant(value) => {
            store(out, base, global.ty.unwrap_or(Type::Ptr), *value, scanner)?;
        }
        GlobalInit::Address(target, addend) => addresses.push(Address {
            offset: 0,
            target: *target,
            addend: *addend,
        }),
        GlobalInit::Data(items) => {
            let mut at = 0_u64;
            for &item in items {
                if let DataItem::Zero(bytes) = item {
                    at = at.wrapping_add(bytes.cast_unsigned());
                    continue;
                }
                let ty = item_type(item).ok_or(Error::Limit)?;
                at = target_align(at, u64::from(value_align(ty)?.min(16)));
                let offset = base
                    .checked_add(usize::try_from(at).map_err(|_| Error::Limit)?)
                    .ok_or(Error::Limit)?;
                match item {
                    DataItem::Address(_, target, addend) => addresses.push(Address {
                        offset: at,
                        target,
                        addend,
                    }),
                    DataItem::Constant(ty, value) => store(out, offset, ty, value, scanner)?,
                    DataItem::Zero(_) => {}
                }
                at = at.wrapping_add(u64::from(value_bytes(ty)?));
            }
        }
        GlobalInit::Declaration | GlobalInit::Zero => {}
    }
    Ok(())
}
fn store(
    out: &mut [u8],
    offset: usize,
    ty: Type,
    value: Constant,
    scanner: &mut FloatingScanner,
) -> Result<(), Error> {
    let bytes = constant_bytes(scanner, ty, value)?;
    let size = usize::try_from(value_bytes(ty)?).map_err(|_| Error::Limit)?;
    if size > bytes.len() {
        return Err(Error::NotImplemented("native object constant payload"));
    }
    out.get_mut(offset..offset.checked_add(size).ok_or(Error::Limit)?)
        .ok_or(Error::Limit)?
        .copy_from_slice(&bytes[..size]);
    Ok(())
}
pub(super) fn is_zero(global: &Global, scanner: &mut FloatingScanner) -> Result<bool, Error> {
    let zero = |scanner: &mut FloatingScanner, ty, value| -> Result<bool, Error> {
        let bytes = constant_bytes(scanner, ty, value)?;
        let size = usize::try_from(value_bytes(ty)?).map_err(|_| Error::Limit)?;
        Ok(bytes
            .get(..size)
            .ok_or(Error::Limit)?
            .iter()
            .all(|&byte| byte == 0))
    };
    match &global.init {
        GlobalInit::Zero => Ok(true),
        GlobalInit::Constant(value) => zero(scanner, global.ty.unwrap_or(Type::Ptr), *value),
        GlobalInit::Data(items) => {
            for item in items {
                match *item {
                    DataItem::Address(..) => return Ok(false),
                    DataItem::Constant(ty, value) if !zero(scanner, ty, value)? => {
                        return Ok(false);
                    }
                    _ => {}
                }
            }
            Ok(true)
        }
        _ => Ok(false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lowir::model::Metadata;
    #[test]
    fn raw_padding_does_not_raise_typed_item_alignment() -> Result<(), Error> {
        let mut program = Program::default();
        let name = program.names.intern("data")?;
        let mut global = Global {
            name,
            ty: None,
            metadata: Metadata::default(),
            init: GlobalInit::Data(vec![
                DataItem::Zero(7),
                DataItem::Constant(Type::I32, Constant::Integer(1)),
                DataItem::Zero(1),
                DataItem::Constant(Type::I16, Constant::Integer(2)),
            ]),
        };
        assert_eq!(layout(&global)?, (16, 4));
        global.ty = Some(Type::Object {
            bytes: 64,
            align: 8,
        });
        assert_eq!(layout(&global)?, (64, 256));
        global.ty = Some(Type::Object {
            bytes: 1_u64 << 40,
            align: 40,
        });
        assert_eq!(layout(&global)?, (1_u64 << 40, 256));
        global.ty = None;
        global.init = GlobalInit::Data(vec![DataItem::Zero(3)]);
        assert_eq!(layout(&global)?, (3, 16));
        global.init = GlobalInit::Data(vec![DataItem::Zero(-1), DataItem::Zero(1)]);
        assert_eq!(layout(&global)?, (1, 16));
        global.init = GlobalInit::Data(vec![
            DataItem::Zero(-1),
            DataItem::Constant(Type::I64, Constant::Integer(7)),
        ]);
        assert_eq!(layout(&global)?, (8, 8));
        Ok(())
    }
}

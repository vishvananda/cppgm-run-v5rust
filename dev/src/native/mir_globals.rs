//! Global MIR views: definition order and literal bits are retained. See NOTICE.
use crate::{
    lowir::{
        Error,
        literals::{self, Constant},
        model::{DataItem, GlobalInit, Program, SymbolId},
        vocabulary::Type,
    },
    types::floating::FloatingScanner,
};
use std::io::Write;

pub(super) fn write(program: &Program, out: &mut impl Write) -> Result<(), Error> {
    let mut floating = FloatingScanner::default();
    for global in program.globals_in_publication_order() {
        if matches!(global.init, GlobalInit::Declaration) {
            continue;
        }
        out.write_all(b"\nglobal @").map_err(Error::Output)?;
        out.write_all(program.names.bytes(global.name))
            .map_err(Error::Output)?;
        if let Some(storage) = global.metadata.storage {
            write!(out, " {}", storage.spelling()).map_err(Error::Output)?;
        }
        writeln!(out).map_err(Error::Output)?;
        if let GlobalInit::Data(items) = &global.init {
            out.write_all(b"  storage data\n").map_err(Error::Output)?;
            for item in items {
                out.write_all(b"  item ").map_err(Error::Output)?;
                match *item {
                    DataItem::Zero(amount) => {
                        write!(out, "zero {amount}").map_err(Error::Output)?;
                    }
                    DataItem::Address(_, target, addend) => {
                        out.write_all(b"ptr ").map_err(Error::Output)?;
                        address(program, target, addend, out).map_err(Error::Output)?;
                    }
                    DataItem::Constant(ty, value) => {
                        write!(out, "{ty} ").map_err(Error::Output)?;
                        constant(&mut floating, ty, value, out)?;
                    }
                }
                writeln!(out).map_err(Error::Output)?;
            }
            continue;
        }
        writeln!(out, "  storage scalar {}", global.ty.unwrap_or(Type::Ptr))
            .map_err(Error::Output)?;
        out.write_all(b"  init ").map_err(Error::Output)?;
        match global.init {
            GlobalInit::Address(target, addend) => {
                address(program, target, addend, out).map_err(Error::Output)?;
            }
            GlobalInit::Constant(value) => {
                let ty = global.ty.unwrap_or(Type::Void);
                write!(out, "{ty} ").map_err(Error::Output)?;
                constant(&mut floating, ty, value, out)?;
            }
            GlobalInit::Zero => {
                let ty = global.ty.unwrap_or(Type::Void);
                write!(
                    out,
                    "{ty} {}",
                    if ty.float_rank() == 0 { "0" } else { "0.0" }
                )
                .map_err(Error::Output)?;
            }
            GlobalInit::Declaration | GlobalInit::Data(_) => {}
        }
        writeln!(out).map_err(Error::Output)?;
    }
    Ok(())
}

fn address(
    program: &Program,
    target: SymbolId,
    addend: i64,
    out: &mut impl Write,
) -> std::io::Result<()> {
    out.write_all(b"addr @")?;
    out.write_all(program.names.bytes(program.symbol(target).name))?;
    if addend != 0 {
        // The frozen writer negates in the signed target word before printing.
        // Its minimum addend therefore has two minus signs in the MIR view.
        let magnitude = if addend < 0 {
            addend.wrapping_neg()
        } else {
            addend
        };
        write!(out, "{}{magnitude}", if addend < 0 { '-' } else { '+' })?;
    }
    Ok(())
}

fn constant(
    floating: &mut FloatingScanner,
    ty: Type,
    value: Constant,
    out: &mut impl Write,
) -> Result<(), Error> {
    match value {
        Constant::Float(kind, bits) => out
            .write_all(literals::mir_text(floating, kind, bits)?.as_bytes())
            .map_err(Error::Output),
        Constant::Integer(bits) => {
            let low = u64::try_from(bits & u128::from(u64::MAX)).map_err(|_| Error::Limit)?;
            let signed = i64::from_le_bytes(low.to_le_bytes());
            if ty == Type::I128 && bits >> 64 != if signed < 0 { u128::from(u64::MAX) } else { 0 } {
                write!(out, "0x{bits:032x}")
            } else {
                write!(out, "{signed}")
            }
            .map_err(Error::Output)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minimum_addend_retains_frozen_signed_word_display() -> Result<(), Error> {
        let mut program = Program::default();
        let name = program.names.intern("target")?;
        let source = program.add_source(std::path::Path::new(""))?;
        let target = program.reference(name, source, 0)?;
        let mut text = Vec::new();
        address(&program, target, i64::MIN, &mut text).map_err(Error::Output)?;
        assert_eq!(text, b"addr @target--9223372036854775808");
        Ok(())
    }
}

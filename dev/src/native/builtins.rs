//! Frozen standalone string and memory routines, encoded directly. See NOTICE.
use super::x86::{Assembler, Register as R, Rm};
use crate::lowir::Error;
// C++ strcmp against fixed vocabulary stops at its first mismatch. Preserve
// its first-NUL boundary without scanning an arbitrary owned object name.
pub(super) fn named(name: &[u8], expected: &[u8]) -> bool {
    name.starts_with(expected) && name.get(expected.len()).is_none_or(|&byte| byte == 0)
}
pub(super) fn emit(a: &mut Assembler, name: &[u8]) -> Result<bool, Error> {
    match name {
        name if named(name, b"cppgm_builtin_strlen") => {
            let chunk = R::xmm(1)?;
            let zero = R::xmm(0)?;
            let next = a.label()?;
            let first = a.label()?;
            a.mov_rr(R::RAX, R::RDI)?;
            a.op(0, false, 0x8b, 1, R::RCX.code(), Rm::Reg(R::RDI), false, 0)?;
            a.alu_ri(4, Rm::Reg(R::RAX), -16, 8)?;
            a.alu_ri(4, Rm::Reg(R::RCX), 15, 4)?;
            a.sse(0x66, 0xef, zero, Rm::Reg(zero))?;
            a.sse(0x66, 0x6f, chunk, Rm::mem(R::RAX, 0))?;
            a.sse(0x66, 0x74, chunk, Rm::Reg(zero))?;
            a.op(
                0x66,
                false,
                0x0fd7,
                2,
                R::RDX.code(),
                Rm::Reg(chunk),
                false,
                0,
            )?;
            a.shift_cl(5, Rm::Reg(R::RDX), 4)?;
            a.op(0, false, 0x85, 1, R::RDX.code(), Rm::Reg(R::RDX), false, 0)?;
            a.branch(Some(5), first, true);
            a.bind(next);
            a.alu_ri(0, Rm::Reg(R::RAX), 16, 8)?;
            a.sse(0x66, 0x6f, chunk, Rm::mem(R::RAX, 0))?;
            a.sse(0x66, 0x74, chunk, Rm::Reg(zero))?;
            a.op(
                0x66,
                false,
                0x0fd7,
                2,
                R::RDX.code(),
                Rm::Reg(chunk),
                false,
                0,
            )?;
            a.op(0, false, 0x85, 1, R::RDX.code(), Rm::Reg(R::RDX), false, 0)?;
            a.branch(Some(4), next, true);
            a.op(
                0,
                false,
                0x0fbc,
                2,
                R::RDX.code(),
                Rm::Reg(R::RDX),
                false,
                0,
            )?;
            a.alu_rr(0, R::RAX, Rm::Reg(R::RDX), 8)?;
            a.alu_rr(5, R::RAX, Rm::Reg(R::RDI), 8)?;
            a.ret();
            a.bind(first);
            a.op(
                0,
                false,
                0x0fbc,
                2,
                R::RAX.code(),
                Rm::Reg(R::RDX),
                false,
                0,
            )?;
            a.ret();
        }
        name if named(name, b"cppgm_builtin_memcpy") => {
            a.mov_rr(R::RAX, R::RDI)?;
            a.mov_rr(R::RCX, R::RDX)?;
            a.byte(0xf3);
            a.byte(0xa4);
            a.ret();
        }
        name if named(name, b"cppgm_builtin_memmove") => {
            let forward = a.label()?;
            a.mov_rr(R::RAX, R::RDI)?;
            a.mov_rr(R::RCX, R::RDX)?;
            a.alu_rr(7, R::RDI, Rm::Reg(R::RSI), 8)?;
            a.branch(Some(6), forward, true);
            a.lea(R::R11, Rm::indexed(R::RSI, 0, R::RDX))?;
            a.alu_rr(7, R::RDI, Rm::Reg(R::R11), 8)?;
            a.branch(Some(3), forward, true);
            a.lea(R::RSI, Rm::indexed(R::RSI, -1, R::RDX))?;
            a.lea(R::RDI, Rm::indexed(R::RDI, -1, R::RDX))?;
            a.byte(0xfd);
            a.byte(0xf3);
            a.byte(0xa4);
            a.byte(0xfc);
            a.ret();
            a.bind(forward);
            a.byte(0xf3);
            a.byte(0xa4);
            a.ret();
        }
        name if named(name, b"cppgm_builtin_memset") => {
            let large = a.label()?;
            let next = a.label()?;
            let done = a.label()?;
            a.mov_rr(R::R11, R::RDI)?;
            a.op(0, false, 0x8b, 1, R::RAX.code(), Rm::Reg(R::RSI), false, 0)?;
            a.alu_ri(7, Rm::Reg(R::RDX), 32, 8)?;
            a.branch(Some(3), large, true);
            a.op(0, true, 0x85, 1, R::RDX.code(), Rm::Reg(R::RDX), false, 0)?;
            a.branch(Some(4), done, true);
            a.bind(next);
            a.store(Rm::mem(R::RDI, 0), R::RAX, 1)?;
            a.alu_ri(0, Rm::Reg(R::RDI), 1, 8)?;
            a.alu_ri(5, Rm::Reg(R::RDX), 1, 8)?;
            a.branch(Some(5), next, true);
            a.bind(done);
            a.mov_rr(R::RAX, R::R11)?;
            a.ret();
            a.bind(large);
            a.mov_rr(R::RCX, R::RDX)?;
            a.byte(0xf3);
            a.byte(0xaa);
            a.mov_rr(R::RAX, R::R11)?;
            a.ret();
        }
        name if named(name, b"cppgm_builtin_memcmp") => {
            let next = a.label()?;
            let differ = a.label()?;
            let equal = a.label()?;
            a.bind(next);
            a.op(0, true, 0x85, 1, R::RDX.code(), Rm::Reg(R::RDX), false, 0)?;
            a.branch(Some(4), equal, true);
            a.load(R::RAX, Rm::mem(R::RDI, 0), 1, false)?;
            a.load(R::RCX, Rm::mem(R::RSI, 0), 1, false)?;
            a.alu_rr(7, R::RAX, Rm::Reg(R::RCX), 4)?;
            a.branch(Some(5), differ, true);
            a.alu_ri(0, Rm::Reg(R::RDI), 1, 8)?;
            a.alu_ri(0, Rm::Reg(R::RSI), 1, 8)?;
            a.alu_ri(5, Rm::Reg(R::RDX), 1, 8)?;
            a.branch(None, next, true);
            a.bind(differ);
            a.alu_rr(5, R::RAX, Rm::Reg(R::RCX), 4)?;
            a.ret();
            a.bind(equal);
            a.op(0, false, 0x33, 1, R::RAX.code(), Rm::Reg(R::RAX), false, 0)?;
            a.ret();
        }
        _ => return Ok(false),
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn c_string_builtin_names_emit_identical_runtime_code() -> Result<(), Error> {
        for builtin in ["strlen", "memcpy", "memmove", "memset", "memcmp"] {
            let name = format!("cppgm_builtin_{builtin}").into_bytes();
            let extended = [name.as_slice(), b"\0retained"].concat();
            let mut plain = Assembler::default();
            let mut terminated = Assembler::default();
            assert!(emit(&mut plain, &name)?);
            assert!(emit(&mut terminated, &extended)?);
            plain.resolve()?;
            terminated.resolve()?;
            assert_eq!(plain.code, terminated.code);
        }
        let mut unknown = Assembler::default();
        assert!(!emit(&mut unknown, b"unknown\0cppgm_builtin_strlen")?);
        assert!(unknown.code.is_empty());
        Ok(())
    }
}

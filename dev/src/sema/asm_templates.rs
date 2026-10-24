// Translated from sema/asm_statements.cpp. See NOTICE.
//! Lift only the frozen AT&T instruction set into typed semantic operations.
use super::assembly::{Binding, Instruction, Operand, Operation as Op};
use crate::lex::identifiers::Identifiers;

#[derive(Clone, Copy)]
enum Source {
    Operand(usize),
    Immediate(u64),
}

pub(super) fn lift(
    text: &[u8],
    operands: &[Operand],
    names: &Identifiers,
) -> Result<Vec<Instruction>, &'static str> {
    if operands.len() > 30 {
        return Err("unsupported GNU asm template: too many operands");
    }
    let mut lifter = Lifter {
        operands,
        names,
        locked: false,
        repeated: false,
        instructions: Vec::new(),
    };
    for line in text.split(|&byte| matches!(byte, b';' | b'\n')) {
        lifter.line(line)?;
    }
    if lifter.locked || lifter.repeated {
        return Err("unsupported GNU asm template: a prefix without its instruction");
    }
    Ok(lifter.instructions)
}

struct Lifter<'a> {
    operands: &'a [Operand],
    names: &'a Identifiers,
    locked: bool,
    repeated: bool,
    instructions: Vec<Instruction>,
}

fn spaces(text: &mut &[u8]) {
    while matches!(text.first(), Some(b' ' | b'\t')) {
        *text = &text[1..];
    }
}
fn word<'a>(text: &mut &'a [u8]) -> &'a [u8] {
    spaces(text);
    let end = text
        .iter()
        .position(|&c| matches!(c, b' ' | b'\t' | b','))
        .unwrap_or(text.len());
    let result = &text[..end];
    *text = &text[end..];
    result
}
fn rest_empty(mut text: &[u8]) -> Result<(), &'static str> {
    spaces(&mut text);
    if text.is_empty() {
        Ok(())
    } else {
        Err("unsupported GNU asm template: trailing text after an instruction")
    }
}
fn register_bytes(operand: &Operand) -> u64 {
    if operand.bytes == 16 {
        8
    } else {
        operand.bytes
    }
}

impl Lifter<'_> {
    fn line(&mut self, mut text: &[u8]) -> Result<(), &'static str> {
        let mut mnemonic = word(&mut text);
        while matches!(mnemonic, b"lock" | b"rep" | b"repe" | b"repz") {
            if mnemonic == b"lock" {
                self.locked = true;
            } else {
                self.repeated = true;
            }
            mnemonic = word(&mut text);
        }
        if mnemonic.is_empty() {
            return Ok(());
        }
        if matches!(mnemonic, b"nop" | b"pause") {
            if self.locked {
                return Err("unsupported GNU asm template: a locked nop");
            }
            self.repeated = false;
            return rest_empty(text);
        }
        if self.repeated {
            return Err("unsupported GNU asm template: a repeated instruction other than nop");
        }
        let (operation, count, mut bytes) = mnemonic_info(mnemonic)?;
        let mut parsed = [Source::Immediate(0); 2];
        for (index, operand) in parsed.iter_mut().enumerate().take(count) {
            if index != 0 {
                spaces(&mut text);
                if text.first() != Some(&b',') {
                    return Err("unsupported GNU asm template: too few instruction operands");
                }
                text = &text[1..];
            }
            *operand = self.operand(&mut text)?;
        }
        rest_empty(text)?;
        let mut instruction = Instruction {
            operation,
            locked: self.locked,
            destination: 0,
            source: None,
            bytes: 0,
            immediate: 0,
        };
        self.locked = false;
        if operation == Op::Fence {
            if instruction.locked {
                return Err("unsupported GNU asm template: a locked fence");
            }
            self.instructions.push(instruction);
            return Ok(());
        }
        if operation == Op::Exchange
            && let [Source::Operand(a), Source::Operand(b)] = parsed
            && self.operands[a].binding == Binding::Memory
            && self.operands[b].binding == Binding::Register
        {
            parsed.swap(0, 1);
        }
        let Source::Operand(destination) = parsed[count - 1] else {
            return Err(
                "unsupported GNU asm template: a destination that is no read-write operand",
            );
        };
        let target = &self.operands[destination];
        if !target.output || !target.read {
            return Err(
                "unsupported GNU asm template: a destination that is no read-write operand",
            );
        }
        if target.binding == Binding::Immediate {
            return Err("unsupported GNU asm template: an immediate destination");
        }
        instruction.destination = destination;
        let memory = target.binding == Binding::Memory;
        let source = (count == 2).then_some(parsed[0]);
        let source_register = match source {
            Some(Source::Operand(index)) if self.operands[index].binding != Binding::Immediate => {
                if self.operands[index].binding != Binding::Register {
                    return Err(
                        "unsupported GNU asm template: a source operand that is no register",
                    );
                }
                Some(index)
            }
            _ => None,
        };
        let register_size = if memory {
            source_register.map_or(0, |index| register_bytes(&self.operands[index]))
        } else {
            register_bytes(target)
        };
        if bytes == 0 && register_size == 0 {
            return Err(
                "unsupported GNU asm template: an instruction of no size (no suffix and no register operand)",
            );
        }
        if bytes == 0 {
            bytes = register_size;
        }
        if (register_size != 0 && bytes != register_size) || !matches!(bytes, 1 | 2 | 4 | 8) {
            return Err(
                "unsupported GNU asm template: an operand size that disagrees with its instruction",
            );
        }
        if source_register.is_some_and(|index| register_bytes(&self.operands[index]) != bytes) {
            return Err(
                "unsupported GNU asm template: a source operand that is no register of the instruction's size",
            );
        }
        if memory && bytes > target.bytes {
            return Err("unsupported GNU asm template: an operation wider than its memory operand");
        }
        instruction.bytes = bytes;
        if mnemonic.starts_with(b"inc") || mnemonic.starts_with(b"dec") {
            instruction.immediate = 1;
        }
        if let Some(source) = source {
            if let Some(index) = source_register {
                instruction.source = Some(index);
            } else {
                instruction.immediate = match source {
                    Source::Immediate(value) => value,
                    Source::Operand(index) => self.operands[index].value,
                };
                let signed = i64::from_le_bytes(instruction.immediate.to_le_bytes());
                if bytes == 8 && i32::try_from(signed).is_err() {
                    return Err(
                        "unsupported GNU asm template: an immediate beyond its instruction's immediate field",
                    );
                }
            }
        }
        if operation == Op::Exchange {
            if !instruction
                .source
                .is_some_and(|index| self.operands[index].output)
            {
                return Err(
                    "unsupported GNU asm template: an exchange of no two read-write operands",
                );
            }
            instruction.locked = memory;
        }
        if operation == Op::ByteSwap && (memory || !matches!(bytes, 4 | 8)) {
            return Err("unsupported GNU asm template: a byte swap of no 32- or 64-bit register");
        }
        if instruction.locked && !memory {
            return Err(
                "unsupported GNU asm template: a locked instruction of no memory destination",
            );
        }
        self.instructions.push(instruction);
        Ok(())
    }

    fn operand(&self, text: &mut &[u8]) -> Result<Source, &'static str> {
        spaces(text);
        if text.first() == Some(&b'$') {
            *text = &text[1..];
            let negative = text.first() == Some(&b'-');
            if negative {
                *text = &text[1..];
            }
            let base = if text.starts_with(b"0x") || text.starts_with(b"0X") {
                *text = &text[2..];
                16
            } else if text.starts_with(b"0b") || text.starts_with(b"0B") {
                *text = &text[2..];
                2
            } else if text.first() == Some(&b'0') && text.get(1).is_some_and(u8::is_ascii_digit) {
                *text = &text[1..];
                8
            } else {
                10
            };
            let start = text.len();
            let mut value = 0u64;
            while let Some(digit) = text.first().and_then(|&c| char::from(c).to_digit(16)) {
                if digit >= base {
                    return Err(
                        "unsupported GNU asm template: an immediate with a digit outside its base",
                    );
                }
                value = value
                    .checked_mul(u64::from(base))
                    .and_then(|n| n.checked_add(u64::from(digit)))
                    .ok_or("unsupported GNU asm template: an immediate beyond 64 bits")?;
                *text = &text[1..];
            }
            if text.len() == start {
                return Err("unsupported GNU asm template: an immediate without digits");
            }
            return Ok(Source::Immediate(if negative {
                value.wrapping_neg()
            } else {
                value
            }));
        }
        if text.first() != Some(&b'%') {
            return Err(
                "unsupported GNU asm template: an operand that is no asm operand or immediate",
            );
        }
        *text = &text[1..];
        let index = if text.first() == Some(&b'[') {
            let close = text
                .iter()
                .position(|&c| c == b']')
                .ok_or("unsupported GNU asm template: an unterminated operand name")?;
            let name = &text[1..close];
            *text = &text[close + 1..];
            self.operands
                .iter()
                .position(|op| op.name.is_some_and(|id| self.names.bytes(id) == name))
                .ok_or("unsupported GNU asm template: an operand name no operand has")?
        } else {
            if !text.first().is_some_and(u8::is_ascii_digit) {
                return Err("unsupported GNU asm template: a register or operand modifier");
            }
            let mut index = 0u32;
            while let Some(&digit) = text.first().filter(|c| c.is_ascii_digit()) {
                index = index.wrapping_mul(10).wrapping_add(u32::from(digit - b'0'));
                *text = &text[1..];
            }
            let index = usize::try_from(index).map_err(
                |_| "unsupported GNU asm template: an operand number beyond the operands",
            )?;
            if index >= self.operands.len() {
                return Err("unsupported GNU asm template: an operand number beyond the operands");
            }
            index
        };
        Ok(Source::Operand(self.operands[index].tied.unwrap_or(index)))
    }
}

fn mnemonic_info(mnemonic: &[u8]) -> Result<(Op, usize, u64), &'static str> {
    const MNEMONICS: &[(&[u8], Op, usize, bool)] = &[
        (b"mfence", Op::Fence, 0, false),
        (b"lfence", Op::Fence, 0, false),
        (b"sfence", Op::Fence, 0, false),
        (b"inc", Op::Add, 1, true),
        (b"dec", Op::Sub, 1, true),
        (b"not", Op::Not, 1, true),
        (b"neg", Op::Neg, 1, true),
        (b"add", Op::Add, 2, true),
        (b"sub", Op::Sub, 2, true),
        (b"and", Op::And, 2, true),
        (b"or", Op::Or, 2, true),
        (b"xor", Op::Xor, 2, true),
        (b"xchg", Op::Exchange, 2, true),
        (b"bswap", Op::ByteSwap, 1, true),
    ];
    for &(name, operation, count, sized) in MNEMONICS {
        if mnemonic == name {
            return Ok((operation, count, 0));
        }
        if sized && let Some(&[suffix]) = mnemonic.strip_prefix(name) {
            let bytes = match suffix {
                b'b' => 1,
                b'w' => 2,
                b'l' => 4,
                b'q' => 8,
                _ => continue,
            };
            return Ok((operation, count, bytes));
        }
    }
    Err("unsupported GNU asm template: an instruction outside the lifted set")
}

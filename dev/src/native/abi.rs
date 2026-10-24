//! Shared native widths and System V eightbyte classification. See NOTICE.
use super::x86::Register as R;
use crate::lowir::{
    Error,
    model::Signature,
    vocabulary::{EightbyteClass, ObjectClasses, Type},
};
// Allocatable caller-saved GPRs; r10/r11 are reserved emission scratch.
pub(super) const CALLER_SAVED: u32 =
    (1 << 0) | (1 << 1) | (1 << 2) | (1 << 6) | (1 << 7) | (1 << 8) | (1 << 9);

pub(super) fn value_bytes(ty: Type) -> Result<u32, Error> {
    Ok(match ty {
        Type::Void => 0,
        Type::I1 | Type::I8 | Type::U8 => 1,
        Type::I16 | Type::U16 => 2,
        Type::I32 | Type::U32 | Type::F32 => 4,
        Type::I64 | Type::F64 | Type::Ptr => 8,
        Type::I128 | Type::F80 => 16,
        Type::Object { bytes, .. } => {
            if bytes > 0x7fff_0000 {
                return Err(Error::Diagnostic(
                    "object larger than the 32-bit displacement range".into(),
                ));
            }
            u32::try_from(bytes).map_err(|_| Error::Limit)?
        }
    })
}

pub(super) fn value_align(ty: Type) -> Result<u32, Error> {
    if let Type::Object { align, .. } = ty {
        Ok(1_u32.wrapping_shl(u32::from(align)))
    } else {
        Ok(value_bytes(ty)?.max(1))
    }
}
pub(super) fn move_type(ty: Type) -> Type {
    if matches!(ty, Type::Object { .. } | Type::I128) {
        Type::I64
    } else {
        ty
    }
}
fn eightbyte_class(classes: Option<ObjectClasses>, index: u32) -> EightbyteClass {
    match (classes, index) {
        (Some(ObjectClasses::Direct(first, _)), 0) => first,
        (Some(ObjectClasses::Direct(_, second)), _) => second.unwrap_or(EightbyteClass::None),
        _ => EightbyteClass::Integer,
    }
}
#[derive(Clone, Copy)]
pub(super) struct Part {
    pub(super) register: Option<R>,
    pub(super) stack: i32,
    pub(super) offset: u32,
    pub(super) size: u32,
}
impl Part {
    pub(super) fn move_type(self) -> Type {
        if self.register.is_some_and(|r| r.code() >= 16) {
            if self.size > 8 {
                Type::I128
            } else if self.size <= 4 {
                Type::F32
            } else {
                Type::F64
            }
        } else {
            Type::I64
        }
    }
}
pub(super) struct Argument {
    first: usize,
    count: usize,
}
pub(super) struct Layout {
    pub(super) arguments: Vec<Argument>,
    parts: Vec<Part>,
    pub(super) indirect_result: bool,
    pub(super) result: [Option<Part>; 2],
    pub(super) classified_result: bool,
    pub(super) x87_result: bool,
    pub(super) stack_align: u32,
    gprs: usize,
    xmms: u8,
    pub(super) stack: u32,
}
impl Layout {
    pub(super) fn gpr_count(&self) -> usize {
        self.gprs
    }
    pub(super) fn vector_count(&self) -> u8 {
        self.xmms
    }
    pub(super) fn parts(&self, argument: &Argument) -> &[Part] {
        &self.parts[argument.first..argument.first + argument.count]
    }
    fn empty(signature: &Signature) -> Result<Self, Error> {
        if matches!(signature.result, Type::Object { .. })
            && signature.boundary.result_classes.is_some()
        {
            value_bytes(signature.result)?;
        }
        let indirect_result = matches!(signature.result,Type::Object{bytes,..} if bytes==0||bytes>16 || matches!(
            signature.boundary.result_classes,
            Some(ObjectClasses::Memory)
        ));
        let classified_result = matches!(signature.result, Type::Object { .. })
            && signature.boundary.result_classes.is_some();
        let x87_result = signature.result == Type::F80
            || !indirect_result
                && classified_result
                && matches!(signature.boundary.result_classes, Some(ObjectClasses::X87));
        let mut result: [Option<Part>; 2] = [None; 2];
        let mut result_count: usize = 0;
        if !indirect_result
            && !x87_result
            && matches!(signature.result, Type::Object { .. } | Type::I128)
        {
            let bytes = value_bytes(signature.result)?;
            let mut gprs = 0;
            let mut xmms = 0;
            for offset in (0..bytes).step_by(8) {
                let class = if classified_result {
                    eightbyte_class(signature.boundary.result_classes, offset / 8)
                } else {
                    EightbyteClass::Integer
                };
                let register = match class {
                    EightbyteClass::None => continue,
                    EightbyteClass::SseUp => {
                        let previous: &mut Option<Part> = result
                            .get_mut(result_count.checked_sub(1).ok_or(Error::Limit)?)
                            .ok_or(Error::Limit)?;
                        previous.as_mut().ok_or(Error::Limit)?.size = bytes;
                        continue;
                    }
                    EightbyteClass::Integer => {
                        let r = if gprs == 0 { R::RAX } else { R::RDX };
                        gprs += 1;
                        r
                    }
                    EightbyteClass::Sse => {
                        let r = R::xmm(xmms)?;
                        xmms += 1;
                        r
                    }
                };
                *result.get_mut(result_count).ok_or(Error::Limit)? = Some(Part {
                    register: Some(register),
                    stack: 0,
                    offset,
                    size: if classified_result {
                        (bytes - offset).min(8)
                    } else {
                        8
                    },
                });
                result_count += 1;
            }
        } else if signature.result != Type::Void
            && signature.result.float_rank() == 0
            && !indirect_result
            && !x87_result
        {
            result[0] = Some(Part {
                register: Some(R::RAX),
                stack: 0,
                offset: 0,
                size: 8,
            });
        }
        Ok(Self {
            arguments: Vec::new(),
            parts: Vec::new(),
            indirect_result,
            result,
            classified_result,
            x87_result,
            stack_align: 16,
            gprs: usize::from(indirect_result),
            xmms: 0,
            stack: 0,
        })
    }
    pub(super) fn classify(signature: &Signature) -> Result<Self, Error> {
        let mut layout = Self::empty(signature)?;
        for parameter in &signature.parameters {
            layout.argument(parameter.ty, parameter.classes)?;
        }
        Ok(layout)
    }
    pub(super) fn classify_call(
        signature: &Signature,
        types: &[Type],
        classes: &[(u32, ObjectClasses)],
    ) -> Result<Self, Error> {
        let mut layout = Self::empty(signature)?;
        for (index, &ty) in types.iter().enumerate() {
            let classes = if let Some(parameter) = signature.parameters.get(index) {
                parameter.classes
            } else {
                classes.iter().find_map(|&(argument, classes)| {
                    (argument as usize == index).then_some(classes)
                })
            };
            layout.argument(ty, classes)?;
        }
        Ok(layout)
    }
    fn stack(&mut self, bytes: u32, align: u32) {
        let align = align.max(8);
        self.stack_align = self.stack_align.max(align);
        self.stack = self.stack.wrapping_add(align - 1) & !(align - 1);
        self.parts.push(Part {
            register: None,
            stack: self.stack.cast_signed(),
            offset: 0,
            size: bytes,
        });
        self.stack = self.stack.wrapping_add(bytes.wrapping_add(7) & !7);
    }
    fn argument(&mut self, ty: Type, classes: Option<ObjectClasses>) -> Result<(), Error> {
        const GPRS: [R; 6] = [R::RDI, R::RSI, R::RDX, R::RCX, R::R8, R::R9];
        let first = self.parts.len();
        let bytes = value_bytes(ty)?;
        if matches!(ty, Type::Object { .. } | Type::I128) {
            let count = bytes.div_ceil(8);
            let (gprs, xmms) = if let Some(ObjectClasses::Direct(a, b)) = classes {
                let types = [Some(a), b];
                (
                    types
                        .iter()
                        .flatten()
                        .filter(|&&k| k == EightbyteClass::Integer)
                        .count(),
                    u8::try_from(
                        types
                            .iter()
                            .flatten()
                            .filter(|&&k| k == EightbyteClass::Sse)
                            .count(),
                    )
                    .map_err(|_| Error::Limit)?,
                )
            } else {
                (usize::try_from(count).map_err(|_| Error::Limit)?, 0)
            };
            if bytes > 16
                || bytes == 0 && classes.is_none()
                || matches!(classes, Some(ObjectClasses::Memory | ObjectClasses::X87))
                || self.gprs + gprs > 6
                || self.xmms + xmms > 8
            {
                self.stack(
                    bytes,
                    if ty == Type::I128 && classes.is_some() {
                        8
                    } else {
                        value_align(ty)?
                    },
                );
            } else {
                for k in 0..count {
                    let class = eightbyte_class(classes, k);
                    let reg = match class {
                        EightbyteClass::None => continue,
                        EightbyteClass::SseUp => {
                            self.parts.last_mut().ok_or(Error::Limit)?.size = bytes;
                            continue;
                        }
                        EightbyteClass::Integer => {
                            let reg = GPRS[self.gprs];
                            self.gprs += 1;
                            reg
                        }
                        EightbyteClass::Sse => {
                            let reg = R::xmm(self.xmms)?;
                            self.xmms += 1;
                            reg
                        }
                    };
                    self.parts.push(Part {
                        register: Some(reg),
                        stack: 0,
                        offset: k * 8,
                        size: (bytes - k * 8).min(8),
                    });
                }
            }
        } else if matches!(ty, Type::F32 | Type::F64) {
            if self.xmms < 8 {
                let reg = R::xmm(self.xmms)?;
                self.xmms += 1;
                self.parts.push(Part {
                    register: Some(reg),
                    stack: 0,
                    offset: 0,
                    size: bytes,
                });
            } else {
                self.stack(8, 8);
            }
        } else if ty == Type::F80 {
            self.stack(16, 16);
        } else if self.gprs < 6 {
            let reg = GPRS[self.gprs];
            self.gprs += 1;
            self.parts.push(Part {
                register: Some(reg),
                stack: 0,
                offset: 0,
                size: bytes,
            });
        } else {
            self.stack(8, 8);
        }
        self.arguments.push(Argument {
            first,
            count: self.parts.len() - first,
        });
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lowir::model::{Boundary, Parameter, Program};
    fn signature(program: &mut Program, types: &[Type], result: Type) -> Result<Signature, Error> {
        let name = program.names.intern("parameter")?;
        let parameters = types
            .iter()
            .map(|&ty| Parameter {
                name,
                ty,
                pass: None,
                no_alias: false,
                object_bytes: 0,
                classes: None,
            })
            .collect();
        Ok(Signature {
            parameters,
            result,
            boundary: Boundary::default(),
        })
    }
    #[test]
    fn explicit_memory_result_metadata_on_void_does_not_add_a_hidden_argument() -> Result<(), Error>
    {
        let mut program = Program::default();
        let mut sig = signature(&mut program, &[Type::Ptr], Type::Void)?;
        sig.boundary.result_classes = Some(ObjectClasses::Memory);
        let layout = Layout::classify(&sig)?;
        assert!(!layout.indirect_result);
        assert_eq!(layout.parts(&layout.arguments[0])[0].register, Some(R::RDI));
        Ok(())
    }
    #[test]
    fn mixed_result_eightbytes_keep_their_offsets_and_partial_widths() -> Result<(), Error> {
        let mut program = Program::default();
        let mut sig = signature(
            &mut program,
            &[],
            Type::Object {
                bytes: 12,
                align: 2,
            },
        )?;
        sig.boundary.result_classes = Some(ObjectClasses::Direct(
            EightbyteClass::Integer,
            Some(EightbyteClass::Sse),
        ));
        let layout = Layout::classify(&sig)?;
        let first = layout.result[0].ok_or(Error::Limit)?;
        let last = layout.result[1].ok_or(Error::Limit)?;
        assert_eq!(
            (first.register, first.offset, first.size),
            (Some(R::RAX), 0, 8)
        );
        assert_eq!(
            (last.register, last.offset, last.size),
            (Some(R::xmm(0)?), 8, 4)
        );
        assert_eq!(last.move_type(), Type::F32);
        Ok(())
    }
    #[test]
    fn an_object_that_cannot_fit_keeps_the_last_gpr_free() -> Result<(), Error> {
        let mut program = Program::default();
        let signature = signature(
            &mut program,
            &[
                Type::I64,
                Type::I64,
                Type::I64,
                Type::I64,
                Type::I64,
                Type::Object {
                    bytes: 16,
                    align: 4,
                },
                Type::I64,
            ],
            Type::Void,
        )?;
        let layout = Layout::classify(&signature)?;
        let [object] = layout.parts(&layout.arguments[5]) else {
            return Err(Error::Limit);
        };
        assert!(object.register.is_none());
        assert_eq!(object.stack, 0);
        assert_eq!(layout.parts(&layout.arguments[6])[0].register, Some(R::R9));
        Ok(())
    }
    #[test]
    fn unused_large_arguments_keep_target_stack_wrapping() -> Result<(), Error> {
        let mut program = Program::default();
        let object = Type::Object {
            bytes: 0x7fff_0000,
            align: 4,
        };
        let sig = signature(&mut program, &[object; 3], Type::Void)?;
        let layout = Layout::classify(&sig)?;
        let offsets: Vec<_> = layout
            .arguments
            .iter()
            .map(|arg| layout.parts(arg)[0].stack)
            .collect();
        assert_eq!(offsets, [0, 0x7fff_0000, -0x2_0000]);
        Ok(())
    }
    #[test]
    fn hidden_result_and_sse_exhaustion_preserve_argument_order() -> Result<(), Error> {
        let mut program = Program::default();
        let result = Type::Object {
            bytes: 32,
            align: 4,
        };
        let sig = signature(&mut program, &[Type::I64], result)?;
        let layout = Layout::classify(&sig)?;
        assert!(layout.indirect_result);
        assert_eq!(layout.parts(&layout.arguments[0])[0].register, Some(R::RSI));
        let mut types = vec![Type::F64; 8];
        types.extend([
            Type::Object {
                bytes: 16,
                align: 4,
            },
            Type::I64,
        ]);
        let mut sig = signature(&mut program, &types, Type::Void)?;
        sig.parameters[8].classes = Some(ObjectClasses::Direct(
            EightbyteClass::Integer,
            Some(EightbyteClass::Sse),
        ));
        let layout = Layout::classify(&sig)?;
        assert!(layout.parts(&layout.arguments[8])[0].register.is_none());
        assert_eq!(layout.parts(&layout.arguments[9])[0].register, Some(R::RDI));
        Ok(())
    }
}

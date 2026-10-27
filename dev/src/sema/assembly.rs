// Translated from sema/analyzer_asm.cpp and asm_statements.h. See NOTICE.
//! Statement-owned immutable lifting facts, published only after operand checking.
use super::{
    Error,
    analyzer::Analyzer,
    expression_facts::ValueCategory,
    graph::{Graph, ScopeId},
    specifiers::InlinePreference,
    types::{Scalar, TypeKind},
};
use crate::{
    lex::{PostTokenKind, identifiers::NameId},
    syntax::ast,
};
use std::collections::HashMap;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Binding {
    Memory,
    Register,
    Immediate,
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Operation {
    Fence,
    Add,
    Sub,
    And,
    Or,
    Xor,
    Not,
    Neg,
    Exchange,
    ByteSwap,
}
#[derive(Clone, Copy)]
pub(super) struct Operand {
    pub name: Option<NameId>,
    pub binding: Binding,
    pub output: bool,
    pub read: bool,
    pub bytes: u64,
    pub value: u64,
    pub tied: Option<usize>,
}
#[derive(Clone, Copy)]
pub(crate) struct Instruction {
    pub operation: Operation,
    pub locked: bool,
    pub destination: usize,
    pub source: Option<usize>,
    pub bytes: u64,
    pub immediate: u64,
}
#[derive(Clone)]
pub(crate) struct Program {
    pub instructions: Box<[Instruction]>,
    pub registers: Box<[bool]>,
    pub tied: Box<[Option<usize>]>,
    pub barrier: bool,
    pub unlifted: Option<&'static str>,
}
#[derive(Default)]
pub(super) struct Programs {
    values: HashMap<ast::StmtId, Program>,
}
impl Graph {
    pub(crate) fn lower_assembly(&self, id: ast::StmtId) -> Result<&Program, Error> {
        self.assembly.values.get(&id).ok_or(Error::Invariant(
            "an asm statement the analysis did not lift",
        ))
    }
}
impl Analyzer<'_> {
    fn asm_text(&self, literal: ast::Literal) -> Option<Vec<u8>> {
        let PostTokenKind::String {
            ty,
            elements,
            suffix: None,
        } = literal.kind
        else {
            return None;
        };
        if elements == 0 {
            return None;
        }
        // StringLiteralUnits: each code unit truncated to char, omit the terminal null.
        Some(
            self.literal_bytes
                .get(literal.data)
                .chunks_exact(ty.size())
                .take(usize::try_from(elements - 1).ok()?)
                .map(|unit| unit[0])
                .collect(),
        )
    }

    pub(super) fn asm_statement(
        &mut self,
        id: ast::StmtId,
        assembly: ast::AsmStatement,
        scope: ScopeId,
    ) -> Result<(), Error> {
        if self.body_context.is_none() || !self.semantic() || self.template_depth != 0 {
            for list in [assembly.outputs, assembly.inputs] {
                for index in 0..self.tree.asm_operands.get(list).len() {
                    self.expression(self.tree.asm_operands.get(list)[index].expression, scope)?;
                }
            }
            return Ok(());
        }
        let location = self.tree.statements.get(id).location;
        let text = self.asm_text(assembly.text).ok_or_else(|| {
            Error::at(
                location,
                "an asm template that is no ordinary string literal",
            )
        })?;
        let outputs = self.tree.asm_operands.get(assembly.outputs).len();
        let mut operands = Vec::<Operand>::new();
        for list in [assembly.outputs, assembly.inputs] {
            for index in 0..self.tree.asm_operands.get(list).len() {
                let operand = self.tree.asm_operands.get(list)[index];
                let output = operands.len() < outputs;
                let info = self.asm_operand(operand, output, outputs, &mut operands, scope)?;
                operands.push(info);
            }
        }
        let (instructions, unlifted) =
            match super::asm_templates::lift(&text, &operands, self.identifiers) {
                Ok(instructions) => (instructions, None),
                Err(message) => {
                    let function = self
                        .body_context
                        .as_ref()
                        .ok_or(Error::Invariant("asm body context"))?
                        .entity;
                    let entity = self.graph.entity(function);
                    if entity.inline != InlinePreference::Inline
                        && !entity.function.is_some_and(|f| {
                            f.internal_linkage && self.graph.lower_always_inline(function)
                        })
                    {
                        return Err(Error::at(location, message));
                    }
                    (Vec::new(), Some(message))
                }
            };
        let mut tied = vec![None; outputs];
        for (index, operand) in operands.iter().enumerate() {
            if let Some(output) = operand.tied {
                tied[output] = Some(index);
            }
        }
        let barrier = self
            .tree
            .asm_clobbers
            .get(assembly.clobbers)
            .iter()
            .any(|&literal| self.asm_text(literal).as_deref() == Some(b"memory"));
        self.graph.assembly.values.insert(
            id,
            Program {
                instructions: instructions.into_boxed_slice(),
                registers: operands
                    .iter()
                    .map(|operand| operand.binding == Binding::Register)
                    .collect(),
                tied: tied.into_boxed_slice(),
                barrier,
                unlifted,
            },
        );
        Ok(())
    }

    fn asm_operand(
        &mut self,
        operand: ast::AsmOperand,
        output: bool,
        outputs: usize,
        prior: &mut [Operand],
        scope: ScopeId,
    ) -> Result<Operand, Error> {
        let location = operand.location;
        let constraint = self.asm_text(operand.constraint).ok_or_else(|| {
            Error::at(
                location,
                "an asm constraint that is no ordinary string literal",
            )
        })?;
        if operand.name.is_some() && prior.iter().any(|info| info.name == operand.name) {
            return Err(Error::at(location, "a duplicate asm operand name"));
        }
        let mut info = Operand {
            name: operand.name,
            binding: Binding::Register,
            output,
            read: false,
            bytes: 0,
            value: 0,
            tied: None,
        };
        if !output
            && constraint
                .first()
                .is_some_and(|c| *c == b'[' || c.is_ascii_digit())
        {
            let tie = if constraint.iter().all(u8::is_ascii_digit) {
                // strtoul saturates at the host unsigned-long limit, then C++
                // explicitly narrows that result to the target uint32_t.
                let value = constraint.iter().fold(0u64, |n, &digit| {
                    n.saturating_mul(10).saturating_add(u64::from(digit - b'0'))
                });
                usize::try_from(value & u64::from(u32::MAX)).ok()
            } else if constraint.len() > 2
                && constraint.first() == Some(&b'[')
                && constraint.last() == Some(&b']')
            {
                prior[..outputs].iter().position(|info| {
                    info.name.is_some_and(|name| {
                        self.identifiers.bytes(name) == &constraint[1..constraint.len() - 1]
                    })
                })
            } else {
                None
            };
            let tie = tie.filter(|&tie| tie < outputs);
            if !tie.is_some_and(|tie| {
                prior[tie].binding == Binding::Register
                    && !prior[tie].read
                    && !prior[outputs..].iter().any(|info| info.tied == Some(tie))
            }) {
                return Err(Error::at(
                    location,
                    "an asm matching constraint of no `=` register output (or of one matched twice)",
                ));
            }
            info.tied = tie;
            if let Some(tie) = tie {
                prior[tie].read = true;
            }
        }
        let mut memory = false;
        let mut immediate = false;
        let mut any = false;
        for &byte in &constraint {
            match byte {
                b'+' => info.read = true,
                b'm' | b'o' | b'V' => memory = true,
                b'i' | b'n' => immediate = true,
                b'=' | b'&' | b'%' => (),
                _ => any = true,
            }
        }
        info.binding = if memory && !any && !immediate {
            Binding::Memory
        } else if immediate && !any && !memory {
            Binding::Immediate
        } else {
            Binding::Register
        };
        if output && !matches!(constraint.first(), Some(b'=' | b'+')) {
            return Err(Error::at(
                location,
                "an asm output whose constraint is no `=` or `+` one",
            ));
        }
        let facts = self.typed_expression(operand.expression, scope)?;
        if info.binding == Binding::Immediate {
            if output {
                return Err(Error::at(
                    location,
                    "an asm immediate operand that is no integral constant",
                ));
            }
            let value = self
                .constant_value(operand.expression, scope, false)?
                .filter(|value| self.integral_kind(value.ty).is_some())
                .ok_or_else(|| {
                    Error::at(
                        location,
                        "an asm immediate operand that is no integral constant",
                    )
                })?;
            info.value = value.bits;
            return Ok(info);
        }
        if (output || info.binding == Binding::Memory) && facts.category != ValueCategory::Lvalue {
            return Err(Error::at(
                location,
                "an asm memory or output operand that is no lvalue",
            ));
        }
        let ty = self.graph.types.get(facts.ty);
        if output
            && (ty.cv.0 & 1 != 0
                || matches!(ty.kind, TypeKind::Array { .. } | TypeKind::Function { .. }))
        {
            return Err(Error::at(
                location,
                "an asm output operand that is no modifiable lvalue",
            ));
        }
        let kind = ty.kind.clone();
        let ty = self.graph.types.intern(kind, super::types::Cv::default())?;
        // The frozen operand record holds a uint32_t object size.
        info.bytes = self.type_size(ty, location)?.0 & u64::from(u32::MAX);
        if info.binding == Binding::Register && !output {
            if !matches!(
                self.graph.types.get(ty).kind,
                TypeKind::Scalar(
                    Scalar::Bool
                        | Scalar::Nullptr
                        | Scalar::Basic(_)
                        | Scalar::Short
                        | Scalar::UnsignedShort
                        | Scalar::SignedChar
                        | Scalar::UnsignedChar
                ) | TypeKind::Enum(_)
                    | TypeKind::Pointer(_)
                    | TypeKind::MemberPointer { .. }
            ) {
                return Err(Error::at(
                    location,
                    "an asm register operand of no scalar type",
                ));
            }
            self.convert_operand_scalar(operand.expression, ty, false)?;
        }
        if info.tied.is_some_and(|tie| info.bytes != prior[tie].bytes) {
            return Err(Error::at(
                location,
                "an asm matching input of another size than its output",
            ));
        }
        Ok(info)
    }
}

#[cfg(test)]
mod tests {
    use super::Operation;
    use crate::{
        preprocess::unit::Preprocessor,
        sema::Analyzer,
        support::{source::SourceBuffer, stack::StackBudget},
        syntax::{
            SyntaxUnit,
            ast::{Body, DeclKind, StmtKind},
        },
    };

    #[test]
    fn lifting_keeps_source_operands_and_expression_storage() {
        let source = SourceBuffer::new(
            br#"int f(int x){asm("addl %1, %0" : "+r"(x) : "i"(2));return x;}"#.to_vec(),
        )
        .unwrap();
        let mut unit = SyntaxUnit::new(Preprocessor::with_name(&source, b"asm-ownership").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let DeclKind::Function { body, .. } = tree
            .declarations
            .get(tree.decl_lists.get(declarations)[0])
            .kind
        else {
            panic!("function");
        };
        let Body::Parsed(body) = *tree.bodies.get(body) else {
            panic!("parsed body");
        };
        let StmtKind::Compound(statements) = tree.statements.get(body.compound).kind else {
            panic!("compound");
        };
        let statement = tree.stmt_lists.get(statements)[0];
        let StmtKind::Asm(assembly) = tree.statements.get(statement).kind else {
            panic!("asm");
        };
        let assembly = *tree.assembly.get(assembly);
        let output = tree.asm_operands.get(assembly.outputs)[0].expression;
        let input = tree.asm_operands.get(assembly.inputs)[0].expression;
        let expressions = tree.expressions.mark();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let graph = analyzer.analyze(declarations).unwrap();
        assert_eq!(tree.expressions.mark(), expressions);
        assert_eq!(
            tree.asm_operands.get(assembly.outputs)[0].expression,
            output
        );
        assert_eq!(tree.asm_operands.get(assembly.inputs)[0].expression, input);
        let program = graph.lower_assembly(statement).unwrap();
        assert_eq!(&*program.registers, &[true, false]);
        assert!(program.instructions[0].operation == Operation::Add);
        assert_eq!(program.instructions[0].source, None);
        assert_eq!(program.instructions[0].immediate, 2);
    }
}

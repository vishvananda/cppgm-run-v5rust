//! Typed construction of runtime globals, signatures and routine bodies. See NOTICE.
use crate::lowir::{
    Error,
    builder::FunctionBuilder,
    literals::{Constant, FloatKind},
    model::{
        BlockId, Body, Boundary, Call, DataItem, Facts, Function, Global, GlobalInit,
        InstructionKind as I, Linkage, Metadata, ObjectAlias, Parameter, Program, Signature,
        SignatureId, SymbolDetail, SymbolId, Value,
    },
    vocabulary::{Binary, Binding, Compare, Convert, Storage, Type},
};
pub(super) struct RuntimeProgram<'a> {
    pub(super) program: &'a mut Program,
}
#[derive(Clone, Copy)]
pub(super) struct Word {
    pub(super) symbol: Option<SymbolId>,
    pub(super) value: i64,
}
impl RuntimeProgram<'_> {
    pub(super) fn reference(&mut self, name: &[u8]) -> Result<SymbolId, Error> {
        let name = self.program.names.intern_bytes(name)?;
        if let Some(symbol) = self.program.find_symbol(name) {
            return Ok(symbol);
        }
        let source = self.program.add_source(std::path::Path::new(""))?;
        self.program.reference(name, source, 0)
    }
    pub(super) fn defined(&mut self, name: &[u8]) -> Result<bool, Error> {
        let name = self.program.names.intern_bytes(name)?;
        Ok(self.program.find_symbol(name).is_some_and(|id| {
            self.program.function_defined(id)
                || self
                    .program
                    .global(self.program.symbol(id))
                    .is_some_and(|g| !matches!(g.init, GlobalInit::Declaration))
        }))
    }
    pub(super) fn signature(
        &mut self,
        result: Type,
        parameters: &[Type],
    ) -> Result<SignatureId, Error> {
        let name = self.program.names.intern("")?;
        self.program.add_signature(Signature {
            parameters: parameters
                .iter()
                .map(|&ty| Parameter {
                    name,
                    ty,
                    pass: None,
                    no_alias: false,
                    object_bytes: 0,
                    classes: None,
                })
                .collect(),
            result,
            boundary: Boundary::default(),
        })
    }
    pub(super) fn function(
        &mut self,
        name: &[u8],
        result: Type,
        parameters: &[Type],
        define: bool,
    ) -> Result<SymbolId, Error> {
        let signature = self.signature(result, parameters)?;
        let symbol = self.reference(name)?;
        if matches!(self.program.symbol(symbol).detail, SymbolDetail::Unresolved) {
            let name = self.program.symbol(symbol).name;
            self.program.add(
                symbol,
                Function {
                    name,
                    signature,
                    metadata: Metadata {
                        binding: Some(Binding::Strong),
                        linkage: Linkage::C,
                        ..Metadata::default()
                    },
                    body: define.then(|| Body::new(None, parameters.len())),
                    location: None,
                },
            )?;
        }
        Ok(symbol)
    }
    pub(super) fn global(&mut self, name: &[u8], bytes: u64) -> Result<SymbolId, Error> {
        let symbol = self.reference(name)?;
        if !matches!(self.program.symbol(symbol).detail, SymbolDetail::Unresolved) {
            return Ok(symbol);
        }
        let init = if bytes == 0 {
            GlobalInit::Declaration
        } else {
            let mut items = vec![DataItem::Constant(Type::I64, Constant::Integer(0))];
            if bytes > 8 {
                items.push(DataItem::Zero(bytes.wrapping_sub(8).cast_signed()));
            }
            GlobalInit::Data(items)
        };
        self.program.add_global(
            symbol,
            Global {
                name: self.program.symbol(symbol).name,
                ty: None,
                metadata: Metadata {
                    binding: Some(Binding::Strong),
                    ..Metadata::default()
                },
                init,
            },
        )?;
        Ok(symbol)
    }
    pub(super) fn words(&mut self, name: &[u8], items: &[Word]) -> Result<SymbolId, Error> {
        let symbol = self.reference(name)?;
        if !matches!(self.program.symbol(symbol).detail, SymbolDetail::Unresolved) {
            return Ok(symbol);
        }
        let items = items
            .iter()
            .map(|word| {
                word.symbol.map_or(
                    DataItem::Constant(
                        Type::I64,
                        Constant::Integer(i128::from(word.value).cast_unsigned()),
                    ),
                    |symbol| DataItem::Address(Type::Ptr, symbol, word.value),
                )
            })
            .collect();
        self.program.add_global(
            symbol,
            Global {
                name: self.program.symbol(symbol).name,
                ty: None,
                metadata: Metadata {
                    binding: Some(Binding::Strong),
                    storage: Some(Storage::ReadOnly),
                    ..Metadata::default()
                },
                init: GlobalInit::Data(items),
            },
        )?;
        Ok(symbol)
    }
    pub(super) fn string(&mut self, name: &[u8], text: &[u8]) -> Result<SymbolId, Error> {
        let symbol = self.reference(name)?;
        if !matches!(self.program.symbol(symbol).detail, SymbolDetail::Unresolved) {
            return Ok(symbol);
        }
        let items = text
            .iter()
            .copied()
            .chain(std::iter::once(0))
            .map(|b| DataItem::Constant(Type::I8, Constant::Integer(u128::from(b))))
            .collect();
        self.program.add_global(
            symbol,
            Global {
                name: self.program.symbol(symbol).name,
                ty: None,
                metadata: Metadata {
                    binding: Some(Binding::Strong),
                    storage: Some(Storage::ReadOnly),
                    ..Metadata::default()
                },
                init: GlobalInit::Data(items),
            },
        )?;
        Ok(symbol)
    }
    pub(super) fn alias(&mut self, name: &[u8], target: SymbolId) -> Result<(), Error> {
        let object = self.program.names.intern_bytes(name)?;
        let _ = self.program.add_alias(ObjectAlias { object, target });
        Ok(())
    }
    pub(super) fn routine(&mut self, symbol: SymbolId) -> Result<Routine<'_>, Error> {
        Routine::new(self.program, symbol)
    }
}
pub(super) struct Routine<'a> {
    builder: FunctionBuilder<'a>,
}
impl<'a> Routine<'a> {
    fn new(program: &'a mut Program, symbol: SymbolId) -> Result<Self, Error> {
        let mut out = Self {
            builder: FunctionBuilder::complete(program, symbol)?,
        };
        let block = out.block()?;
        out.enter(block)?;
        Ok(out)
    }
    pub(super) fn runtime(&mut self) -> RuntimeProgram<'_> {
        RuntimeProgram {
            program: self.builder.program(),
        }
    }
    pub(super) fn parameter(&self, index: usize) -> Result<Value, Error> {
        self.builder.parameter(index)
    }
    pub(super) fn integer(&mut self, value: i64) -> Result<Value, Error> {
        self.builder.integer(i128::from(value).cast_unsigned())
    }
    pub(super) fn floating(&mut self, kind: FloatKind, bits: u128) -> Result<Value, Error> {
        self.builder.constant(Constant::Float(kind, bits))
    }
    pub(super) fn at(&mut self, base: Value, offset: Value) -> Result<Value, Error> {
        self.builder
            .generated(I::Index(Type::I8, None, base, offset))
    }
    pub(super) fn at_offset(&mut self, base: Value, offset: i64) -> Result<Value, Error> {
        if offset == 0 {
            return Ok(base);
        }
        let offset = self.integer(offset)?;
        self.at(base, offset)
    }
    pub(super) fn load(&mut self, kind: Type, address: Value) -> Result<Value, Error> {
        self.builder
            .generated(I::Load(Facts::default(), kind, address))
    }
    pub(super) fn load_offset(
        &mut self,
        kind: Type,
        address: Value,
        offset: i64,
    ) -> Result<Value, Error> {
        let address = self.at_offset(address, offset)?;
        self.load(kind, address)
    }
    pub(super) fn store(&mut self, kind: Type, value: Value, address: Value) -> Result<(), Error> {
        self.builder
            .effect(I::Store(Facts::default(), kind, value, address))
    }
    pub(super) fn store_offset(
        &mut self,
        kind: Type,
        value: Value,
        address: Value,
        offset: i64,
    ) -> Result<(), Error> {
        let address = self.at_offset(address, offset)?;
        self.store(kind, value, address)
    }
    pub(super) fn address(&mut self, symbol: SymbolId) -> Result<Value, Error> {
        self.builder.generated(I::Addr(Value::Symbol(symbol)))
    }
    pub(super) fn slot_address(&mut self, slot: Value) -> Result<Value, Error> {
        self.builder.generated(I::Addr(slot))
    }
    pub(super) fn binary(
        &mut self,
        op: Binary,
        kind: Type,
        a: Value,
        b: Value,
    ) -> Result<Value, Error> {
        self.builder
            .generated(I::Binary(op, Facts::default(), kind, a, b))
    }
    pub(super) fn add(&mut self, kind: Type, a: Value, b: Value) -> Result<Value, Error> {
        self.binary(Binary::Add, kind, a, b)
    }
    pub(super) fn compare(
        &mut self,
        predicate: Compare,
        kind: Type,
        a: Value,
        b: Value,
    ) -> Result<Value, Error> {
        self.builder.generated(I::Compare(predicate, kind, a, b))
    }
    pub(super) fn equal(&mut self, kind: Type, a: Value, b: Value) -> Result<Value, Error> {
        self.compare(Compare::Eq, kind, a, b)
    }
    pub(super) fn convert(
        &mut self,
        op: Convert,
        to: Type,
        from: Type,
        value: Value,
    ) -> Result<Value, Error> {
        self.builder.generated(I::Convert(op, to, from, value))
    }
    pub(super) fn call(
        &mut self,
        result: Type,
        callee: SymbolId,
        args: &[Value],
    ) -> Result<Value, Error> {
        self.call_indirect(result, Value::Symbol(callee), None, args)
    }
    pub(super) fn call_indirect(
        &mut self,
        result: Type,
        callee: Value,
        signature: Option<SignatureId>,
        args: &[Value],
    ) -> Result<Value, Error> {
        let arguments = self.builder.arguments(args)?;
        self.builder.generated(I::Call(Call {
            result,
            callee,
            arguments,
            copy_elision: false,
            argument_classes: (Vec::new()).into(),
            signature,
        }))
    }
    pub(super) fn call_void(&mut self, callee: SymbolId, args: &[Value]) -> Result<(), Error> {
        self.call_indirect_void(Value::Symbol(callee), None, args)
    }
    pub(super) fn call_indirect_void(
        &mut self,
        callee: Value,
        signature: Option<SignatureId>,
        args: &[Value],
    ) -> Result<(), Error> {
        let arguments = self.builder.arguments(args)?;
        self.builder.effect(I::Call(Call {
            result: Type::Void,
            callee,
            arguments,
            copy_elision: false,
            argument_classes: (Vec::new()).into(),
            signature,
        }))
    }
    pub(super) fn local(&mut self, kind: Type) -> Result<Value, Error> {
        self.builder.anonymous_slot(kind)
    }
    pub(super) fn block(&mut self) -> Result<BlockId, Error> {
        self.builder.anonymous_block()
    }
    pub(super) fn enter(&mut self, block: BlockId) -> Result<(), Error> {
        self.builder.place_block(block)
    }
    pub(super) fn jump(&mut self, block: BlockId) -> Result<(), Error> {
        self.builder.effect(I::Jump(block))
    }
    pub(super) fn branch(
        &mut self,
        condition: Value,
        yes: BlockId,
        no: BlockId,
    ) -> Result<(), Error> {
        self.builder.effect(I::Branch(condition, yes, no))
    }
    pub(super) fn ret(&mut self, kind: Type, value: Value) -> Result<(), Error> {
        self.builder.effect(I::Return(kind, Some(value)))
    }
    pub(super) fn ret_void(&mut self) -> Result<(), Error> {
        self.builder.effect(I::Return(Type::Void, None))
    }
    pub(super) fn unreachable(&mut self) -> Result<(), Error> {
        self.builder.effect(I::Unreachable)
    }
    pub(super) fn finish(self) -> Result<(), Error> {
        self.builder.finish()
    }
}

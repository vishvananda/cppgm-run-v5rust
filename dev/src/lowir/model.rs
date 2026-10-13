//! One unit of typed `LowIR`, with private ID stores and function-owned bodies. See NOTICE.
use super::{
    Error,
    vocabulary::{Binary, Binding, Compare, Convert, Effects, Pass, Projection, Role, Type, Unary},
};
use std::collections::HashMap;
use std::sync::Arc;
macro_rules! identity {
    ($($name:ident),+ ) => { $(
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub(super) struct $name(u32);
    )+ };
}
identity!(
    NameId,
    TempId,
    SlotId,
    BlockId,
    InstId,
    FunctionId,
    LiteralId,
    SymbolId,
    SignatureId,
    SourceId
);
#[derive(Default)]
pub(super) struct Names {
    text: Vec<Arc<str>>,
    index: HashMap<Arc<str>, NameId>,
}
impl Names {
    pub(super) fn intern(&mut self, text: &str) -> Result<NameId, Error> {
        if let Some(&id) = self.index.get(text) {
            return Ok(id);
        }
        let id = NameId(u32::try_from(self.text.len()).map_err(|_| Error::Limit)?);
        let text: Arc<str> = Arc::from(text);
        self.text.push(Arc::clone(&text));
        self.index.insert(text, id);
        Ok(id)
    }
    pub(super) fn get(&self, id: NameId) -> &str {
        &self.text[id.0 as usize]
    }
}
#[derive(Default)]
pub(super) struct Program {
    pub(super) names: Names,
    functions: Vec<Function>,
    symbols: Vec<Symbol>,
    index: HashMap<NameId, SymbolId>,
    signatures: Vec<Signature>,
    sources: Vec<String>,
}
impl Program {
    pub(super) fn add(&mut self, symbol: SymbolId, function: Function) -> Result<(), Error> {
        let id = FunctionId(u32::try_from(self.functions.len()).map_err(|_| Error::Limit)?);
        self.symbols[symbol.0 as usize].function = Some(id);
        self.functions.push(function);
        Ok(())
    }
    pub(super) fn reference(
        &mut self,
        name: NameId,
        source: SourceId,
        line: u32,
    ) -> Result<SymbolId, Error> {
        if let Some(&id) = self.index.get(&name) {
            return Ok(id);
        }
        let id = SymbolId(u32::try_from(self.symbols.len()).map_err(|_| Error::Limit)?);
        self.symbols.push(Symbol {
            name,
            function: None,
            source,
            line,
        });
        self.index.insert(name, id);
        Ok(id)
    }
    pub(super) fn add_source(&mut self, path: &str) -> Result<SourceId, Error> {
        let id = SourceId(u32::try_from(self.sources.len()).map_err(|_| Error::Limit)?);
        self.sources.push(path.into());
        Ok(id)
    }
    pub(super) fn symbol(&self, id: SymbolId) -> &Symbol {
        &self.symbols[id.0 as usize]
    }
    pub(super) fn finish_symbols(&self) -> Result<(), Error> {
        for symbol in &self.symbols {
            if symbol.function.is_none() {
                return Err(Error::Diagnostic(format!(
                    "undefined top-level symbol: {} at {}:{}",
                    self.names.get(symbol.name),
                    self.sources[symbol.source.0 as usize],
                    symbol.line
                )));
            }
        }
        Ok(())
    }
    pub(super) fn add_signature(&mut self, signature: Signature) -> Result<SignatureId, Error> {
        let id = SignatureId(u32::try_from(self.signatures.len()).map_err(|_| Error::Limit)?);
        self.signatures.push(signature);
        Ok(id)
    }
    pub(super) fn signature(&self, id: SignatureId) -> &Signature {
        &self.signatures[id.0 as usize]
    }
    pub(super) fn call_boundary(&self, call: &Call) -> Option<&Signature> {
        if let Some(id) = call.signature {
            return Some(self.signature(id));
        }
        self.callee_function(call.callee)
            .map(|f| self.signature(f.signature))
    }
    pub(super) fn callee_function(&self, value: Value) -> Option<&Function> {
        if let Value::Symbol(id) = value {
            return self
                .symbol(id)
                .function
                .map(|id| &self.functions[id.0 as usize]);
        }
        None
    }
    pub(super) fn may_end_block(&self, kind: &InstructionKind) -> bool {
        kind.terminator()
            || matches!(kind, InstructionKind::Call(call) if self.call_boundary(call).is_some_and(|s| s.boundary.returns == Returns::NoReturn))
    }
    pub(super) fn functions(&self) -> &[Function] {
        &self.functions
    }
    pub(super) fn functions_in_symbol_order(&self) -> impl Iterator<Item = &Function> {
        self.symbols
            .iter()
            .filter_map(|symbol| symbol.function.map(|id| &self.functions[id.0 as usize]))
    }
    pub(super) fn legacy_roles(&mut self) {
        if self
            .functions
            .iter()
            .any(|f| f.metadata.role == Some(Role::Entry))
        {
            return;
        }
        for (name, role) in [
            ("main", Role::Entry),
            ("__cppgm_init", Role::Init),
            ("__cppgm_fini", Role::Fini),
        ] {
            if self.functions.iter().any(|f| f.metadata.role == Some(role)) {
                continue;
            }
            if let Some(f) = self.functions.iter_mut().find(|f| {
                f.body.is_some() && f.metadata.role.is_none() && self.names.get(f.name) == name
            }) {
                f.metadata.role = Some(role);
            }
        }
    }
}
pub(super) struct Symbol {
    pub(super) name: NameId,
    pub(super) function: Option<FunctionId>,
    source: SourceId,
    line: u32,
}
pub(super) struct Signature {
    pub(super) parameters: Vec<Parameter>,
    pub(super) result: Type,
    pub(super) boundary: Boundary,
}
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum Arity {
    #[default]
    Fixed,
    Variadic,
}
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum Unwind {
    #[default]
    Default,
    No,
}
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum Returns {
    #[default]
    Default,
    NoReturn,
}
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum Linkage {
    #[default]
    Default,
    C,
}
#[derive(Default)]
pub(super) struct Boundary {
    pub(super) arity: Arity,
    pub(super) effects: Option<Effects>,
    pub(super) unwind: Unwind,
    pub(super) returns: Returns,
    pub(super) result_classes: Option<super::vocabulary::ObjectClasses>,
}
#[derive(Default)]
pub(super) struct Metadata {
    pub(super) role: Option<Role>,
    pub(super) linkage: Linkage,
    pub(super) binding: Option<Binding>,
    pub(super) flags: u8,
    pub(super) object: Option<NameId>,
}
pub(super) struct Parameter {
    pub(super) name: NameId,
    pub(super) ty: Type,
    pub(super) pass: Option<Pass>,
    pub(super) no_alias: bool,
    pub(super) object_bytes: u64,
    pub(super) classes: Option<super::vocabulary::ObjectClasses>,
}
pub(super) struct Function {
    pub(super) name: NameId,
    pub(super) signature: SignatureId,
    pub(super) metadata: Metadata,
    pub(super) body: Option<Body>,
}
#[derive(Clone, Copy, Debug)]
pub(super) enum Value {
    Temp(TempId),
    Slot(SlotId),
    Integer(LiteralId),
    Symbol(SymbolId),
}
pub(super) struct Temp {
    pub(super) name: NameId,
    pub(super) ty: Option<Type>,
    pub(super) definition: Option<InstId>,
    pub(super) first_use: u32,
    pub(super) uses: u32,
}
pub(super) struct Slot {
    pub(super) name: NameId,
    pub(super) ty: Type,
}
pub(super) struct Block {
    pub(super) name: NameId,
    pub(super) placed: bool,
    pub(super) first_use: u32,
    pub(super) instructions: Vec<InstId>,
}
#[derive(Default)]
pub(super) struct Body {
    literals: Vec<u128>,
    operands: Vec<Value>,
    temps: Vec<Temp>,
    slots: Vec<Slot>,
    blocks: Vec<Block>,
    instructions: Vec<Instruction>,
    temp_index: HashMap<NameId, TempId>,
    slot_index: HashMap<NameId, SlotId>,
    block_index: HashMap<NameId, BlockId>,
    pub(super) layout: Vec<BlockId>,
    pub(super) parameter_count: usize,
    pub(super) source: String,
}
impl Body {
    pub(super) fn new(source: &str, parameter_count: usize) -> Self {
        Self {
            source: source.into(),
            parameter_count,
            ..Self::default()
        }
    }
    pub(super) fn temp(&self, id: TempId) -> &Temp {
        &self.temps[id.0 as usize]
    }
    pub(super) fn temp_mut(&mut self, id: TempId) -> &mut Temp {
        &mut self.temps[id.0 as usize]
    }
    pub(super) fn slot(&self, id: SlotId) -> &Slot {
        &self.slots[id.0 as usize]
    }
    pub(super) fn block(&self, id: BlockId) -> &Block {
        &self.blocks[id.0 as usize]
    }
    pub(super) fn block_mut(&mut self, id: BlockId) -> &mut Block {
        &mut self.blocks[id.0 as usize]
    }
    pub(super) fn instruction(&self, id: InstId) -> &Instruction {
        &self.instructions[id.0 as usize]
    }
    pub(super) fn integer(&mut self, bits: u128) -> Result<Value, Error> {
        let id = LiteralId(u32::try_from(self.literals.len()).map_err(|_| Error::Limit)?);
        self.literals.push(bits);
        Ok(Value::Integer(id))
    }
    pub(super) fn literal(&self, id: LiteralId) -> u128 {
        self.literals[id.0 as usize]
    }
    pub(super) fn add_arguments(&mut self, arguments: &[Value]) -> Result<Arguments, Error> {
        let first = u32::try_from(self.operands.len()).map_err(|_| Error::Limit)?;
        let count = u32::try_from(arguments.len()).map_err(|_| Error::Limit)?;
        let _ = first.checked_add(count).ok_or(Error::Limit)?;
        self.operands.extend_from_slice(arguments);
        Ok(Arguments { first, count })
    }
    pub(super) fn arguments(&self, arguments: Arguments) -> &[Value] {
        arguments.slice(&self.operands)
    }
    pub(super) fn visit_values(&self, kind: &InstructionKind, visit: impl FnMut(Value)) {
        kind.visit_values(&self.operands, visit);
    }
    pub(super) fn temps(&self) -> &[Temp] {
        &self.temps
    }
    pub(super) fn slots(&self) -> &[Slot] {
        &self.slots
    }
    pub(super) fn blocks(&self) -> &[Block] {
        &self.blocks
    }
    pub(super) fn instructions(&self) -> &[Instruction] {
        &self.instructions
    }
    pub(super) fn temp_is_parameter(&self, id: TempId) -> bool {
        (id.0 as usize) < self.parameter_count
    }
    pub(super) fn use_temp(&mut self, name: NameId, line: u32) -> Result<TempId, Error> {
        if let Some(&id) = self.temp_index.get(&name) {
            return Ok(id);
        }
        let id = TempId(u32::try_from(self.temps.len()).map_err(|_| Error::Limit)?);
        self.temps.push(Temp {
            name,
            ty: None,
            definition: None,
            first_use: line,
            uses: 0,
        });
        self.temp_index.insert(name, id);
        Ok(id)
    }
    pub(super) fn use_block(&mut self, name: NameId, line: u32) -> Result<BlockId, Error> {
        if let Some(&id) = self.block_index.get(&name) {
            return Ok(id);
        }
        let id = BlockId(u32::try_from(self.blocks.len()).map_err(|_| Error::Limit)?);
        self.blocks.push(Block {
            name,
            placed: false,
            first_use: line,
            instructions: Vec::new(),
        });
        self.block_index.insert(name, id);
        Ok(id)
    }
    pub(super) fn find_slot(&self, name: NameId) -> Option<SlotId> {
        self.slot_index.get(&name).copied()
    }
    pub(super) fn find_temp(&self, name: NameId) -> Option<TempId> {
        self.temp_index.get(&name).copied()
    }
    pub(super) fn add_slot(&mut self, name: NameId, ty: Type) -> Result<(), Error> {
        let id = SlotId(u32::try_from(self.slots.len()).map_err(|_| Error::Limit)?);
        self.slots.push(Slot { name, ty });
        self.slot_index.insert(name, id);
        Ok(())
    }
    pub(super) fn append(
        &mut self,
        block: BlockId,
        instruction: Instruction,
    ) -> Result<InstId, Error> {
        let id = InstId(u32::try_from(self.instructions.len()).map_err(|_| Error::Limit)?);
        let mut overflow = false;
        let temps = &mut self.temps;
        instruction.kind.visit_values(&self.operands, |value| {
            if let Value::Temp(temp) = value {
                if let Some(count) = temps[temp.0 as usize].uses.checked_add(1) {
                    temps[temp.0 as usize].uses = count;
                } else {
                    overflow = true;
                }
            }
        });
        if overflow {
            return Err(Error::Limit);
        }
        if let Some(result) = instruction.result {
            self.temp_mut(result).definition = Some(id);
        }
        self.instructions.push(instruction);
        self.block_mut(block).instructions.push(id);
        Ok(id)
    }
}
pub(super) struct Instruction {
    pub(super) result: Option<TempId>,
    pub(super) kind: InstructionKind,
    pub(super) line: u32,
}
pub(super) enum InstructionKind {
    Const(Type, Value),
    Copy(Type, Value),
    Unary(Unary, Type, Value),
    Binary(Binary, Facts, Type, Value, Value),
    Compare(Compare, Type, Value, Value),
    Convert(Convert, Type, Type, Value),
    Select(Type, Value, Value, Value),
    Phi(Type, Vec<(BlockId, Value)>),
    Addr(Value),
    Load(Facts, Type, Value),
    Store(Facts, Type, Value, Value),
    Index(Type, Option<Projection>, Value, Value),
    CopyObject(u64, u8, Value, Value),
    ZeroInit(u64, u8, Value),
    Call(Call),
    Jump(BlockId),
    Branch(Value, BlockId, BlockId),
    Switch(Value, BlockId, Vec<(Value, BlockId)>),
    Return(Type, Option<Value>),
    Unreachable,
}
pub(super) struct Call {
    pub(super) result: Type,
    pub(super) callee: Value,
    pub(super) arguments: Arguments,
    pub(super) copy_elision: bool,
    pub(super) signature: Option<SignatureId>,
}
/// A checked slice in the owning function's mutable extra-operand store.
#[derive(Clone, Copy)]
pub(super) struct Arguments {
    first: u32,
    count: u32,
}
impl Arguments {
    fn slice(self, operands: &[Value]) -> &[Value] {
        &operands[self.first as usize..(self.first + self.count) as usize]
    }
}
#[derive(Clone, Copy, Default)]
pub(super) struct Facts(u8);
impl Facts {
    pub(super) fn arithmetic(nowrap: bool, exact: bool) -> Self {
        Self((u8::from(nowrap) * 32) | (u8::from(exact) * 16))
    }
    pub(super) fn memory(volatile: bool, typed: bool) -> Self {
        Self(u8::from(volatile) | (u8::from(typed) * 64))
    }
    pub(super) fn volatile(self) -> bool {
        self.0 & 1 != 0
    }
    pub(super) fn typed(self) -> bool {
        self.0 & 64 != 0
    }
    pub(super) fn nowrap(self) -> bool {
        self.0 & 32 != 0
    }
    pub(super) fn exact(self) -> bool {
        self.0 & 16 != 0
    }
}
impl InstructionKind {
    pub(super) fn terminator(&self) -> bool {
        matches!(
            self,
            Self::Jump(_)
                | Self::Branch(..)
                | Self::Switch(..)
                | Self::Return(..)
                | Self::Unreachable
        )
    }
    pub(super) fn result_type(&self) -> Type {
        match self {
            Self::Const(t, _)
            | Self::Copy(t, _)
            | Self::Unary(_, t, _)
            | Self::Binary(_, _, t, _, _)
            | Self::Convert(_, t, _, _)
            | Self::Select(t, ..)
            | Self::Phi(t, _)
            | Self::Load(_, t, _) => *t,
            Self::Compare(..) => Type::I64,
            Self::Addr(_) | Self::Index(..) => Type::Ptr,
            Self::Call(call) => call.result,
            _ => Type::Void,
        }
    }
    fn visit_values(&self, operands: &[Value], mut visit: impl FnMut(Value)) {
        match self {
            Self::Const(_, v)
            | Self::Copy(_, v)
            | Self::Unary(_, _, v)
            | Self::Convert(_, _, _, v)
            | Self::Addr(v)
            | Self::Load(_, _, v)
            | Self::ZeroInit(_, _, v)
            | Self::Branch(v, ..)
            | Self::Return(_, Some(v)) => visit(*v),
            Self::Binary(_, _, _, a, b)
            | Self::Compare(_, _, a, b)
            | Self::Store(_, _, a, b)
            | Self::Index(_, _, a, b)
            | Self::CopyObject(_, _, a, b) => {
                visit(*a);
                visit(*b);
            }
            Self::Select(_, a, b, c) => {
                visit(*a);
                visit(*b);
                visit(*c);
            }
            Self::Phi(_, pairs) => {
                for &(_, v) in pairs {
                    visit(v);
                }
            }
            Self::Switch(v, _, pairs) => {
                visit(*v);
                for &(v, _) in pairs {
                    visit(v);
                }
            }
            Self::Call(call) => {
                visit(call.callee);
                for &value in call.arguments.slice(operands) {
                    visit(value);
                }
            }
            _ => {}
        }
    }
    pub(super) fn visit_targets(&self, mut visit: impl FnMut(BlockId)) {
        match self {
            Self::Jump(b) => visit(*b),
            Self::Branch(_, a, b) => {
                visit(*a);
                visit(*b);
            }
            Self::Switch(_, b, pairs) => {
                visit(*b);
                for &(_, b) in pairs {
                    visit(b);
                }
            }
            _ => {}
        }
    }
}
impl InstId {
    pub(super) fn index(self) -> usize {
        self.0 as usize
    }
}
impl BlockId {
    pub(super) fn index(self) -> usize {
        self.0 as usize
    }
}

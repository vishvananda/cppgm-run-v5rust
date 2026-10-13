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
    NameId, TempId, SlotId, BlockId, InstId, FunctionId, LiteralId
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
    index: HashMap<NameId, FunctionId>,
}
impl Program {
    pub(super) fn add(&mut self, function: Function) -> Result<(), Error> {
        let id = FunctionId(u32::try_from(self.functions.len()).map_err(|_| Error::Limit)?);
        self.index.insert(function.name, id);
        self.functions.push(function);
        Ok(())
    }
    pub(super) fn contains(&self, name: NameId) -> bool {
        self.index.contains_key(&name)
    }
    pub(super) fn functions(&self) -> &[Function] {
        &self.functions
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
pub(super) struct Metadata {
    pub(super) arity: Arity,
    pub(super) effects: Option<Effects>,
    pub(super) unwind: Unwind,
    pub(super) returns: Returns,
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
}
pub(super) struct Function {
    pub(super) name: NameId,
    pub(super) parameters: Vec<Parameter>,
    pub(super) result: Type,
    pub(super) metadata: Metadata,
    pub(super) body: Option<Body>,
}
#[derive(Clone, Copy, Debug)]
pub(super) enum Value {
    Temp(TempId),
    Slot(SlotId),
    Integer(LiteralId),
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
        instruction.kind.visit_values(|value| {
            if let Value::Temp(temp) = value {
                if let Some(count) = self.temp(temp).uses.checked_add(1) {
                    self.temp_mut(temp).uses = count;
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
    Jump(BlockId),
    Branch(Value, BlockId, BlockId),
    Switch(Value, BlockId, Vec<(Value, BlockId)>),
    Return(Type, Option<Value>),
    Unreachable,
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
            _ => Type::Void,
        }
    }
    pub(super) fn visit_values(&self, mut visit: impl FnMut(Value)) {
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

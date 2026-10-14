//! One unit of typed `LowIR`, with private ID stores and function-owned bodies. See NOTICE.
use super::literals::{Constant, FloatKind};
use super::{
    Error,
    vocabulary::{
        AtomicUpdate, Binary, Binding, Compare, Convert, Effects, Fence, MemoryOrder, Pass,
        Projection, Role, Storage, Type, Unary,
    },
};
use std::collections::{HashMap, HashSet};
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use std::sync::Arc;
macro_rules! identity {
    ($($name:ident),+ ) => { $(
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub(crate) struct $name(u32);
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
    SourceId,
    GlobalId
);
impl TempId {
    pub(crate) fn index(self) -> usize {
        self.0 as usize
    }
}
pub(crate) struct Names {
    text: Vec<Arc<[u8]>>,
    index: HashMap<Arc<[u8]>, NameId>,
}
impl Default for Names {
    fn default() -> Self {
        let empty: Arc<[u8]> = Arc::from(&b""[..]);
        Self {
            text: vec![Arc::clone(&empty)],
            index: HashMap::from([(empty, NameId(0))]),
        }
    }
}
impl Names {
    pub(crate) fn intern(&mut self, text: &str) -> Result<NameId, Error> {
        self.intern_bytes(text.as_bytes())
    }
    pub(crate) fn intern_bytes(&mut self, text: &[u8]) -> Result<NameId, Error> {
        if let Some(&id) = self.index.get(text) {
            return Ok(id);
        }
        let id = NameId(u32::try_from(self.text.len()).map_err(|_| Error::Limit)?);
        let text: Arc<[u8]> = Arc::from(text);
        self.text.push(Arc::clone(&text));
        self.index.insert(text, id);
        Ok(id)
    }
    pub(crate) fn get(&self, id: NameId) -> &str {
        // Grammar names are ASCII; only debug file tokens use the byte view.
        std::str::from_utf8(self.bytes(id)).unwrap_or("")
    }
    pub(crate) fn len(&self) -> usize {
        self.text.len()
    }
    pub(crate) fn storage_bytes(&self) -> usize {
        self.text.iter().map(|text| text.len()).sum()
    }
    pub(crate) fn bytes(&self, id: NameId) -> &[u8] {
        &self.text[id.0 as usize]
    }
}
#[derive(Default)]
pub(crate) struct Program {
    pub(crate) names: Names,
    functions: Vec<Function>,
    streamed_functions: HashSet<SymbolId>,
    symbols: Vec<Symbol>,
    index: HashMap<NameId, SymbolId>,
    signatures: Vec<Signature>,
    sources: Vec<PathBuf>,
    globals: Vec<Global>,
    aliases: Vec<ObjectAlias>,
    alias_names: HashSet<NameId>,
    reader_vocabulary: bool,
}
impl Program {
    pub(crate) fn prepare_reader(&mut self) -> Result<(), Error> {
        if !self.reader_vocabulary {
            super::vocabulary::intern_reader_words(&mut self.names)?;
            self.reader_vocabulary = true;
        }
        Ok(())
    }
    pub(crate) fn statistics(&self) -> super::Statistics {
        let mut stats = super::Statistics {
            symbols: self.symbols.len(),
            functions: self.functions.len(),
            globals: self.globals.len(),
            signatures: self.signatures.len(),
            names: self.names.len(),
            name_arena_bytes: self.names.storage_bytes(),
            ..super::Statistics::default()
        };
        for global in &self.globals {
            if let GlobalInit::Data(items) = &global.init {
                stats.data_items += items.len();
            }
        }
        for body in self
            .functions
            .iter()
            .filter_map(|function| function.body.as_ref())
        {
            stats.blocks += body.blocks.len();
            stats.instructions += body.instructions.len();
            stats.temps += body.temps.len();
            stats.literals += body.literals.len();
            stats.extra_operands += body.operands.len();
            // The oracle stores each predecessor/value or value/target pair
            // in its extra-operand vector; our typed pairs have the same census.
            for instruction in &body.instructions {
                stats.extra_operands += match &instruction.kind {
                    InstructionKind::Phi(_, pairs) => pairs.len() * 2,
                    InstructionKind::Switch(_, _, pairs) => pairs.len() * 2,
                    _ => 0,
                };
            }
        }
        stats
    }
    pub(crate) fn add(&mut self, symbol: SymbolId, function: Function) -> Result<(), Error> {
        let id = FunctionId(u32::try_from(self.functions.len()).map_err(|_| Error::Limit)?);
        self.symbols[symbol.0 as usize].detail = SymbolDetail::Function(id);
        self.functions.push(function);
        Ok(())
    }
    pub(crate) fn find_symbol(&self, name: NameId) -> Option<SymbolId> {
        self.index.get(&name).copied()
    }
    pub(crate) fn reference(
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
            detail: SymbolDetail::Unresolved,
            source,
            line,
        });
        self.index.insert(name, id);
        Ok(id)
    }
    pub(crate) fn add_source(&mut self, path: &Path) -> Result<SourceId, Error> {
        let id = SourceId(u32::try_from(self.sources.len()).map_err(|_| Error::Limit)?);
        self.sources.push(path.into());
        Ok(id)
    }
    pub(crate) fn source(&self, id: SourceId) -> &Path {
        &self.sources[id.0 as usize]
    }
    pub(crate) fn symbol(&self, id: SymbolId) -> &Symbol {
        &self.symbols[id.0 as usize]
    }
    pub(crate) fn finish_symbols(&self) -> Result<(), Error> {
        for symbol in &self.symbols {
            if matches!(symbol.detail, SymbolDetail::Unresolved) {
                return Err(Error::at_path(
                    format!(
                        "undefined top-level symbol: {}",
                        self.names.get(symbol.name)
                    ),
                    &self.sources[symbol.source.0 as usize],
                    symbol.line,
                ));
            }
        }
        Ok(())
    }
    pub(crate) fn add_signature(&mut self, signature: Signature) -> Result<SignatureId, Error> {
        let id = SignatureId(u32::try_from(self.signatures.len()).map_err(|_| Error::Limit)?);
        self.signatures.push(signature);
        Ok(id)
    }
    pub(crate) fn signature_mut(&mut self, id: SignatureId) -> &mut Signature {
        &mut self.signatures[id.0 as usize]
    }
    pub(crate) fn signature(&self, id: SignatureId) -> &Signature {
        &self.signatures[id.0 as usize]
    }
    pub(crate) fn call_boundary(&self, call: &Call) -> Option<&Signature> {
        if let Some(id) = call.signature {
            return Some(self.signature(id));
        }
        self.callee_function(call.callee)
            .map(|f| self.signature(f.signature))
    }
    pub(crate) fn callee_function(&self, value: Value) -> Option<&Function> {
        if let Value::Symbol(id) = value
            && let SymbolDetail::Function(id) = self.symbol(id).detail
        {
            return Some(&self.functions[id.0 as usize]);
        }
        None
    }
    pub(crate) fn may_end_block(&self, kind: &InstructionKind) -> bool {
        kind.terminator()
            || matches!(kind, InstructionKind::Call(call) if self.call_boundary(call).is_some_and(|s| s.boundary.returns == Returns::NoReturn))
    }
    pub(crate) fn stream_last_definition(
        &mut self,
        output: &mut impl std::io::Write,
    ) -> Result<(), Error> {
        let function = self
            .functions
            .last()
            .ok_or(Error::Construction("missing completed definition".into()))?;
        super::writer::write_definition(self, function, output).map_err(Error::Output)?;
        if let Some(symbol) = self.find_symbol(function.name) {
            self.streamed_functions.insert(symbol);
        }
        if let Some(function) = self.functions.last_mut() {
            function.body = None;
        }
        Ok(())
    }
    pub(crate) fn function_defined(&self, symbol: SymbolId) -> bool {
        self.streamed_functions.contains(&symbol)
            || self
                .function(self.symbol(symbol))
                .is_some_and(|f| f.body.is_some())
    }
    pub(crate) fn function_declarations(&self) -> impl Iterator<Item = &Function> {
        self.symbols
            .iter()
            .enumerate()
            .filter_map(|(index, symbol)| {
                let SymbolDetail::Function(id) = symbol.detail else {
                    return None;
                };
                let symbol_id = SymbolId(u32::try_from(index).ok()?);
                let function = &self.functions[id.0 as usize];
                (function.body.is_none() && !self.streamed_functions.contains(&symbol_id))
                    .then_some(function)
            })
    }
    pub(crate) fn functions(&self) -> &[Function] {
        &self.functions
    }
    pub(crate) fn functions_in_symbol_order(&self) -> impl Iterator<Item = &Function> {
        self.symbols
            .iter()
            .filter_map(|symbol| match symbol.detail {
                SymbolDetail::Function(id) => Some(&self.functions[id.0 as usize]),
                _ => None,
            })
    }
    pub(crate) fn add_global(&mut self, symbol: SymbolId, global: Global) -> Result<(), Error> {
        let id = GlobalId(u32::try_from(self.globals.len()).map_err(|_| Error::Limit)?);
        self.symbols[symbol.0 as usize].detail = SymbolDetail::Global(id);
        self.globals.push(global);
        Ok(())
    }
    pub(crate) fn globals(&self) -> &[Global] {
        &self.globals
    }
    pub(crate) fn globals_in_publication_order(&self) -> impl Iterator<Item = &Global> {
        self.globals.iter().enumerate().filter_map(|(index, global)| {
            let symbol = self.find_symbol(global.name)?;
            matches!(self.symbol(symbol).detail, SymbolDetail::Global(id) if usize::try_from(id.0).ok() == Some(index)).then_some(global)
        })
    }
    pub(crate) fn symbols(&self) -> &[Symbol] {
        &self.symbols
    }
    pub(crate) fn symbol_metadata(&self, symbol: &Symbol) -> Option<&Metadata> {
        match symbol.detail {
            SymbolDetail::Function(id) => Some(&self.functions[id.0 as usize].metadata),
            SymbolDetail::Global(id) => Some(&self.globals[id.0 as usize].metadata),
            SymbolDetail::Unresolved => None,
        }
    }
    pub(crate) fn function(&self, symbol: &Symbol) -> Option<&Function> {
        if let SymbolDetail::Function(id) = symbol.detail {
            Some(&self.functions[id.0 as usize])
        } else {
            None
        }
    }
    pub(crate) fn global(&self, symbol: &Symbol) -> Option<&Global> {
        if let SymbolDetail::Global(id) = symbol.detail {
            Some(&self.globals[id.0 as usize])
        } else {
            None
        }
    }
    pub(crate) fn add_alias(&mut self, alias: ObjectAlias) -> bool {
        if !self.alias_names.insert(alias.object) {
            return false;
        }
        self.aliases.push(alias);
        true
    }
    pub(crate) fn has_alias(&self, name: NameId) -> bool {
        self.alias_names.contains(&name)
    }
    pub(crate) fn aliases(&self) -> &[ObjectAlias] {
        &self.aliases
    }
    pub(crate) fn legacy_roles(&mut self) {
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
pub(crate) struct Symbol {
    pub(crate) name: NameId,
    pub(crate) detail: SymbolDetail,
    source: SourceId,
    line: u32,
}
#[derive(Clone, Copy)]
pub(crate) enum SymbolDetail {
    Unresolved,
    Function(FunctionId),
    Global(GlobalId),
}
impl Symbol {
    pub(crate) fn declared(&self) -> bool {
        !matches!(self.detail, SymbolDetail::Unresolved)
    }
}
pub(crate) struct Global {
    pub(crate) name: NameId,
    pub(crate) ty: Option<Type>,
    pub(crate) metadata: Metadata,
    pub(crate) init: GlobalInit,
}
pub(crate) enum GlobalInit {
    Declaration,
    Zero,
    Constant(Constant),
    Address(SymbolId, i64),
    Data(Vec<DataItem>),
}
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum DataItem {
    Zero(i64),
    Constant(Type, Constant),
    Address(Type, SymbolId, i64),
}
pub(crate) struct ObjectAlias {
    pub(crate) object: NameId,
    pub(crate) target: SymbolId,
}
pub(crate) struct Signature {
    pub(crate) parameters: Vec<Parameter>,
    pub(crate) result: Type,
    pub(crate) boundary: Boundary,
}
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum Arity {
    #[default]
    Fixed,
    Variadic,
}
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum Unwind {
    #[default]
    Default,
    No,
}
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum Returns {
    #[default]
    Default,
    NoReturn,
}
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum Linkage {
    #[default]
    Default,
    C,
}
#[derive(Default)]
pub(crate) struct Boundary {
    pub(crate) arity: Arity,
    pub(crate) effects: Option<Effects>,
    pub(crate) unwind: Unwind,
    pub(crate) returns: Returns,
    pub(crate) result_classes: Option<super::vocabulary::ObjectClasses>,
}
#[derive(Clone, Copy, Default)]
pub(crate) struct Metadata {
    pub(crate) role: Option<Role>,
    pub(crate) linkage: Linkage,
    pub(crate) binding: Option<Binding>,
    pub(crate) flags: u8,
    pub(crate) object: Option<NameId>,
    pub(crate) storage: Option<Storage>,
    pub(crate) section: Option<NameId>,
    pub(crate) tls_for: Option<SymbolId>,
}
pub(crate) struct Parameter {
    pub(crate) name: NameId,
    pub(crate) ty: Type,
    pub(crate) pass: Option<Pass>,
    pub(crate) no_alias: bool,
    pub(crate) object_bytes: u64,
    pub(crate) classes: Option<super::vocabulary::ObjectClasses>,
}
pub(crate) struct Function {
    pub(crate) name: NameId,
    pub(crate) signature: SignatureId,
    pub(crate) metadata: Metadata,
    pub(crate) body: Option<Body>,
    pub(crate) location: Option<DebugLocation>,
}
#[derive(Clone, Copy)]
pub(crate) struct DebugLocation {
    pub(crate) file: NameId,
    pub(crate) line: u32,
    pub(crate) column: u32,
}
/// One-based body-local identity: absence needs no extra instruction word.
#[derive(Clone, Copy)]
pub(crate) struct LocationId(NonZeroU32);
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Value {
    Temp(TempId),
    Slot(SlotId),
    Integer(LiteralId),
    Float(LiteralId, FloatKind),
    Symbol(SymbolId),
}
pub(crate) struct Temp {
    pub(crate) generated: bool,
    pub(crate) name: NameId,
    pub(crate) ty: Option<Type>,
    pub(crate) definition: Option<InstId>,
    pub(crate) first_use: u32,
    pub(crate) uses: u32,
}
pub(crate) struct Slot {
    pub(crate) name: NameId,
    pub(crate) ty: Type,
}
pub(crate) struct Block {
    pub(crate) name: NameId,
    pub(crate) placed: bool,
    pub(crate) first_use: u32,
    pub(crate) instructions: Vec<InstId>,
}
#[derive(Default)]
pub(crate) struct Body {
    literals: Vec<u128>,
    operands: Vec<Value>,
    temps: Vec<Temp>,
    slots: Vec<Slot>,
    blocks: Vec<Block>,
    instructions: Vec<Instruction>,
    locations: Vec<DebugLocation>,
    temp_index: HashMap<NameId, TempId>,
    slot_index: HashMap<NameId, SlotId>,
    block_index: HashMap<NameId, BlockId>,
    pub(crate) layout: Vec<BlockId>,
    pub(crate) parameter_count: usize,
    pub(crate) source: Option<SourceId>,
}
impl Body {
    pub(crate) fn new(source: Option<SourceId>, parameter_count: usize) -> Self {
        Self {
            parameter_count,
            source,
            ..Self::default()
        }
    }
    pub(crate) fn temp(&self, id: TempId) -> &Temp {
        &self.temps[id.0 as usize]
    }
    pub(crate) fn temp_mut(&mut self, id: TempId) -> &mut Temp {
        &mut self.temps[id.0 as usize]
    }
    pub(crate) fn slot(&self, id: SlotId) -> &Slot {
        &self.slots[id.0 as usize]
    }
    pub(crate) fn block(&self, id: BlockId) -> &Block {
        &self.blocks[id.0 as usize]
    }
    pub(crate) fn block_mut(&mut self, id: BlockId) -> &mut Block {
        &mut self.blocks[id.0 as usize]
    }
    pub(crate) fn instruction(&self, id: InstId) -> &Instruction {
        &self.instructions[id.0 as usize]
    }
    pub(crate) fn set_location(
        &mut self,
        inst: InstId,
        location: DebugLocation,
    ) -> Result<(), Error> {
        let next = self.locations.len().checked_add(1).ok_or(Error::Limit)?;
        let next = u32::try_from(next).map_err(|_| Error::Limit)?;
        let id = LocationId(NonZeroU32::new(next).ok_or(Error::Limit)?);
        let instruction = self
            .instructions
            .get_mut(inst.index())
            .ok_or(Error::Limit)?;
        self.locations.push(location);
        instruction.location = Some(id);
        Ok(())
    }
    pub(crate) fn location(&self, inst: &Instruction) -> Option<DebugLocation> {
        inst.location
            .map(|id| self.locations[(id.0.get() - 1) as usize])
    }
    pub(crate) fn integer(&mut self, bits: u128) -> Result<Value, Error> {
        let id = LiteralId(u32::try_from(self.literals.len()).map_err(|_| Error::Limit)?);
        self.literals.push(bits);
        Ok(Value::Integer(id))
    }
    pub(crate) fn constant(&mut self, constant: Constant) -> Result<Value, Error> {
        let (bits, kind) = match constant {
            Constant::Integer(bits) => (bits, None),
            Constant::Float(kind, bits) => (bits, Some(kind)),
        };
        let Value::Integer(id) = self.integer(bits)? else {
            return Err(Error::Limit);
        };
        Ok(kind.map_or(Value::Integer(id), |kind| Value::Float(id, kind)))
    }
    pub(crate) fn literal(&self, id: LiteralId) -> u128 {
        self.literals[id.0 as usize]
    }
    pub(crate) fn add_arguments(&mut self, arguments: &[Value]) -> Result<Arguments, Error> {
        let first = u32::try_from(self.operands.len()).map_err(|_| Error::Limit)?;
        let count = u32::try_from(arguments.len()).map_err(|_| Error::Limit)?;
        let _ = first.checked_add(count).ok_or(Error::Limit)?;
        self.operands.extend_from_slice(arguments);
        Ok(Arguments { first, count })
    }
    pub(crate) fn arguments(&self, arguments: Arguments) -> &[Value] {
        arguments.slice(&self.operands)
    }
    pub(crate) fn visit_values(&self, kind: &InstructionKind, visit: impl FnMut(Value)) {
        kind.visit_values(&self.operands, visit);
    }
    pub(crate) fn temps(&self) -> &[Temp] {
        &self.temps
    }
    pub(crate) fn slots(&self) -> &[Slot] {
        &self.slots
    }
    pub(crate) fn blocks(&self) -> &[Block] {
        &self.blocks
    }
    pub(crate) fn instructions(&self) -> &[Instruction] {
        &self.instructions
    }
    pub(crate) fn temp_is_parameter(&self, id: TempId) -> bool {
        (id.0 as usize) < self.parameter_count
    }
    pub(crate) fn generated_temp(&mut self, ty: Type) -> Result<TempId, Error> {
        let id = TempId(u32::try_from(self.temps.len()).map_err(|_| Error::Limit)?);
        self.temps.push(Temp {
            name: NameId(0),
            generated: true,
            ty: Some(ty),
            definition: None,
            first_use: 0,
            uses: 0,
        });
        Ok(id)
    }
    pub(crate) fn use_temp(&mut self, name: NameId, line: u32) -> Result<TempId, Error> {
        if let Some(&id) = self.temp_index.get(&name) {
            return Ok(id);
        }
        let id = TempId(u32::try_from(self.temps.len()).map_err(|_| Error::Limit)?);
        self.temps.push(Temp {
            generated: false,
            name,
            ty: None,
            definition: None,
            first_use: line,
            uses: 0,
        });
        self.temp_index.insert(name, id);
        Ok(id)
    }
    pub(crate) fn use_block(&mut self, name: NameId, line: u32) -> Result<BlockId, Error> {
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
    pub(crate) fn find_slot(&self, name: NameId) -> Option<SlotId> {
        self.slot_index.get(&name).copied()
    }
    pub(crate) fn find_temp(&self, name: NameId) -> Option<TempId> {
        self.temp_index.get(&name).copied()
    }
    pub(crate) fn add_slot(&mut self, name: NameId, ty: Type) -> Result<(), Error> {
        let id = SlotId(u32::try_from(self.slots.len()).map_err(|_| Error::Limit)?);
        self.slots.push(Slot { name, ty });
        self.slot_index.insert(name, id);
        Ok(())
    }
    pub(crate) fn constructed_slot(&mut self, ty: Type) -> Result<Value, Error> {
        let id = SlotId(u32::try_from(self.slots.len()).map_err(|_| Error::Limit)?);
        self.slots.push(Slot {
            name: NameId(0),
            ty,
        });
        Ok(Value::Slot(id))
    }
    pub(crate) fn name_constructed_slots(&mut self, names: &[NameId]) -> Result<(), Error> {
        if self.slots.len() != names.len() {
            return Err(Error::Construction(
                "slot names do not match storage".into(),
            ));
        }
        self.slot_index.clear();
        for (index, (slot, &name)) in self.slots.iter_mut().zip(names).enumerate() {
            slot.name = name;
            let id = SlotId(u32::try_from(index).map_err(|_| Error::Limit)?);
            if self.slot_index.insert(name, id).is_some() {
                return Err(Error::Construction(
                    "duplicate constructed slot name".into(),
                ));
            }
        }
        Ok(())
    }
    pub(crate) fn append(
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
pub(crate) struct Instruction {
    pub(crate) result: Option<TempId>,
    pub(crate) kind: InstructionKind,
    pub(crate) line: u32,
    location: Option<LocationId>,
}
impl Instruction {
    pub(crate) fn new(result: Option<TempId>, kind: InstructionKind, line: u32) -> Self {
        Self {
            result,
            kind,
            line,
            location: None,
        }
    }
}
pub(crate) enum InstructionKind {
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
    Runtime(RuntimeInstruction),
    Jump(BlockId),
    Branch(Value, BlockId, BlockId),
    Switch(Value, BlockId, Vec<(Value, BlockId)>),
    Return(Type, Option<Value>),
    Unreachable,
}
/// Runtime facts only: later consumers implement their machine/runtime effects.
pub(crate) enum RuntimeInstruction {
    AtomicLoad(Type, Value, MemoryOrder),
    AtomicStore(Type, Value, Value, MemoryOrder),
    AtomicUpdate(AtomicUpdate, Type, Value, Value, MemoryOrder),
    AtomicCompareExchange {
        ty: Type,
        address: Value,
        expected: Value,
        desired: Value,
        success: MemoryOrder,
        failure: MemoryOrder,
    },
    Fence(Fence, MemoryOrder),
    VaStart(Value),
    VaArg(Type, Value),
    StackAlloc(Value),
    Exception(Type),
    ExceptionSelector(Type),
    EhTry(BlockId),
    EhCleanup(Option<BlockId>),
    EhCatch(SymbolId, Option<i64>),
    EhFilter(Arguments, Option<i64>),
    EhCatchAll(Option<i64>),
    EhEnd,
    Throw(Type, Value),
    Resume,
}
impl RuntimeInstruction {
    /// Handler references are distinct from ordinary control-flow successors.
    pub(crate) fn handler_target(&self) -> Option<BlockId> {
        match self {
            Self::EhTry(block) | Self::EhCleanup(Some(block)) => Some(*block),
            _ => None,
        }
    }
    fn result_type(&self) -> Type {
        match self {
            Self::AtomicLoad(ty, ..)
            | Self::AtomicUpdate(_, ty, ..)
            | Self::VaArg(ty, _)
            | Self::Exception(ty) => *ty,
            Self::AtomicCompareExchange { .. } | Self::ExceptionSelector(_) => Type::I64,
            Self::StackAlloc(_) => Type::Ptr,
            _ => Type::Void,
        }
    }
    fn visit_values(&self, operands: &[Value], mut visit: impl FnMut(Value)) {
        match self {
            Self::AtomicLoad(_, v, _)
            | Self::VaStart(v)
            | Self::VaArg(_, v)
            | Self::StackAlloc(v)
            | Self::Throw(_, v) => visit(*v),
            Self::AtomicStore(_, a, b, _) | Self::AtomicUpdate(_, _, a, b, _) => {
                visit(*a);
                visit(*b);
            }
            Self::AtomicCompareExchange {
                address,
                expected,
                desired,
                ..
            } => {
                visit(*address);
                visit(*expected);
                visit(*desired);
            }
            Self::EhCatch(symbol, _) => visit(Value::Symbol(*symbol)),
            Self::EhFilter(arguments, _) => {
                for &value in arguments.slice(operands) {
                    visit(value);
                }
            }
            _ => {}
        }
    }
}
pub(crate) struct Call {
    pub(crate) result: Type,
    pub(crate) callee: Value,
    pub(crate) arguments: Arguments,
    pub(crate) copy_elision: bool,
    pub(crate) signature: Option<SignatureId>,
}
/// A checked slice in the owning function's mutable extra-operand store.
#[derive(Clone, Copy)]
pub(crate) struct Arguments {
    first: u32,
    count: u32,
}
impl Arguments {
    fn slice(self, operands: &[Value]) -> &[Value] {
        &operands[self.first as usize..(self.first + self.count) as usize]
    }
}
#[derive(Clone, Copy, Default)]
pub(crate) struct Facts(u8);
impl Facts {
    pub(crate) fn arithmetic(nowrap: bool, exact: bool) -> Self {
        Self((u8::from(nowrap) * 32) | (u8::from(exact) * 16))
    }
    pub(crate) fn memory(volatile: bool, typed: bool) -> Self {
        Self(u8::from(volatile) | (u8::from(typed) * 64))
    }
    pub(crate) fn volatile(self) -> bool {
        self.0 & 1 != 0
    }
    pub(crate) fn typed(self) -> bool {
        self.0 & 64 != 0
    }
    pub(crate) fn nowrap(self) -> bool {
        self.0 & 32 != 0
    }
    pub(crate) fn exact(self) -> bool {
        self.0 & 16 != 0
    }
}
impl InstructionKind {
    pub(crate) fn terminator(&self) -> bool {
        matches!(
            self,
            Self::Jump(_)
                | Self::Branch(..)
                | Self::Switch(..)
                | Self::Return(..)
                | Self::Unreachable
                | Self::Runtime(RuntimeInstruction::Throw(..) | RuntimeInstruction::Resume)
        )
    }
    pub(crate) fn result_type(&self) -> Type {
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
            Self::Runtime(inst) => inst.result_type(),
            _ => Type::Void,
        }
    }
    fn visit_values(&self, operands: &[Value], mut visit: impl FnMut(Value)) {
        match self {
            Self::Runtime(inst) => inst.visit_values(operands, visit),
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
    pub(crate) fn visit_targets(&self, mut visit: impl FnMut(BlockId)) {
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
    pub(crate) fn index(self) -> usize {
        self.0 as usize
    }
}
impl BlockId {
    pub(crate) fn index(self) -> usize {
        self.0 as usize
    }
}

impl SymbolId {
    pub(crate) fn index(self) -> usize {
        self.0 as usize
    }
}

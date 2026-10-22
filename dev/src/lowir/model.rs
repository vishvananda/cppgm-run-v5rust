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
#[path = "body_edits.rs"]
mod body_edits;
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
impl SlotId {
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
/// A consumer owns only its phase's encoded state, never front-end facts.
pub(crate) trait DefinitionSink: std::any::Any {
    fn append(&mut self, program: &Program, symbol: SymbolId, held: bool) -> Result<(), Error>;
    fn release_held(&mut self);
    fn into_any(self: Box<Self>) -> Box<dyn std::any::Any>;
}
#[derive(Default)]
pub(crate) struct Program {
    definition_sink: Option<Box<dyn DefinitionSink>>,
    retained_definitions: Option<RetainedDefinitions>,
    forward_boundaries: Vec<Option<(SignatureId, Metadata)>>,
    forward_globals: Vec<Option<ForwardGlobal>>,
    immediate_declarations: Vec<SymbolId>,
    immediate_declaration_set: std::collections::HashSet<SymbolId>,
    pub(crate) names: Names,
    functions: Vec<Function>,
    completed_definition: Option<SymbolId>,
    publications: Vec<SymbolId>,
    streamed_functions: HashSet<SymbolId>,
    optional_references: HashSet<SymbolId>,
    symbols: Vec<Symbol>,
    index: HashMap<NameId, SymbolId>,
    signatures: Vec<Signature>,
    sources: Vec<PathBuf>,
    globals: Vec<Global>,
    aliases: Vec<ObjectAlias>,
    alias_names: HashSet<NameId>,
    reader_vocabulary: bool,
}
#[derive(Default)]
struct RetainedDefinitions {
    completed: Vec<SymbolId>,
    held: Vec<SymbolId>,
}
impl Program {
    pub(crate) fn retain_definitions(&mut self) -> Result<(), Error> {
        if self.definition_sink.is_some() || self.retained_definitions.is_some() {
            return Err(Error::Construction(
                "definition delivery already configured".into(),
            ));
        }
        self.retained_definitions = Some(RetainedDefinitions::default());
        Ok(())
    }
    pub(crate) fn finish_retained_definitions(&mut self, startup: [Option<SymbolId>; 2]) {
        if let Some(mut retained) = self.retained_definitions.take() {
            retained.completed.append(&mut retained.held);
            retained.completed.extend(startup.into_iter().flatten());
            self.order_retained_definitions(&retained.completed);
        }
    }
    pub(crate) fn set_forward_global(&mut self, symbol: SymbolId, facts: ForwardGlobal) {
        self.forward_globals.resize(self.symbols.len(), None);
        if self.forward_globals[symbol.index()].is_none() {
            self.forward_globals[symbol.index()] = Some(facts);
        }
    }
    pub(crate) fn forward_global(&self, symbol: SymbolId) -> Option<ForwardGlobal> {
        self.forward_globals.get(symbol.index()).copied().flatten()
    }
    pub(crate) fn set_definition_sink(
        &mut self,
        sink: Box<dyn DefinitionSink>,
    ) -> Result<(), Error> {
        if self.definition_sink.is_some() || self.retained_definitions.is_some() {
            return Err(Error::Construction(
                "definition sink already installed".into(),
            ));
        }
        self.definition_sink = Some(sink);
        Ok(())
    }
    pub(crate) fn take_definition_sink(&mut self) -> Option<Box<dyn DefinitionSink>> {
        self.definition_sink.take()
    }
    pub(crate) fn has_definition_sink(&self) -> bool {
        self.definition_sink.is_some()
    }
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
            stats.instructions += body
                .blocks
                .iter()
                .map(|block| block.instructions.len())
                .sum::<usize>();
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
        if function.body.is_some() {
            self.completed_definition = Some(symbol);
        }
        self.symbols[symbol.0 as usize].detail = SymbolDetail::Function(id);
        self.functions.push(function);
        self.publications.push(symbol);
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
    pub(crate) fn set_forward_boundary(
        &mut self,
        symbol: SymbolId,
        signature: SignatureId,
        metadata: Metadata,
    ) {
        self.forward_boundaries.resize(self.symbols.len(), None);
        self.forward_boundaries[symbol.index()] = Some((signature, metadata));
    }
    pub(crate) fn forward_boundary(&self, symbol: SymbolId) -> Option<SignatureId> {
        self.forward_boundaries
            .get(symbol.index())
            .copied()
            .flatten()
            .map(|(signature, _)| signature)
    }
    pub(crate) fn callee_metadata(&self, value: Value) -> Option<&Metadata> {
        if let Some(function) = self.callee_function(value) {
            return Some(&function.metadata);
        }
        let Value::Symbol(symbol) = value else {
            return None;
        };
        self.forward_boundaries
            .get(symbol.index())
            .and_then(Option::as_ref)
            .map(|(_, metadata)| metadata)
    }
    pub(crate) fn call_boundary(&self, call: &Call) -> Option<&Signature> {
        if let Some(id) = call.signature {
            return Some(self.signature(id));
        }
        self.callee_function(call.callee)
            .map(|f| f.signature)
            .or_else(|| {
                if let Value::Symbol(symbol) = call.callee {
                    self.forward_boundary(symbol)
                } else {
                    None
                }
            })
            .map(|signature| self.signature(signature))
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
        self.stream_last_function(output, false, false).map(|_| ())
    }
    pub(crate) fn stream_last_body(
        &mut self,
        output: &mut impl std::io::Write,
    ) -> Result<SymbolId, Error> {
        self.stream_last_function(output, true, false)
    }
    /// Replace only function-definition positions, as `ProgramLowering`'s retained
    /// order does; globals and declarations retain their publication positions.
    pub(crate) fn order_retained_definitions(&mut self, completed: &[SymbolId]) {
        let mut seen = HashSet::new();
        let completed: Vec<_> = completed
            .iter()
            .copied()
            .filter(|&symbol| {
                self.function(self.symbol(symbol))
                    .is_some_and(|function| function.body.is_some())
                    && seen.insert(symbol)
            })
            .collect();
        let definitions: Vec<_> = self
            .publications
            .iter()
            .enumerate()
            .filter_map(|(index, &symbol)| {
                self.function(self.symbol(symbol))
                    .is_some_and(|function| function.body.is_some())
                    .then_some(index)
            })
            .collect();
        if completed.len() == definitions.len() {
            for (index, symbol) in definitions.into_iter().zip(completed) {
                self.publications[index] = symbol;
            }
        }
    }
    pub(crate) fn defer_last_definition(&mut self) -> Result<(), Error> {
        self.stream_last_function(&mut std::io::sink(), false, true)
            .map(|_| ())
    }
    pub(crate) fn release_held_definitions(&mut self) {
        if let Some(retained) = &mut self.retained_definitions {
            retained.completed.append(&mut retained.held);
        }
        if let Some(sink) = self.definition_sink.as_mut() {
            sink.release_held();
        }
    }
    fn stream_last_function(
        &mut self,
        output: &mut impl std::io::Write,
        body_only: bool,
        held: bool,
    ) -> Result<SymbolId, Error> {
        let symbol = self
            .completed_definition
            .ok_or_else(|| Error::Construction("missing completed definition".into()))?;
        if let Some(retained) = &mut self.retained_definitions {
            if held {
                retained.held.push(symbol);
            } else {
                retained.completed.push(symbol);
            }
            self.completed_definition = None;
            return Ok(symbol);
        }
        if let Some(mut sink) = self.definition_sink.take() {
            let result = sink.append(self, symbol, held);
            self.definition_sink = Some(sink);
            result?;
        } else {
            let function = self.function(self.symbol(symbol)).ok_or_else(|| {
                Error::Construction("completed definition without function".into())
            })?;
            if body_only {
                super::writer::write_definition_body(self, function, output)
            } else {
                super::writer::write_definition(self, function, output)
            }
            .map_err(Error::Output)?;
        }
        self.release_body(symbol)?;
        self.completed_definition = None;
        Ok(symbol)
    }
    pub(crate) fn complete_body(&mut self, symbol: SymbolId, body: Body) -> Result<(), Error> {
        let SymbolDetail::Function(id) = self.symbol(symbol).detail else {
            return Err(Error::Construction(
                "body completion for a non-function".into(),
            ));
        };
        self.functions[id.0 as usize].body = Some(body);
        self.completed_definition = Some(symbol);
        Ok(())
    }
    pub(crate) fn release_body(&mut self, symbol: SymbolId) -> Result<(), Error> {
        let SymbolDetail::Function(id) = self.symbol(symbol).detail else {
            return Err(Error::Construction(
                "body release for a non-function".into(),
            ));
        };
        self.streamed_functions.insert(symbol);
        self.functions[id.0 as usize].body = None;
        Ok(())
    }
    pub(crate) fn first_source(&self) -> Option<&Path> {
        self.sources.first().map(PathBuf::as_path)
    }
    pub(crate) fn function_defined(&self, symbol: SymbolId) -> bool {
        self.streamed_functions.contains(&symbol)
            || self
                .function(self.symbol(symbol))
                .is_some_and(|f| f.body.is_some())
    }
    pub(crate) fn function_declarations(&self) -> impl Iterator<Item = &Function> {
        self.immediate_declarations
            .iter()
            .filter_map(|&id| {
                let function = self.function(self.symbol(id))?;
                (!self.function_defined(id)).then_some(function)
            })
            .chain(
                self.symbols
                    .iter()
                    .enumerate()
                    .filter_map(|(index, symbol)| {
                        let SymbolDetail::Function(id) = symbol.detail else {
                            return None;
                        };
                        let symbol_id = SymbolId(u32::try_from(index).ok()?);
                        let function = &self.functions[id.0 as usize];
                        (function.body.is_none()
                            && !self.streamed_functions.contains(&symbol_id)
                            && !self.immediate_declaration_set.contains(&symbol_id))
                        .then_some(function)
                    }),
            )
    }
    /// Runtime declarations publish immediately; ordinary referenced declarations
    /// are the producer's pending list, published after all definitions.
    pub(crate) fn publish_runtime_declaration(&mut self, symbol: SymbolId) {
        if self.immediate_declaration_set.insert(symbol) {
            self.immediate_declarations.push(symbol);
        }
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
        self.publications.push(symbol);
        Ok(())
    }
    pub(crate) fn publications(&self) -> &[SymbolId] {
        &self.publications
    }
    pub(crate) fn replace_publications(&mut self, order: Vec<SymbolId>) {
        self.publications = order;
    }
    pub(crate) fn discard_function_definition(&mut self, symbol: SymbolId) -> Result<(), Error> {
        let body = self.take_body(symbol)?.ok_or(Error::Limit)?;
        drop(body);
        if self.completed_definition == Some(symbol) {
            self.completed_definition = None;
        }
        Ok(())
    }
    pub(crate) fn drop_aliases_of(&mut self, dropped: &[bool]) {
        self.aliases.retain(|alias| !dropped[alias.target.index()]);
        self.alias_names.clear();
        self.alias_names
            .extend(self.aliases.iter().map(|alias| alias.object));
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
    /// Source object declarations require a definition in another unit. TLS
    /// wrappers carry a generated address definition, even before encoding.
    pub(crate) fn require_object_definitions(&mut self) {
        for symbol in &self.symbols {
            if self.optional_references.contains(&self.index[&symbol.name]) {
                continue;
            }
            let metadata = match symbol.detail {
                SymbolDetail::Function(id) => {
                    let function = &mut self.functions[id.0 as usize];
                    if function.body.is_some()
                        || function.metadata.tls_for.is_some()
                        || self.streamed_functions.contains(&self.index[&symbol.name])
                    {
                        continue;
                    }
                    &mut function.metadata
                }
                SymbolDetail::Global(id) => {
                    let global = &mut self.globals[id.0 as usize];
                    if !matches!(global.init, GlobalInit::Declaration) {
                        continue;
                    }
                    &mut global.metadata
                }
                SymbolDetail::Unresolved => continue,
            };
            if metadata.binding == Some(super::vocabulary::Binding::Weak) {
                metadata.binding = Some(super::vocabulary::Binding::Strong);
            }
        }
    }
    pub(crate) fn optional_reference(&mut self, symbol: SymbolId) {
        self.optional_references.insert(symbol);
    }
    pub(crate) fn root_function(&mut self, symbol: SymbolId) {
        if let SymbolDetail::Function(id) = self.symbol(symbol).detail {
            self.functions[id.0 as usize].metadata.flags |= 4;
        }
    }
    pub(crate) fn function(&self, symbol: &Symbol) -> Option<&Function> {
        if let SymbolDetail::Function(id) = symbol.detail {
            Some(&self.functions[id.0 as usize])
        } else {
            None
        }
    }
    pub(crate) fn take_body(&mut self, symbol: SymbolId) -> Result<Option<Body>, Error> {
        let SymbolDetail::Function(id) = self.symbol(symbol).detail else {
            return Err(Error::Construction(
                "body mutation for a non-function".into(),
            ));
        };
        Ok(self.functions[id.0 as usize].body.take())
    }
    pub(crate) fn restore_body(&mut self, symbol: SymbolId, body: Body) -> Result<(), Error> {
        let SymbolDetail::Function(id) = self.symbol(symbol).detail else {
            return Err(Error::Construction(
                "body restoration for a non-function".into(),
            ));
        };
        let slot = &mut self.functions[id.0 as usize].body;
        if slot.is_some() {
            return Err(Error::Construction(
                "body restored over an existing definition".into(),
            ));
        }
        *slot = Some(body);
        Ok(())
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
/// Address-selection facts stated before a streamed global is published.
#[derive(Clone, Copy)]
pub(crate) struct ForwardGlobal {
    pub(crate) binding: Option<Binding>,
    pub(crate) storage: Option<Storage>,
    pub(crate) defined: bool,
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
#[derive(Clone)]
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
    origin: BlockName,
}
#[derive(Clone, Copy)]
enum BlockName {
    Source(NameId),
    Generated(NameId),
    Anonymous,
}
#[derive(Default)]
pub(crate) struct Body {
    literals: Vec<u128>,
    operands: Vec<Value>,
    temps: Vec<Temp>,
    slots: Vec<Slot>,
    slot_names: Vec<super::construction_names::SlotName>,
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
    pub(super) fn retarget_region(&mut self, id: InstId, target: BlockId) -> Result<(), Error> {
        match &mut self.instructions[id.0 as usize].kind {
            InstructionKind::Runtime(RuntimeInstruction::EhTry(current))
            | InstructionKind::Jump(current) => *current = target,
            _ => return Err(Error::Limit),
        }
        Ok(())
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
    pub(crate) fn replace_argument(
        &mut self,
        arguments: Arguments,
        index: usize,
        value: Value,
    ) -> Result<(), Error> {
        if index >= arguments.count as usize {
            return Err(Error::Limit);
        }
        let position = arguments.first as usize + index;
        let previous = self.operands[position];
        if previous == value {
            return Ok(());
        }
        if let Value::Temp(temp) = value {
            self.temp_mut(temp).uses = self.temp(temp).uses.checked_add(1).ok_or(Error::Limit)?;
        }
        if let Value::Temp(temp) = previous {
            self.temp_mut(temp).uses = self.temp(temp).uses.checked_sub(1).ok_or(Error::Limit)?;
        }
        self.operands[position] = value;
        Ok(())
    }
    pub(crate) fn inherit_location(&mut self, destination: InstId, source: InstId) {
        self.instructions[destination.index()].location =
            self.instructions[source.index()].location;
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
    pub(crate) fn temp_id(&self, index: usize) -> Option<TempId> {
        (index < self.temps.len())
            .then(|| u32::try_from(index).ok().map(TempId))
            .flatten()
    }
    pub(crate) fn parameter_value(&self, index: usize) -> Result<Value, Error> {
        if index >= self.parameter_count || index >= self.temps.len() {
            return Err(Error::Limit);
        }
        Ok(Value::Temp(TempId(
            u32::try_from(index).map_err(|_| Error::Limit)?,
        )))
    }
    pub(crate) fn generated_block(&mut self) -> Result<BlockId, Error> {
        let id = BlockId(u32::try_from(self.blocks.len()).map_err(|_| Error::Limit)?);
        self.blocks.push(Block {
            name: NameId(0),
            origin: BlockName::Anonymous,
            placed: false,
            first_use: 0,
            instructions: Vec::new(),
        });
        Ok(id)
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
            origin: BlockName::Source(name),
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
    pub(super) fn retain_slot_names(&mut self, names: Vec<super::construction_names::SlotName>) {
        self.slot_names = names;
    }
    pub(crate) fn refarg_slot(&mut self, ty: Type) -> Result<Value, Error> {
        if self.slot_names.is_empty() {
            self.slot_names.extend(
                self.slots
                    .iter()
                    .map(|slot| super::construction_names::SlotName::Source(slot.name)),
            );
        }
        let slot = self.constructed_slot(ty)?;
        self.slot_names
            .push(super::construction_names::SlotName::Generated("refarg"));
        Ok(slot)
    }
    pub(crate) fn name_optimizer_slots(
        &mut self,
        names: &mut Names,
        parameters: &[NameId],
    ) -> Result<(), Error> {
        let positional: HashSet<_> = self
            .slot_names
            .iter()
            .filter_map(|label| {
                if let super::construction_names::SlotName::Parameter(index) = label {
                    Some(*index)
                } else {
                    None
                }
            })
            .collect();
        let mut explicit: Vec<_> = parameters
            .iter()
            .enumerate()
            .filter_map(|(index, &name)| {
                (!positional.contains(&index)
                    && self.temps.get(index).is_none_or(|temp| !temp.generated))
                .then_some(name)
            })
            .collect();
        explicit.extend(
            self.temps
                .iter()
                .enumerate()
                .filter(|(index, temp)| !temp.generated && !positional.contains(index))
                .map(|(_, temp)| temp.name),
        );
        let labels = super::construction_names::slots(names, &self.slot_names, &explicit)?;
        self.name_constructed_slots(&labels)
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
#[derive(Clone)]
pub(crate) struct Instruction {
    pub(crate) result: Option<TempId>,
    pub(crate) kind: InstructionKind,
    pub(crate) line: u32,
    location: Option<LocationId>,
    switch_type: Option<Type>,
}
impl Instruction {
    pub(crate) fn new(result: Option<TempId>, kind: InstructionKind, line: u32) -> Self {
        Self {
            result,
            kind,
            line,
            location: None,
            switch_type: None,
        }
    }
    pub(crate) fn switch_type(&self) -> Option<Type> {
        self.switch_type
    }
}
#[derive(Clone)]
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
#[derive(Clone)]
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
#[derive(Clone)]
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
#[derive(Clone, Copy, Default, PartialEq, Eq)]
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

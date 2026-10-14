// Translated from lower/symbol_lowering.cpp's procedural symbol and ABI construction. See NOTICE.
use super::Error;
use crate::{
    abi::{
        self,
        graph::{Modifier, Node, NodeId, Qualifiers},
        vocabulary::Builtin as B,
    },
    lowir::model::{Linkage, Metadata, NameId, Program, SourceId, SymbolId, Value},
    sema::{
        Graph,
        lowering::{EntityId, FunctionName, Scalar, ScopeId, TypeId, TypeView},
    },
    support::stack::StackBudget,
    syntax::{ast, cursor::Cursor},
    types::FundamentalType as F,
};
use std::{collections::HashMap, fmt::Write};
#[derive(Default)]
pub(super) struct SymbolTable {
    objects: HashMap<NameId, Option<SymbolId>>,
    suffixes: HashMap<NameId, u32>,
    source: Option<SourceId>,
    string_literals: u32,
    reference_temporaries: u32,
}
impl SymbolTable {
    fn allocate(
        &mut self,
        program: &mut Program,
        spelling: &str,
        object: Option<NameId>,
        internal: bool,
    ) -> Result<SymbolId, Error> {
        if !internal
            && let Some(object) = object
            && let Some(Some(symbol)) = self.objects.get(&object)
        {
            return Ok(*symbol);
        }
        let base = program.names.intern(spelling)?;
        let mut name = base;
        if program.find_symbol(name).is_some() {
            let suffix = self.suffixes.entry(base).or_insert(2);
            loop {
                name = program.names.intern(&format!("{spelling}__{suffix}"))?;
                *suffix = suffix.checked_add(1).ok_or(crate::lowir::Error::Limit)?;
                if program.find_symbol(name).is_none() {
                    break;
                }
            }
        }
        let source = if let Some(source) = self.source {
            source
        } else {
            let source = program.add_source(std::path::Path::new(""))?;
            self.source = Some(source);
            source
        };
        let symbol = program.reference(name, source, 0)?;
        if let Some(object) = object {
            if internal {
                self.objects.entry(object).or_insert(None);
            } else {
                self.objects.insert(object, Some(symbol));
            }
        }
        Ok(symbol)
    }
}
pub(super) struct Symbols<'a, 'source> {
    graph: &'a Graph,
    tree: &'a ast::Tree,
    cursor: &'a Cursor<'source>,
    abi: abi::graph::Graph,
    types: HashMap<TypeId, NodeId>,
    scopes: HashMap<ScopeId, Option<NodeId>>,
    budget: StackBudget,
    stack_base: usize,
    pub pending: Vec<EntityId>,
    seen: std::collections::HashSet<EntityId>,
    ordinals: HashMap<EntityId, usize>,
    names: HashMap<EntityId, std::sync::Arc<str>>,
    callees: HashMap<EntityId, Value>,
    table: &'a mut SymbolTable,
    pub globals: Vec<EntityId>,
    objects: HashMap<EntityId, Option<NameId>>,
    signatures: HashMap<(TypeId, bool), crate::lowir::model::SignatureId>,
    pub data: super::data::Objects,
    temporary_owner: Option<(EntityId, u32)>,
}
impl<'a, 'source> Symbols<'a, 'source> {
    pub(super) fn new(
        graph: &'a Graph,
        tree: &'a ast::Tree,
        cursor: &'a Cursor<'source>,
        budget: StackBudget,
        table: &'a mut SymbolTable,
    ) -> Self {
        let marker = 0u8;
        Self {
            stack_base: std::ptr::from_ref(&marker).addr(),
            graph,
            tree,
            cursor,
            abi: abi::graph::Graph::default(),
            types: HashMap::new(),
            scopes: HashMap::new(),
            budget,
            pending: Vec::new(),
            seen: std::collections::HashSet::new(),
            ordinals: HashMap::new(),
            names: HashMap::new(),
            callees: HashMap::new(),
            table,
            globals: Vec::new(),
            objects: HashMap::new(),
            data: super::data::Objects::default(),
            temporary_owner: None,
            signatures: HashMap::new(),
        }
    }
    pub(super) fn identifier(&self, name: crate::lex::identifiers::NameId) -> Result<&str, Error> {
        std::str::from_utf8(self.cursor.name(name))
            .map_err(|_| Error::Missing("local name spelling"))
    }
    pub(super) fn literal_bytes(&self, literal: ast::Literal) -> &[u8] {
        self.cursor.spellings.get(literal.data)
    }
    fn function_label(
        &self,
        name: FunctionName,
        label: Option<ast::Name>,
    ) -> Result<String, Error> {
        let mut bytes = Vec::new();
        if let FunctionName::Identifier(name) = name {
            bytes.extend_from_slice(self.cursor.name(name));
        } else if let Some(label) = label {
            crate::syntax::write_member_label(self.tree, self.cursor, label, &mut bytes).map_err(
                |_| Error::Semantic(crate::sema::Error::Invariant("function label rendering")),
            )?;
        } else {
            match name {
                FunctionName::Operator { token, array } => {
                    bytes.extend_from_slice(b"operator");
                    bytes.extend_from_slice(token.spelling().as_bytes());
                    if array {
                        bytes.extend_from_slice(b"[]");
                    }
                }
                FunctionName::Literal(suffix) => {
                    bytes.extend_from_slice(b"operator\"\"");
                    bytes.extend_from_slice(self.cursor.name(suffix));
                }
                FunctionName::Identifier(_) => {
                    return Err(Error::Semantic(crate::sema::Error::Invariant(
                        "ordinary function label",
                    )));
                }
            }
        }
        // C++ AppendIdentifierSpelling: source labels are presentation only.
        Ok(bytes
            .into_iter()
            .filter(|&byte| byte != b' ')
            .map(|byte| {
                char::from(if byte.is_ascii_alphanumeric() || byte == b'_' {
                    byte
                } else {
                    b'_'
                })
            })
            .collect())
    }
    fn function_terminal(&mut self, name: FunctionName, ty: TypeId) -> Result<NodeId, Error> {
        let node = match name {
            FunctionName::Identifier(name) => {
                return Ok(self.abi.string(self.cursor.name(name))?);
            }
            FunctionName::Literal(suffix) => {
                Node::LiteralOperator(self.abi.string(self.cursor.name(suffix))?)
            }
            FunctionName::Operator { token, array } => {
                use crate::{
                    abi::vocabulary::Terminal as T,
                    lex::{pp_lexer::Punctuator as P, vocabulary::Keyword as K},
                };
                let terminal = match (token.as_keyword(), token.as_punctuator()) {
                    (Some(K::New), _) => {
                        if array {
                            T::NewArray
                        } else {
                            T::New
                        }
                    }
                    (Some(K::Delete), _) => {
                        if array {
                            T::DeleteArray
                        } else {
                            T::Delete
                        }
                    }
                    (_, Some(op)) => match op {
                        P::Add => T::Plus,
                        P::Subtract => T::Minus,
                        P::Multiply => T::Multiply,
                        P::Divide => T::Divide,
                        P::Remainder => T::Remainder,
                        P::BitAnd => T::BitAnd,
                        P::BitOr => T::BitOr,
                        P::Xor => T::BitXor,
                        P::Complement => T::Complement,
                        P::Assign => T::Assign,
                        P::AddAssign => T::PlusAssign,
                        P::SubtractAssign => T::MinusAssign,
                        P::MultiplyAssign => T::MultiplyAssign,
                        P::DivideAssign => T::DivideAssign,
                        P::RemainderAssign => T::RemainderAssign,
                        P::AndAssign => T::AndAssign,
                        P::OrAssign => T::OrAssign,
                        P::XorAssign => T::XorAssign,
                        P::ShiftLeft => T::LeftShift,
                        P::ShiftRight => T::RightShift,
                        P::ShiftLeftAssign => T::LeftShiftAssign,
                        P::ShiftRightAssign => T::RightShiftAssign,
                        P::Equal => T::Equal,
                        P::NotEqual => T::NotEqual,
                        P::Less => T::Less,
                        P::Greater => T::Greater,
                        P::LessEqual => T::LessEqual,
                        P::GreaterEqual => T::GreaterEqual,
                        P::LogicalNot => T::LogicalNot,
                        P::LogicalAnd => T::LogicalAnd,
                        P::LogicalOr => T::LogicalOr,
                        P::Increment => T::Increment,
                        P::Decrement => T::Decrement,
                        P::Comma => T::Comma,
                        P::ArrowStar => T::MemberPointer,
                        P::Arrow => T::Arrow,
                        P::LeftParen => T::Call,
                        P::LeftBracket => T::Index,
                        _ => return Err(Error::Missing("operator function ABI terminal")),
                    },
                    _ => return Err(Error::Missing("operator function ABI terminal")),
                };
                let TypeView::Function { parameters, .. } = self.graph.lower_type(ty)? else {
                    return Err(Error::Semantic(crate::sema::Error::Invariant(
                        "operator without function type",
                    )));
                };
                Node::Terminal {
                    kind: terminal.for_operands(parameters.len()),
                    inherited: None,
                }
            }
        };
        Ok(self.abi.intern(node, &[])?)
    }
    pub(super) fn reference_temporary_symbol(
        &mut self,
        program: &mut Program,
        entity: EntityId,
        constant: bool,
    ) -> Result<(SymbolId, Option<NameId>), Error> {
        let spelling = if constant {
            self.table.reference_temporaries = self
                .table
                .reference_temporaries
                .checked_add(1)
                .ok_or(crate::lowir::Error::Limit)?;
            format!("__ref_tmp__{}", self.table.reference_temporaries)
        } else {
            format!(
                "__ref_tmp__{}",
                self.identifier(self.graph.lower_variable(entity)?.name)?
            )
        };
        let Some((owner, sequence)) = self.temporary_owner else {
            return Ok((self.table.allocate(program, &spelling, None, true)?, None));
        };
        self.temporary_owner = Some((
            owner,
            sequence.checked_add(1).ok_or(crate::lowir::Error::Limit)?,
        ));
        let facts = self.graph.lower_variable(owner)?;
        let parent = self.scope(facts.home)?;
        let terminal = self.abi.string(self.cursor.name(facts.name))?;
        let owner = self.abi.intern(
            Node::Name {
                parent,
                terminal,
                internal: facts.internal,
                occurrence: 0,
            },
            &[],
        )?;
        let mut bytes = Vec::new();
        abi::encoder::Encoder::encode(
            &self.abi,
            abi::encoder::Target::ReferenceTemporary { owner, sequence },
            &mut bytes,
            self.budget,
        )?;
        let object = program.names.intern_bytes(&bytes)?;
        let local_object = (!self.table.objects.contains_key(&object)).then_some(object);
        let symbol = self
            .table
            .allocate(program, &spelling, Some(object), true)?;
        Ok((symbol, local_object))
    }
    pub(super) fn replace_temporary_owner(
        &mut self,
        owner: Option<(EntityId, u32)>,
    ) -> Option<(EntityId, u32)> {
        std::mem::replace(&mut self.temporary_owner, owner)
    }
    pub(super) fn data_symbol(
        &mut self,
        program: &mut Program,
        image: Option<usize>,
    ) -> Result<SymbolId, Error> {
        let name = if let Some(index) = image {
            format!("__constexpr_template__{}", index + 1)
        } else {
            self.table.string_literals = self
                .table
                .string_literals
                .checked_add(1)
                .ok_or(crate::lowir::Error::Limit)?;
            format!("__strlit__{}", self.table.string_literals)
        };
        self.table.allocate(program, &name, None, true)
    }
    pub(super) fn check_stack(&self) -> Result<(), Error> {
        let marker = 0u8;
        if self
            .budget
            .exceeded(self.stack_base, std::ptr::from_ref(&marker).addr())
        {
            return Err(Error::Semantic(crate::sema::Error::Limit(
                "constructs nested too deeply",
            )));
        }
        Ok(())
    }
    fn scope(&mut self, scope: ScopeId) -> Result<Option<NodeId>, Error> {
        self.check_stack()?;
        if let Some(&id) = self.scopes.get(&scope) {
            return Ok(id);
        }
        let (parent, name) = self.graph.lower_namespace(scope)?;
        let id = if let Some(parent) = parent {
            let parent = self.scope(parent)?;
            let name = name.ok_or(Error::Missing("anonymous namespace symbol lowering"))?;
            let terminal = self
                .abi
                .string(self.cursor.name(name))
                .map_err(Error::from)?;
            Some(
                self.abi
                    .intern(
                        Node::Name {
                            parent,
                            terminal,
                            internal: false,
                            occurrence: 0,
                        },
                        &[],
                    )
                    .map_err(Error::from)?,
            )
        } else {
            None
        };
        self.scopes.insert(scope, id);
        Ok(id)
    }
    fn abi_type(&mut self, ty: TypeId) -> Result<NodeId, Error> {
        self.check_stack()?;
        if let Some(&id) = self.types.get(&ty) {
            return Ok(id);
        }
        let node = match self.graph.lower_type(ty)? {
            TypeView::Scalar(scalar) => Node::Builtin(match scalar {
                Scalar::Void => B::Void,
                Scalar::Bool => B::Bool,
                Scalar::SignedChar => B::SignedChar,
                Scalar::UnsignedChar => B::UnsignedChar,
                Scalar::Short => B::Short,
                Scalar::UnsignedShort => B::UnsignedShort,
                Scalar::Nullptr => B::Nullptr,
                Scalar::Basic(f) => match f {
                    F::Char => B::Char,
                    F::Char16T => B::Char16,
                    F::Char32T => B::Char32,
                    F::WcharT => B::Wchar,
                    F::Int => B::Int,
                    F::UnsignedInt => B::UnsignedInt,
                    F::LongInt => B::Long,
                    F::UnsignedLongInt => B::UnsignedLong,
                    F::LongLongInt => B::LongLong,
                    F::UnsignedLongLongInt => B::UnsignedLongLong,
                    F::Float => B::Float,
                    F::Double => B::Double,
                    F::LongDouble => B::LongDouble,
                },
            }),
            TypeView::Pointer(element) => Node::Modified {
                modifier: Modifier::Pointer,
                operand: self.abi_type(element)?,
            },
            TypeView::Array { element, bound } => Node::Modified {
                modifier: if bound == 0 {
                    Modifier::UnknownArray
                } else {
                    Modifier::Array(bound)
                },
                operand: self.abi_type(element)?,
            },
            TypeView::Reference { element, rvalue } => Node::Modified {
                modifier: if rvalue {
                    Modifier::RvalueReference
                } else {
                    Modifier::LvalueReference
                },
                operand: self.abi_type(element)?,
            },
            TypeView::Enum(_) => {
                let (home, name) = self.graph.lower_nominal_name(ty)?;
                let parent = self.scope(home)?;
                let terminal = self.abi.string(self.cursor.name(name))?;
                Node::Name {
                    parent,
                    terminal,
                    internal: false,
                    occurrence: 0,
                }
            }
            TypeView::Function {
                result,
                parameters,
                variadic,
            } => {
                let result = self.abi_type(result)?;
                let mut args = Vec::with_capacity(parameters.len());
                for &parameter in parameters {
                    args.push(self.abi_type(parameter)?);
                }
                let node = Node::FunctionType {
                    result,
                    owner: None,
                    qualifiers: Qualifiers::default(),
                    variadic,
                };
                let id = self.abi.intern(node, &args)?;
                self.types.insert(ty, id);
                return Ok(id);
            }
            TypeView::Unsupported => return Err(Error::Missing("aggregate ABI type lowering")),
        };
        let mut id = self.abi.intern(node, &[]).map_err(Error::from)?;
        for (bit, modifier) in [
            (2, Modifier::Qualified(Qualifiers::VOLATILE)),
            (1, Modifier::Qualified(Qualifiers::CONST)),
        ] {
            if !matches!(self.graph.lower_type(ty)?, TypeView::Array { .. })
                && self.graph.lower_cv(ty) & bit != 0
            {
                id = self
                    .abi
                    .intern(
                        Node::Modified {
                            modifier,
                            operand: id,
                        },
                        &[],
                    )
                    .map_err(Error::from)?;
            }
        }
        self.types.insert(ty, id);
        Ok(id)
    }
    pub(super) fn demand(&mut self, entity: EntityId) {
        let entity = self.graph.lower_function_entity(entity);
        if self.seen.insert(entity) {
            self.pending.push(entity);
        }
    }
    pub(super) fn callee(
        &mut self,
        entity: EntityId,
        builder: &mut crate::lowir::builder::FunctionBuilder<'_>,
    ) -> Result<crate::lowir::model::Value, Error> {
        if let Some(&value) = self.callees.get(&entity) {
            return Ok(value);
        }
        let symbol = self.function_symbol(builder.program(), entity)?;
        let value = Value::Symbol(symbol);
        self.callees.insert(entity, value);
        Ok(value)
    }
    pub(super) fn function_symbol(
        &mut self,
        program: &mut Program,
        entity: EntityId,
    ) -> Result<SymbolId, Error> {
        let entity = self.graph.lower_function_entity(entity);
        if let Some(Value::Symbol(symbol)) = self.callees.get(&entity) {
            return Ok(*symbol);
        }
        let name = self.name(entity)?;
        let facts = self.graph.lower_function(entity)?;
        let metadata = self.metadata(program, entity, &name)?;
        let object = if facts.builtin.is_some() {
            metadata.object.or(Some(program.names.intern(&name)?))
        } else if facts.c_linkage && !facts.internal
            || metadata.role == Some(crate::lowir::vocabulary::Role::Entry)
        {
            Some(program.names.intern(&name)?)
        } else {
            metadata.object
        };
        let local_object = if facts.internal
            && (facts.c_linkage
                || object.is_some_and(|object| self.table.objects.contains_key(&object)))
        {
            None
        } else {
            metadata.object
        };
        let symbol = self
            .table
            .allocate(program, &name, object, facts.internal)?;
        self.objects.insert(entity, local_object);
        self.callees.insert(entity, Value::Symbol(symbol));
        Ok(symbol)
    }
    pub(super) fn name(&mut self, entity: EntityId) -> Result<std::sync::Arc<str>, Error> {
        let entity = self.graph.lower_function_entity(entity);
        if let Some(name) = self.names.get(&entity) {
            return Ok(std::sync::Arc::clone(name));
        }
        if !self.ordinals.contains_key(&entity) {
            for (index, overload) in self.graph.lower_overloads(entity)?.enumerate() {
                self.ordinals.insert(overload, index + 1);
            }
        }
        let facts = self.graph.lower_function(entity)?;
        let own = match facts.builtin {
            Some(crate::sema::lowering::BuiltinFunction::Memcmp) => "memcmp".to_owned(),
            Some(crate::sema::lowering::BuiltinFunction::Memset) => "memset".to_owned(),
            _ if facts.c_linkage && !facts.internal => {
                let FunctionName::Identifier(name) = facts.name else {
                    return Err(Error::Semantic(crate::sema::Error::Invariant(
                        "C linkage function without identifier",
                    )));
                };
                self.identifier(name)?.to_owned()
            }
            _ => self.function_label(facts.name, facts.label)?,
        };
        let mut components = vec![own.as_str()];
        if !facts.c_linkage || facts.internal {
            let mut scope = facts.home;
            while let (Some(parent), name) = self.graph.lower_namespace(scope)? {
                let name = name.ok_or(Error::Missing("anonymous namespace symbols"))?;
                components.push(
                    std::str::from_utf8(self.cursor.name(name))
                        .map_err(|_| Error::Missing("non-ASCII symbol rendering"))?,
                );
                scope = parent;
            }
        }
        components.reverse();
        let mut name = components.join("__");
        let ordinal = self.ordinals.get(&entity).copied().unwrap_or(1);
        if ordinal > 1 && (!facts.c_linkage || facts.internal) {
            write!(name, "__ov{ordinal}").map_err(|_| {
                Error::Semantic(crate::sema::Error::Invariant("symbol formatting failure"))
            })?;
        }
        let name: std::sync::Arc<str> = name.into();
        self.names.insert(entity, std::sync::Arc::clone(&name));
        Ok(name)
    }
    pub(super) fn call_signature(
        &mut self,
        program: &mut Program,
        ty: TypeId,
        nothrow: bool,
    ) -> Result<crate::lowir::model::SignatureId, Error> {
        if let Some(&signature) = self.signatures.get(&(ty, nothrow)) {
            return Ok(signature);
        }
        let signature = super::function::signature(program, self.graph, ty, nothrow)?;
        self.signatures.insert((ty, nothrow), signature);
        Ok(signature)
    }
    pub(super) fn variable_symbol(
        &mut self,
        program: &mut Program,
        entity: EntityId,
    ) -> Result<SymbolId, Error> {
        if let Some(Value::Symbol(symbol)) = self.callees.get(&entity) {
            return Ok(*symbol);
        }
        let facts = self.graph.lower_variable(entity)?;
        if facts.thread_local {
            return Err(Error::Missing("thread-local global lowering"));
        }
        let mut components = vec![self.identifier(facts.name)?];
        let mut scope = facts.home;
        while let (Some(parent), name) = self.graph.lower_namespace(scope)? {
            components.push(self.identifier(
                name.ok_or(Error::Missing("anonymous namespace variable lowering"))?,
            )?);
            scope = parent;
        }
        components.reverse();
        let spelling = components.join("__");
        let parent = self.scope(facts.home)?;
        let object = if facts.c_linkage || parent.is_none() && !facts.internal {
            program.names.intern_bytes(self.cursor.name(facts.name))?
        } else {
            let terminal = self.abi.string(self.cursor.name(facts.name))?;
            let name = self.abi.intern(
                Node::Name {
                    parent,
                    terminal,
                    internal: facts.internal,
                    occurrence: 0,
                },
                &[],
            )?;
            let mut bytes = Vec::new();
            abi::encoder::Encoder::encode(
                &self.abi,
                abi::encoder::Target::Entity(name),
                &mut bytes,
                self.budget,
            )?;
            program.names.intern_bytes(&bytes)?
        };
        let local_object =
            if facts.internal && (facts.c_linkage || self.table.objects.contains_key(&object)) {
                None
            } else {
                Some(object)
            };
        let symbol = self
            .table
            .allocate(program, &spelling, Some(object), facts.internal)?;
        self.callees.insert(entity, Value::Symbol(symbol));
        self.objects.insert(entity, local_object);
        self.globals.push(entity);
        Ok(symbol)
    }
    pub(super) fn variable_metadata(
        &self,
        program: &Program,
        entity: EntityId,
        symbol: SymbolId,
    ) -> Result<Metadata, Error> {
        let facts = self.graph.lower_variable(entity)?;
        let object = self
            .objects
            .get(&entity)
            .copied()
            .flatten()
            .filter(|&object| facts.internal || object != program.symbol(symbol).name);
        Ok(Metadata {
            linkage: if facts.c_linkage {
                Linkage::C
            } else {
                Linkage::Default
            },
            binding: Some(if facts.internal {
                crate::lowir::vocabulary::Binding::Internal
            } else {
                crate::lowir::vocabulary::Binding::Strong
            }),
            object,
            ..Metadata::default()
        })
    }
    pub(super) fn metadata(
        &mut self,
        program: &mut crate::lowir::model::Program,
        entity: EntityId,
        name: &str,
    ) -> Result<Metadata, Error> {
        use crate::lowir::vocabulary::{Binding, Role};
        let entity = self.graph.lower_function_entity(entity);
        let facts = self.graph.lower_function(entity)?;
        if name == "main" && self.graph.lower_namespace(facts.home)?.0.is_none() {
            return Ok(Metadata {
                role: Some(Role::Entry),
                binding: Some(Binding::Strong),
                flags: 1,
                ..Metadata::default()
            });
        }
        let binding = if facts.internal {
            Binding::Internal
        } else if facts.emission == crate::sema::lowering::Emission::Demanded {
            Binding::Weak
        } else {
            Binding::Strong
        };
        let mut metadata = Metadata {
            binding: Some(binding),
            flags: if facts.emission == crate::sema::lowering::Emission::Demanded {
                16
            } else {
                0
            },
            ..Metadata::default()
        };
        if let Some(builtin) = facts.builtin {
            use crate::sema::lowering::BuiltinFunction as B;
            match builtin {
                B::Memcpy | B::Memmove | B::Strlen => {
                    let object = match builtin {
                        B::Memcpy => "cppgm_builtin_memcpy",
                        B::Memmove => "cppgm_builtin_memmove",
                        _ => "cppgm_builtin_strlen",
                    };
                    metadata.object = Some(program.names.intern(object)?);
                }
                B::Memcmp | B::Memset | B::Abort => metadata.linkage = Linkage::C,
            }
            return Ok(metadata);
        }
        if let Some(allocation) = facts.allocation {
            if allocation.array && facts.definition.is_none() {
                return Err(Error::Missing("array allocation runtime forwarding"));
            }
            if !allocation.array {
                metadata.role = Some(match allocation.role {
                    crate::sema::lowering::AllocationRole::Allocate => Role::AllocateMemory,
                    crate::sema::lowering::AllocationRole::Free => Role::FreeMemory,
                });
            }
        }
        if facts.c_linkage {
            metadata.linkage = Linkage::C;
        }
        if facts.c_linkage && !facts.internal {
            return Ok(metadata);
        }
        if let Some(&object) = self.objects.get(&entity) {
            metadata.object = object;
            return Ok(metadata);
        }
        if let Some(allocation) = facts.allocation {
            metadata.object = Some(program.names.intern(
                match (allocation.role, allocation.array) {
                    (crate::sema::lowering::AllocationRole::Allocate, false) => {
                        "cppgm_builtin_operator_new"
                    }
                    (crate::sema::lowering::AllocationRole::Allocate, true) => {
                        "cppgm_builtin_operator_new_array"
                    }
                    (crate::sema::lowering::AllocationRole::Free, false) => {
                        "cppgm_builtin_operator_delete"
                    }
                    (crate::sema::lowering::AllocationRole::Free, true) => {
                        "cppgm_builtin_operator_delete_array"
                    }
                },
            )?);
            return Ok(metadata);
        }
        let parent = self.scope(facts.home)?;
        let terminal = self.function_terminal(facts.name, facts.ty)?;
        let name = self
            .abi
            .intern(
                Node::Name {
                    parent,
                    terminal,
                    internal: facts.internal,
                    occurrence: 0,
                },
                &[],
            )
            .map_err(Error::from)?;
        let TypeView::Function {
            parameters,
            variadic,
            ..
        } = self.graph.lower_type(facts.ty)?
        else {
            return Err(Error::Semantic(crate::sema::Error::Invariant(
                "function without function type",
            )));
        };
        let mut args = Vec::with_capacity(parameters.len());
        for &parameter in parameters {
            args.push(self.abi_type(parameter)?);
        }
        let function = self
            .abi
            .intern(
                Node::Function {
                    name,
                    result: None,
                    qualifiers: Qualifiers::default(),
                    variadic,
                },
                &args,
            )
            .map_err(Error::from)?;
        let mut object = Vec::new();
        abi::encoder::Encoder::encode(
            &self.abi,
            abi::encoder::Target::Entity(function),
            &mut object,
            self.budget,
        )
        .map_err(Error::from)?;
        metadata.object = Some(program.names.intern_bytes(&object)?);
        Ok(metadata)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn external_identity_and_internal_names_survive_unit_boundaries() -> Result<(), Error> {
        let mut program = Program::default();
        let mut table = SymbolTable::default();
        let object = program.names.intern("_ZN1n1xE")?;
        let first = table.allocate(&mut program, "n__x", Some(object), false)?;
        let second = table.allocate(
            &mut program,
            "different_source_spelling",
            Some(object),
            false,
        )?;
        assert_eq!(first, second);
        let local_object = program.names.intern("_ZL1x")?;
        let local1 = table.allocate(&mut program, "x", Some(local_object), true)?;
        let local2 = table.allocate(&mut program, "x", Some(local_object), true)?;
        assert_ne!(local1, local2);
        assert_eq!(program.names.get(program.symbol(local2).name), "x__2");
        let collision = table.allocate(&mut program, "n__x", None, true)?;
        assert_eq!(program.names.get(program.symbol(collision).name), "n__x__2");
        let declared_suffix = table.allocate(&mut program, "x__3", None, true)?;
        let local3 = table.allocate(&mut program, "x", Some(local_object), true)?;
        assert_ne!(declared_suffix, local3);
        assert_eq!(program.names.get(program.symbol(local3).name), "x__4");
        Ok(())
    }
}

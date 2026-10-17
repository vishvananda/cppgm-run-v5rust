// Translated from lower/symbol_lowering.cpp's procedural symbol and ABI construction. See NOTICE.
use super::Error;
#[path = "placeholder_mangling.rs"]
mod placeholder_mangling;
#[path = "specialization_labels.rs"]
mod specialization_labels;
#[path = "vtables.rs"]
mod vtables;
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
    initializer_list_backings: u32,
    initializer: Option<SymbolId>,
    exception_helpers: Option<super::exception_helpers::Helpers>,
    terminate_pending: bool,
    bad_array_length: Option<SymbolId>,
    runtime_roles: Vec<super::runtime_roles::RoleSymbol>,
    entry_aliases: Vec<crate::lowir::model::ObjectAlias>,
}
impl SymbolTable {
    pub(super) fn finalizer_symbol(&mut self, program: &mut Program) -> Result<SymbolId, Error> {
        self.allocate(program, "__cppgm_fini", None, true)
    }
    pub(super) fn finish_entry_aliases(&mut self, program: &mut Program) {
        for alias in self.entry_aliases.drain(..) {
            if self
                .objects
                .get(&alias.object)
                .is_some_and(|symbol| symbol.is_none_or(|symbol| program.function_defined(symbol)))
            {
                continue;
            }
            let object = alias.object;
            let _inserted = program.add_alias(alias);
            self.objects.entry(object).or_insert(None);
        }
    }
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
enum EmptyListConstruction {
    Complete(bool),
    DepthCut,
}
pub(super) struct Symbols<'a, 'source> {
    graph: &'a Graph,
    tree: &'a ast::Tree,
    cursor: &'a Cursor<'source>,
    abi: abi::graph::Graph,
    types: HashMap<TypeId, NodeId>,
    placeholders: HashMap<TypeId, Option<NodeId>>,
    unmangled: HashMap<TypeId, bool>,
    scopes: HashMap<ScopeId, Option<NodeId>>,
    budget: StackBudget,
    stack_base: usize,
    pub pending: super::definition_demands::Demands,
    pub initialization: Option<SymbolId>,
    pub aggregate_helpers: super::aggregate_helpers::Helpers,
    seen: std::collections::HashSet<super::construction::Entry>,
    entries: HashMap<super::construction::Entry, SymbolId>,
    entry_objects: HashMap<super::construction::Entry, Option<NameId>>,
    entry_families:
        HashMap<crate::sema::lowering::ConstructorId, super::definition_order::FamilyId>,
    destructor_families: HashMap<EntityId, super::definition_order::FamilyId>,
    copy_families: HashMap<(EntityId, bool), super::definition_order::FamilyId>,
    ordinals: HashMap<EntityId, usize>,
    names: HashMap<EntityId, std::sync::Arc<str>>,
    callees: HashMap<EntityId, Value>,
    table: &'a mut SymbolTable,
    pub globals: Vec<EntityId>,
    // Syntax can require declaration-only storage even when semantic
    // redeclaration bookkeeping retains a definition flag.
    declaration_only_globals: std::collections::HashSet<EntityId>,
    variable_requests: HashMap<EntityId, super::definition_demands::RequestId>,
    variable_owner: Option<EntityId>,
    active_variables: std::collections::HashSet<EntityId>,
    variable_dependencies: std::collections::HashSet<(EntityId, EntityId)>,
    pub instance_variables: std::collections::VecDeque<EntityId>,
    objects: HashMap<EntityId, Option<NameId>>,
    signatures: HashMap<(TypeId, bool), crate::lowir::model::SignatureId>,
    numeric_scanner: crate::types::floating::FloatingScanner,
    pub data: super::data::Objects,
    vtables: vtables::Tables,
    temporary_owner: Option<(EntityId, u32)>,
    local_counts: HashMap<(EntityId, crate::lex::identifiers::NameId), u32>,
    local_discriminators: HashMap<EntityId, u32>,
    tls_wrappers: HashMap<EntityId, SymbolId>,
    variable_names: HashMap<EntityId, NodeId>,
    class_storage: HashMap<EntityId, super::types::ClassStorage>,
    empty_list_constructions: HashMap<EntityId, bool>,
    class_parameters: super::class_parameters::Classes,
    class_labels: specialization_labels::Labels,
    rooted_destructors: std::collections::HashSet<EntityId>,
    inert_destructors: HashMap<EntityId, bool>,
    anonymous_names: super::anonymous_names::Names,
    // A source declaration is shared by every ABI entry running its body.
    local_statics: HashMap<EntityId, super::local_storage::StorageSymbols>,
    persistent_finalizers: Vec<super::shutdown::Finalizer>,
}
impl<'a, 'source> Symbols<'a, 'source> {
    pub(super) fn note_declaration_only_global(&mut self, entity: EntityId) {
        self.declaration_only_globals.insert(entity);
    }
    pub(super) fn declaration_only_global(&self, entity: EntityId) -> bool {
        self.declaration_only_globals.contains(&entity)
    }
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
            placeholders: HashMap::new(),
            unmangled: HashMap::new(),
            scopes: HashMap::new(),
            budget,
            pending: super::definition_demands::Demands::default(),
            initialization: None,
            aggregate_helpers: super::aggregate_helpers::Helpers::default(),
            seen: std::collections::HashSet::new(),
            entries: HashMap::new(),
            entry_objects: HashMap::new(),
            entry_families: HashMap::new(),
            destructor_families: HashMap::new(),
            copy_families: HashMap::new(),
            ordinals: HashMap::new(),
            names: HashMap::new(),
            callees: HashMap::new(),
            table,
            globals: Vec::new(),
            declaration_only_globals: std::collections::HashSet::new(),
            variable_requests: HashMap::new(),
            variable_owner: None,
            active_variables: std::collections::HashSet::new(),
            variable_dependencies: std::collections::HashSet::new(),
            instance_variables: std::collections::VecDeque::new(),
            objects: HashMap::new(),
            data: super::data::Objects::default(),
            vtables: vtables::Tables::default(),
            temporary_owner: None,
            local_counts: HashMap::new(),
            local_discriminators: HashMap::new(),
            tls_wrappers: HashMap::new(),
            variable_names: HashMap::new(),
            class_storage: HashMap::new(),
            empty_list_constructions: HashMap::new(),
            class_parameters: super::class_parameters::Classes::default(),
            class_labels: specialization_labels::Labels::default(),
            rooted_destructors: std::collections::HashSet::new(),
            inert_destructors: HashMap::new(),
            anonymous_names: super::anonymous_names::Names::default(),
            local_statics: HashMap::new(),
            persistent_finalizers: Vec::new(),
            signatures: HashMap::new(),
            numeric_scanner: crate::types::floating::FloatingScanner::default(),
        }
    }
    pub(super) fn publish_finalizers(&mut self, output: &mut Vec<super::shutdown::Finalizer>) {
        output.append(&mut self.persistent_finalizers);
    }
    pub(super) fn persistent_finalizer(
        &mut self,
        program: &mut Program,
        global: SymbolId,
        ty: TypeId,
    ) -> Result<Option<SymbolId>, Error> {
        let graph = self.graph;
        let Some(mut finalizer) =
            super::shutdown::for_type(program, self, graph, global, ty, None)?
        else {
            return Ok(None);
        };
        let name = format!(
            "{}_constructed",
            program.names.get(program.symbol(global).name)
        );
        let flag = self.table.allocate(program, &name, None, true)?;
        program.add_global(
            flag,
            crate::lowir::model::Global {
                name: program.symbol(flag).name,
                ty: Some(crate::lowir::vocabulary::Type::I8),
                metadata: Metadata {
                    binding: Some(crate::lowir::vocabulary::Binding::Internal),
                    ..Metadata::default()
                },
                init: crate::lowir::model::GlobalInit::Zero,
            },
        )?;
        finalizer.constructed = Some(flag);
        finalizer.reference_temporary = true;
        self.persistent_finalizers.push(finalizer);
        Ok(Some(flag))
    }
    pub(super) fn parameter_classes(
        &mut self,
        ty: TypeId,
    ) -> Result<Option<crate::lowir::vocabulary::ObjectClasses>, Error> {
        if matches!(self.graph.lower_type(ty)?, TypeView::Class(class) if !self.graph.lower_direct_class_parameter(class)?)
        {
            return Ok(None);
        }
        self.class_parameters.parameter(self.graph, ty, self.budget)
    }
    pub(super) fn class_result_memory(&mut self, class: EntityId) -> Result<bool, Error> {
        self.class_parameters
            .result_memory(self.graph, class, self.budget)
    }
    pub(super) fn identifier(&self, name: crate::lex::identifiers::NameId) -> Result<&str, Error> {
        std::str::from_utf8(self.cursor.name(name))
            .map_err(|_| Error::Missing("local name spelling"))
    }
    pub(super) fn request_initialization(&mut self, program: &mut Program) -> Result<(), Error> {
        let symbol = if let Some(symbol) = self.table.initializer {
            symbol
        } else {
            let symbol = self.table.allocate(program, "__cppgm_init", None, true)?;
            self.table.initializer = Some(symbol);
            symbol
        };
        self.initialization = Some(symbol);
        Ok(())
    }
    pub(super) fn resolve_runtime_roles(
        &mut self,
        program: &mut Program,
        definitions: &mut super::definition_order::Definitions,
    ) -> Result<(), Error> {
        super::runtime_roles::resolve(program, definitions, &self.table.runtime_roles)
    }
    pub(super) fn bad_array_length(&mut self, program: &mut Program) -> Result<SymbolId, Error> {
        if let Some(symbol) = self.table.bad_array_length {
            return Ok(symbol);
        }
        let symbol = self.table.allocate(
            program,
            "__external_runtime____cxa_throw_bad_array_new_length",
            None,
            true,
        )?;
        super::exception_helpers::declare_bad_array_length(program, symbol)?;
        self.table.bad_array_length = Some(symbol);
        Ok(symbol)
    }
    pub(super) fn terminate_helper(&mut self, program: &mut Program) -> Result<SymbolId, Error> {
        if let Some(helpers) = self.table.exception_helpers {
            return Ok(helpers.helper);
        }
        let helpers = super::exception_helpers::Helpers {
            begin_catch: self.table.allocate(
                program,
                "__external_runtime____cxa_begin_catch",
                None,
                true,
            )?,
            terminate: self.table.allocate(program, "std_terminate", None, true)?,
            helper: self
                .table
                .allocate(program, "cppgm_call_terminate", None, true)?,
        };
        super::exception_helpers::declare(program, helpers)?;
        self.table
            .runtime_roles
            .push(super::runtime_roles::RoleSymbol {
                symbol: helpers.terminate,
                role: crate::lowir::vocabulary::Role::Terminate,
                origin: super::runtime_roles::Origin::Runtime,
            });
        self.table.exception_helpers = Some(helpers);
        self.table.terminate_pending = true;
        Ok(helpers.helper)
    }
    pub(super) fn take_terminate_helper(&mut self) -> Option<super::exception_helpers::Helpers> {
        if !self.table.terminate_pending {
            return None;
        }
        self.table.terminate_pending = false;
        self.table.exception_helpers
    }
    pub(super) fn aggregate_helper_symbol(
        &mut self,
        program: &mut Program,
        class: EntityId,
        count: usize,
        cv: u8,
    ) -> Result<SymbolId, Error> {
        let key = super::aggregate_helpers::Key { class, count, cv };
        if let Some(symbol) = self.aggregate_helpers.get(key) {
            return Ok(symbol);
        }
        let (_, mut scope, _) = self
            .graph
            .lower_nominal_name(self.graph.lower_entity_type(class)?)?;
        let label = self.class_label(self.graph.lower_class_spelling(class)?)?;
        let mut components = vec![format!("{label}__{label}__aggregate")];
        while let (Some(parent), name) = self.graph.lower_presentation_scope(scope) {
            components.push(self.presentation_scope_label(scope, name)?);
            scope = parent;
        }
        components.reverse();
        let mut spelling = components.join("__");
        let shapes = self.aggregate_helpers.shapes(class);
        if shapes != 0 {
            write!(spelling, "__sym{}", shapes + 1)
                .map_err(|_| Error::Missing("helper label formatting"))?;
        }
        let symbol = self.table.allocate(program, &spelling, None, true)?;
        self.aggregate_helpers.insert(key, symbol);
        Ok(symbol)
    }
    pub(super) fn readonly_object(&mut self, mut ty: TypeId) -> Result<bool, Error> {
        if self.graph.lower_cv(ty) != 1
            || matches!(self.graph.lower_type(ty)?, TypeView::Reference { .. })
        {
            return Ok(false);
        }
        while let TypeView::Array { element, .. } = self.graph.lower_type(ty)? {
            ty = element;
        }
        if let TypeView::Class(class) = self.graph.lower_type(ty)? {
            let properties = self.class_storage_properties(class)?;
            Ok(!properties.changeable && !properties.destruction)
        } else {
            Ok(true)
        }
    }
    pub(super) fn require_trivial_storage(&mut self, mut ty: TypeId) -> Result<(), Error> {
        while let TypeView::Array { element, .. } = self.graph.lower_type(ty)? {
            ty = element;
        }
        if let TypeView::Class(class) = self.graph.lower_type(ty)?
            && self.class_storage_properties(class)?.destruction
        {
            return Err(Error::Missing("class storage destruction lowering"));
        }
        Ok(())
    }
    pub(super) fn storage_destructor(&mut self, ty: TypeId) -> Result<Option<EntityId>, Error> {
        self.class_destructor(ty)
    }
    pub(super) fn class_destructor(&mut self, mut ty: TypeId) -> Result<Option<EntityId>, Error> {
        while let TypeView::Array { element, .. } = self.graph.lower_type(ty)? {
            ty = element;
        }
        let TypeView::Class(class) = self.graph.lower_type(ty)? else {
            return Ok(None);
        };
        if !self.graph.lower_complete_object(ty)? {
            return Ok(None);
        }
        if self.graph.lower_implicit_union_destructor(class) {
            return Ok(None);
        }
        if !self.class_storage_properties(class)?.destruction {
            return Ok(None);
        }
        let destructor = self
            .graph
            .lower_destructor_entity(class)
            .ok_or(Error::Missing("implicit recursive destructor lowering"))?;
        Ok(Some(destructor))
    }
    pub(super) fn root_destructor(
        &mut self,
        program: &mut Program,
        entity: EntityId,
    ) -> Result<(), Error> {
        self.demand(entity);
        self.rooted_destructors.insert(entity);
        let symbol = self.function_symbol(program, entity)?;
        program.root_function(symbol);
        Ok(())
    }
    pub(super) fn empty_list_constructs(&mut self, class: EntityId) -> Result<bool, Error> {
        Ok(!matches!(
            self.empty_list_construction(class, 0)?,
            EmptyListConstruction::Complete(false)
        ))
    }
    fn empty_list_construction(
        &mut self,
        class: EntityId,
        depth: u16,
    ) -> Result<EmptyListConstruction, Error> {
        self.check_stack()?;
        if let Some(&known) = self.empty_list_constructions.get(&class) {
            return Ok(EmptyListConstruction::Complete(known));
        }
        // This is the frozen semantic query cap, independent of native stack
        // exhaustion. A conservative cut is never published as a class fact.
        if depth > 64 {
            return Ok(EmptyListConstruction::DepthCut);
        }
        let graph = self.graph;
        let mut result = EmptyListConstruction::Complete(false);
        for &member in graph.lower_class_member_entities(class) {
            let mut ty = graph.lower_entity_type(member)?;
            while let TypeView::Array { element, .. } = graph.lower_type(ty)? {
                ty = element;
            }
            let TypeView::Class(child) = graph.lower_type(ty)? else {
                continue;
            };
            result = if graph.lower_aggregate_class(child) {
                self.empty_list_construction(child, depth + 1)?
            } else {
                EmptyListConstruction::Complete(
                    graph
                        .lower_ready_default_constructor(child)
                        .is_none_or(|constructor| !graph.lower_constructor_trivial(constructor)),
                )
            };
            if !matches!(result, EmptyListConstruction::Complete(false)) {
                break;
            }
        }
        if let EmptyListConstruction::Complete(constructs) = result {
            self.empty_list_constructions.insert(class, constructs);
        }
        Ok(result)
    }
    pub(super) fn zeroed_by_value(&mut self, ty: TypeId) -> Result<bool, Error> {
        if let TypeView::Class(class) = self.graph.lower_type(ty)? {
            return Ok(self.class_storage_properties(class)?.value_zero);
        }
        Ok(false)
    }
    fn class_storage_properties(
        &mut self,
        class: EntityId,
    ) -> Result<super::types::ClassStorage, Error> {
        self.check_stack()?;
        if let Some(&properties) = self.class_storage.get(&class) {
            return Ok(properties);
        }
        let graph = self.graph;
        let mut properties = super::types::ClassStorage {
            changeable: false,
            destruction: graph.lower_nontrivial_destructor(class)?,
            value_zero: graph.lower_aggregate_class(class) && !graph.lower_polymorphic(class),
        };
        for base in graph.lower_class_bases(class) {
            let child = self.class_storage_properties(base)?;
            properties.changeable |= child.changeable;
            properties.destruction |= child.destruction;
            properties.value_zero &= child.value_zero;
        }
        for &member in graph.lower_class_member_entities(class) {
            properties.changeable |= graph.lower_mutable_member(member);
            let mut ty = graph.lower_entity_type(member)?;
            if matches!(graph.lower_type(ty)?, TypeView::Reference { .. }) {
                continue;
            }
            properties.changeable |= graph.lower_cv(ty) & 2 != 0;
            while let TypeView::Array { element, .. } = graph.lower_type(ty)? {
                ty = element;
            }
            if let TypeView::Class(class) = graph.lower_type(ty)? {
                let child = self.class_storage_properties(class)?;
                properties.changeable |= child.changeable;
                properties.destruction |= child.destruction;
                properties.value_zero &= child.value_zero;
            }
        }
        self.class_storage.insert(class, properties);
        Ok(properties)
    }
    pub(super) fn literal_bytes(&self, literal: ast::Literal) -> &[u8] {
        self.cursor.spellings.get(literal.data)
    }
    fn source_function_spelling(&self, name: FunctionName) -> Result<String, Error> {
        match name {
            FunctionName::Identifier(name) => Ok(self.identifier(name)?.to_owned()),
            FunctionName::Literal(suffix) => {
                Ok(format!("operator\"\" {}", self.identifier(suffix)?))
            }
            _ => {
                Err(crate::sema::Error::Invariant("C linkage function without source name").into())
            }
        }
    }
    pub(super) fn literal_spelling(&self, literal: ast::Literal) -> &[u8] {
        self.cursor.spellings.get(literal.spelling)
    }
    pub(super) fn numeric_literal_value(
        &mut self,
        literal: ast::Literal,
        digits: usize,
    ) -> Result<Option<crate::types::floating::ScalarBytes>, Error> {
        let digits = self
            .cursor
            .spellings
            .get(literal.spelling)
            .get(..digits)
            .ok_or(crate::sema::Error::Invariant(
                "numeric literal digits outside spelling",
            ))?;
        self.numeric_scanner
            .scan(digits, crate::types::FundamentalType::LongDouble)
            .map_err(|_| crate::lowir::Error::Limit.into())
    }
    fn presentation_scope_label(
        &mut self,
        scope: ScopeId,
        name: Option<crate::lex::identifiers::NameId>,
    ) -> Result<String, Error> {
        if let Some(class) = self.graph.lower_scope_class(scope) {
            return self.specialized_class_label(class);
        }
        if let Some((class, _, _)) = self.graph.lower_anonymous_scope(scope) {
            return self.class_label(self.graph.lower_class_spelling(class)?);
        }
        Ok(name
            .map_or(Ok("_GLOBAL__N_1"), |name| self.identifier(name))?
            .to_owned())
    }
    fn class_label(&self, label: crate::sema::lowering::ClassSpelling) -> Result<String, Error> {
        use crate::sema::lowering::ClassSpelling;
        Ok(match label {
            ClassSpelling::Name(name) => self.identifier(name)?.to_owned(),
            ClassSpelling::Local(ordinal) => format!("__local_type{ordinal}"),
            ClassSpelling::Anonymous(tokens) => {
                format!("__anonymous_union_type__{}_{}", tokens.begin, tokens.end)
            }
        })
    }
    fn function_label(
        &self,
        name: FunctionName,
        label: Option<ast::Name>,
    ) -> Result<String, Error> {
        let mut bytes = Vec::new();
        if let FunctionName::Destructor(name) = name {
            bytes.push(b'~');
            if let Some(name) = name {
                bytes.extend_from_slice(self.cursor.name(name));
            }
        } else if let FunctionName::Identifier(name) = name {
            bytes.extend_from_slice(self.cursor.name(name));
        } else if let FunctionName::Constructor(spelling) = name {
            bytes.extend_from_slice(self.class_label(spelling)?.as_bytes());
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
                FunctionName::Identifier(_)
                | FunctionName::Constructor(_)
                | FunctionName::Destructor(_)
                | FunctionName::Conversion(_) => {
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
    fn function_terminal(
        &mut self,
        name: FunctionName,
        ty: TypeId,
        member: bool,
    ) -> Result<NodeId, Error> {
        let node = match name {
            FunctionName::Constructor(_) => Node::Terminal {
                kind: abi::vocabulary::Terminal::ConstructorComplete,
                inherited: None,
            },
            FunctionName::Destructor(_) => Node::Terminal {
                kind: abi::vocabulary::Terminal::DestructorComplete,
                inherited: None,
            },
            FunctionName::Identifier(name) => {
                return Ok(self.abi.string(self.cursor.name(name))?);
            }
            FunctionName::Conversion(target) => Node::Conversion(self.abi_type(target)?),
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
                    kind: terminal.for_operands(parameters.len() + usize::from(member)),
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
                self.identifier(self.graph.lower_variable(entity)?.named()?)?
            )
        };
        self.named_reference_temporary(program, &spelling)
    }
    pub(super) fn dynamic_temporary_symbol(
        &mut self,
        program: &mut Program,
        ty: TypeId,
    ) -> Result<(SymbolId, Option<NameId>), Error> {
        let spelling = if let Some((owner, sequence)) = self.temporary_owner {
            let facts = self.graph.lower_variable(owner)?;
            if !matches!(self.graph.lower_type(facts.ty)?, TypeView::Reference { .. })
                && matches!(self.graph.lower_type(ty)?, TypeView::Array { .. })
            {
                self.table.initializer_list_backings = self
                    .table
                    .initializer_list_backings
                    .checked_add(1)
                    .ok_or(crate::lowir::Error::Limit)?;
                format!(
                    "__cppgm_initlist_backing_{}",
                    self.table.initializer_list_backings
                )
            } else {
                let symbol = self.variable_symbol(program, owner)?;
                format!(
                    "__cppgm_ref_temporary__{}_{}",
                    program.names.get(program.symbol(symbol).name),
                    sequence
                )
            }
        } else {
            self.table.reference_temporaries = self
                .table
                .reference_temporaries
                .checked_add(1)
                .ok_or(crate::lowir::Error::Limit)?;
            format!("__ref_tmp__{}", self.table.reference_temporaries)
        };
        self.named_reference_temporary(program, &spelling)
    }
    fn named_reference_temporary(
        &mut self,
        program: &mut Program,
        spelling: &str,
    ) -> Result<(SymbolId, Option<NameId>), Error> {
        let Some((owner, sequence)) = self.temporary_owner else {
            return Ok((self.table.allocate(program, spelling, None, true)?, None));
        };
        self.temporary_owner = Some((
            owner,
            sequence.checked_add(1).ok_or(crate::lowir::Error::Limit)?,
        ));
        let facts = self.graph.lower_variable(owner)?;
        let parent = self.scope(facts.home)?;
        let terminal = self.abi.string(self.cursor.name(facts.named()?))?;
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
        let symbol = self.table.allocate(program, spelling, Some(object), true)?;
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
        if let Some(class) = self.graph.lower_scope_class(scope) {
            let id = Some(self.abi_type(self.graph.lower_entity_type(class)?)?);
            self.scopes.insert(scope, id);
            return Ok(id);
        }
        if let Some(function) = self.graph.lower_local_scope(scope) {
            let facts = self.graph.lower_function(function)?;
            let node = if facts.c_linkage
                || matches!(facts.name,
                FunctionName::Identifier(name) if self.cursor.name(name) == b"main")
            {
                let spelling = self.source_function_spelling(facts.name)?;
                let raw = format!("Z{}{spelling}E", spelling.len());
                Node::RawFunctionScope(self.abi.string(raw.as_bytes())?)
            } else {
                Node::FunctionScope {
                    encoding: self.function_encoding(function)?,
                    default_argument: 0,
                }
            };
            let id = Some(self.abi.intern(node, &[])?);
            self.scopes.insert(scope, id);
            return Ok(id);
        }
        let (parent, name, namespace) = self.graph.lower_abi_scope(scope)?;
        let id = if let Some(parent) = parent {
            let parent = self.scope(parent)?;
            if namespace
                && parent.is_none()
                && name.is_some_and(|name| self.cursor.name(name) == b"std")
            {
                let id = Some(self.abi.intern(Node::Std, &[])?);
                self.scopes.insert(scope, id);
                return Ok(id);
            }
            let terminal = if let Some(terminal) =
                self.anonymous_names
                    .scope_terminal(self.graph, &mut self.abi, scope)?
            {
                terminal
            } else {
                self.abi
                    .string(name.map_or(&b"_GLOBAL__N_1"[..], |name| self.cursor.name(name)))?
            };
            let occurrence = if let Some((class, function, declared)) =
                self.graph.lower_local_class_scope(scope)
            {
                self.local_discriminator(class, function, name, declared)?
            } else {
                0
            };
            Some(
                self.abi
                    .intern(
                        Node::Name {
                            parent,
                            terminal,
                            internal: false,
                            occurrence,
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
    fn local_discriminator(
        &mut self,
        entity: EntityId,
        function: EntityId,
        name: Option<crate::lex::identifiers::NameId>,
        declared: u32,
    ) -> Result<u32, Error> {
        if self.graph.lower_function_visible_locals(function) {
            return Ok(declared);
        }
        if let Some(&known) = self.local_discriminators.get(&entity) {
            return Ok(known);
        }
        let Some(name) = name else {
            return Ok(declared);
        };
        let count = self.local_counts.entry((function, name)).or_default();
        let occurrence = *count;
        *count = count.checked_add(1).ok_or(crate::lowir::Error::Limit)?;
        self.local_discriminators.insert(entity, occurrence);
        Ok(occurrence)
    }
    pub(super) fn name_scopes(&mut self, scopes: &[ScopeId]) -> Result<(), Error> {
        for &scope in scopes {
            self.scope(scope)?;
        }
        Ok(())
    }
    fn name_local_classes(&mut self, function: EntityId) -> Result<(), Error> {
        if self.graph.lower_function_visible_locals(function) {
            return Ok(());
        }
        let graph = self.graph;
        for &class in graph.lower_deferred_local_classes(function) {
            if let Some(scope) = graph.lower_local_class_definition_scope(class) {
                self.scope(scope)?;
            }
        }
        Ok(())
    }
    fn abi_type(&mut self, ty: TypeId) -> Result<NodeId, Error> {
        self.check_stack()?;
        if let Some(&id) = self.types.get(&ty) {
            return Ok(id);
        }
        let node = if let Some(index) = self.graph.lower_template_parameter(ty) {
            if self.graph.lower_value_argument(ty) {
                let parameter = self.abi.intern(
                    Node::TemplateParam {
                        index,
                        substitutable: false,
                    },
                    &[],
                )?;
                Node::ExpressionArgument(parameter)
            } else {
                Node::TemplateParam {
                    index,
                    substitutable: true,
                }
            }
        } else if let Some((owner, member)) = self.graph.lower_member_pointer_type(ty) {
            Node::MemberPointer {
                owner: self.abi_type(owner)?,
                member: self.abi_type(member)?,
            }
        } else if let Some((qualifier, name)) = self.graph.lower_dependent_member(ty) {
            Node::Name {
                parent: Some(self.abi_type(qualifier)?),
                terminal: self.abi.string(self.cursor.name(name))?,
                internal: false,
                occurrence: 0,
            }
        } else if let Some((operand, _)) = self.graph.lower_template_operand(ty) {
            let expression = self
                .placeholder_node(ty)?
                .ok_or(Error::Missing("unmangled dependent ABI type"))?;
            let kind = self.tree.expressions.get(operand).kind;
            if self.graph.lower_value_argument(ty) {
                Node::ExpressionArgument(expression)
            } else {
                Node::Decltype {
                    expression,
                    id_expression: matches!(
                        kind,
                        ast::ExprKind::Id(_) | ast::ExprKind::Member { .. }
                    ),
                }
            }
        } else {
            match self.graph.lower_type(ty)? {
                TypeView::Value { ty, bits } => Node::Literal {
                    ty: self.abi_type(ty)?,
                    bits,
                },
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
                    modifier: if let Some(operand) = self.graph.lower_array_operand(ty) {
                        let (_, words) = self
                            .graph
                            .lower_template_operand(operand)
                            .ok_or(Error::Missing("dependent array bound ABI operand"))?;
                        Modifier::ExpressionArray(self.placeholder_expression(words)?)
                    } else if bound == 0 {
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
                TypeView::Enum(_) | TypeView::Class(_) => {
                    let (entity, home, name) = self.graph.lower_nominal_name(ty)?;
                    let parent =
                        if let Some((qualifier, _)) = self.graph.lower_dependent_template(entity) {
                            Some(self.abi_type(qualifier)?)
                        } else {
                            self.scope(home)?
                        };
                    let terminal = if let Some(name) = name {
                        self.abi.string(self.cursor.name(name))?
                    } else {
                        self.anonymous_names
                            .terminal(self.graph, &mut self.abi, entity)?
                    };
                    let occurrence = if let TypeView::Class(class) = self.graph.lower_type(ty)?
                        && let Some((function, declared)) = self.graph.lower_local_type(class)
                    {
                        self.local_discriminator(class, function, name, declared)?
                    } else {
                        0
                    };
                    let name = self.abi.intern(
                        Node::Name {
                            parent,
                            terminal,
                            internal: false,
                            occurrence,
                        },
                        &[],
                    )?;
                    let id = if let Some(arguments) = self.graph.lower_class_arguments(entity) {
                        let mut nodes = Vec::with_capacity(arguments.len());
                        for argument in arguments {
                            nodes.push(self.abi_type(argument)?);
                        }
                        self.abi.intern(Node::Template(name), &nodes)?
                    } else {
                        name
                    };
                    return self.finish_abi_type(ty, id);
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
                        qualifiers: self.graph.lower_function_type_qualifiers(ty),
                        variadic,
                    };
                    let id = self.abi.intern(node, &args)?;
                    self.types.insert(ty, id);
                    return Ok(id);
                }
                TypeView::Unsupported => return Err(Error::Missing("aggregate ABI type lowering")),
            }
        };
        let id = self.abi.intern(node, &[]).map_err(Error::from)?;
        self.finish_abi_type(ty, id)
    }
    fn finish_abi_type(&mut self, ty: TypeId, mut id: NodeId) -> Result<NodeId, Error> {
        if !matches!(self.graph.lower_type(ty)?, TypeView::Array { .. }) {
            let mut cv = Qualifiers::default();
            if self.graph.lower_cv(ty) & 1 != 0 {
                cv = cv.union(Qualifiers::CONST);
            }
            if self.graph.lower_cv(ty) & 2 != 0 {
                cv = cv.union(Qualifiers::VOLATILE);
            }
            if !cv.is_empty() {
                id = self.abi.intern(
                    Node::Modified {
                        modifier: Modifier::Qualified(cv),
                        operand: id,
                    },
                    &[],
                )?;
            }
        }
        self.types.insert(ty, id);
        Ok(id)
    }
    pub(super) fn demand_destruction(&mut self, entity: EntityId) {
        if self.graph.lower_virtual_slot(entity).is_some() {
            self.demand_entry(super::construction::Entry::DestructorBase(entity));
            self.demand_entry(super::construction::Entry::DestructorDeleting(entity));
        }
        self.demand(entity);
    }
    pub(super) fn demand(&mut self, entity: EntityId) {
        let entity = self.graph.lower_function_entity(entity);
        let entry = if let Some(id) = self.graph.lower_construction(entity) {
            super::construction::Entry::Constructor(id, super::construction::Variant::Complete)
        } else {
            super::construction::Entry::Function(entity)
        };
        self.demand_entry(entry);
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
        if let Some(
            role @ (crate::lowir::vocabulary::Role::AllocateMemory
            | crate::lowir::vocabulary::Role::FreeMemory
            | crate::lowir::vocabulary::Role::Terminate),
        ) = metadata.role
        {
            self.table
                .runtime_roles
                .push(super::runtime_roles::RoleSymbol {
                    symbol,
                    role,
                    origin: if facts.allocation.is_some_and(|allocation| allocation.array)
                        || matches!(
                            facts.builtin,
                            Some(crate::sema::lowering::BuiltinFunction::Abort)
                        ) {
                        super::runtime_roles::Origin::Secondary
                    } else {
                        super::runtime_roles::Origin::Primary
                    },
                });
        }
        Ok(symbol)
    }
    pub(super) fn name(&mut self, entity: EntityId) -> Result<std::sync::Arc<str>, Error> {
        let entity = self.graph.lower_function_entity(entity);
        if let Some(name) = self.names.get(&entity) {
            return Ok(std::sync::Arc::clone(name));
        }
        if !self.ordinals.contains_key(&entity) {
            let facts = self.graph.lower_function(entity)?;
            if matches!(facts.name, FunctionName::Constructor(_)) {
                let class = facts
                    .object
                    .ok_or(Error::Semantic(crate::sema::Error::Invariant(
                        "constructor without owner",
                    )))?;
                for (index, overload) in self.graph.lower_constructor_overloads(class).enumerate() {
                    self.ordinals.insert(overload, index + 1);
                }
            } else if matches!(facts.name, FunctionName::Conversion(_)) {
                let class = facts
                    .object
                    .ok_or(crate::sema::Error::Invariant("conversion without owner"))?;
                for (index, overload) in self.graph.lower_conversion_overloads(class).enumerate() {
                    self.ordinals.insert(overload, index + 1);
                }
            } else if !matches!(facts.name, FunctionName::Destructor(_)) {
                for (index, overload) in self.graph.lower_overloads(entity)?.enumerate() {
                    self.ordinals.insert(overload, index + 1);
                }
            }
        }
        let facts = self.graph.lower_function(entity)?;
        let own = match facts.builtin {
            Some(crate::sema::lowering::BuiltinFunction::Memcmp) => "memcmp".to_owned(),
            Some(crate::sema::lowering::BuiltinFunction::Memset) => "memset".to_owned(),
            _ if facts.c_linkage && !facts.internal => self.source_function_spelling(facts.name)?,
            _ => self.function_label(facts.name, facts.label)?,
        };
        let mut components = vec![own];
        if !facts.c_linkage || facts.internal {
            let mut scope = facts.home;
            while let (Some(parent), name) = self.graph.lower_presentation_scope(scope) {
                components.push(self.presentation_scope_label(scope, name)?);
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
        let signature = super::function::signature(program, self, self.graph, ty, nothrow)?;
        self.signatures.insert((ty, nothrow), signature);
        Ok(signature)
    }
    pub(super) fn variable_symbol(
        &mut self,
        program: &mut Program,
        entity: EntityId,
    ) -> Result<SymbolId, Error> {
        if let Some(&Value::Symbol(symbol)) = self.callees.get(&entity) {
            self.request_variable_dependency(program, entity, symbol);
            return Ok(symbol);
        }
        let facts = self.graph.lower_variable(entity)?;
        let mut components = vec![if let Some(name) = facts.name {
            self.identifier(name)?.to_owned()
        } else {
            "__anonymous_union_object".to_owned()
        }];
        let mut scope = facts.home;
        while let (Some(parent), name) = self.graph.lower_presentation_scope(scope) {
            components.push(self.presentation_scope_label(scope, name)?);
            scope = parent;
        }
        components.reverse();
        let spelling = components.join("__");
        // Named variables publish their ABI identity even when their external
        // spelling is emitted directly, preserving the original demand order.
        let named_identity = facts
            .name
            .map(|_| self.variable_abi_name(entity))
            .transpose()?;
        let object =
            if facts.name.is_none() && self.graph.lower_first_named_member(facts.ty).is_none() {
                None
            } else if facts.name.is_some()
                && (facts.c_linkage
                    || self.graph.lower_presentation_scope(facts.home).0.is_none()
                        && !facts.internal
                        && self.graph.lower_variable_arguments(entity).is_none())
            {
                Some(
                    program
                        .names
                        .intern_bytes(self.cursor.name(facts.named()?))?,
                )
            } else {
                let name = if let Some(name) = named_identity {
                    name
                } else {
                    self.variable_abi_name(entity)?
                };
                let mut bytes = Vec::new();
                abi::encoder::Encoder::encode(
                    &self.abi,
                    abi::encoder::Target::Entity(name),
                    &mut bytes,
                    self.budget,
                )?;
                Some(program.names.intern_bytes(&bytes)?)
            };
        let local_object = if facts.locality == crate::sema::lowering::Locality::Unit
            && (facts.c_linkage
                || object.is_some_and(|object| self.table.objects.contains_key(&object)))
        {
            None
        } else {
            object
        };
        let symbol = self.table.allocate(
            program,
            &spelling,
            object,
            facts.locality == crate::sema::lowering::Locality::Unit,
        )?;
        self.callees.insert(entity, Value::Symbol(symbol));
        self.objects.insert(entity, local_object);
        self.globals.push(entity);
        if facts.emission == crate::sema::lowering::Emission::Demanded
            && self.graph.lower_instance_initializer(entity).is_some()
        {
            self.instance_variables.push_back(entity);
            let request = self.pending.variable();
            self.variable_requests.insert(entity, request);
            if let Some(owner) = self.variable_owner {
                self.variable_dependencies.insert((owner, entity));
            }
        }
        Ok(symbol)
    }
    // C++ variable_symbol defines an initializer's dependencies immediately,
    // including a symbol an earlier expression already made pending. Promote
    // that dependency into the current definition's isolated queue once; active
    // definitions keep cyclic address initializers from reentering themselves.
    fn request_variable_dependency(
        &mut self,
        program: &Program,
        entity: EntityId,
        symbol: SymbolId,
    ) {
        if let Some(owner) = self.variable_owner
            && self.variable_requests.contains_key(&entity)
            && !self.active_variables.contains(&entity)
            && program
                .global(program.symbol(symbol))
                .is_none_or(|g| matches!(g.init, crate::lowir::model::GlobalInit::Declaration))
            && self.variable_dependencies.insert((owner, entity))
        {
            // Its old reservation has no children until definition begins.
            // Fill a reservation under this owner so initializer-demanded
            // functions also precede the caller's later requests.
            let request = self.pending.variable();
            self.variable_requests.insert(entity, request);
            self.instance_variables.push_back(entity);
        }
    }
    pub(super) fn begin_variable_definition(&mut self, entity: EntityId) -> Option<EntityId> {
        self.active_variables.insert(entity);
        self.variable_owner.replace(entity)
    }
    pub(super) fn end_variable_definition(&mut self, entity: EntityId, previous: Option<EntityId>) {
        self.active_variables.remove(&entity);
        self.variable_owner = previous;
    }
    pub(super) fn variable_request(
        &self,
        entity: EntityId,
    ) -> Option<super::definition_demands::RequestId> {
        self.variable_requests.get(&entity).copied()
    }
    pub(super) fn budget(&self) -> StackBudget {
        self.budget
    }
    pub(super) fn instance_constructed(
        &mut self,
        program: &mut Program,
        entity: EntityId,
    ) -> Result<SymbolId, Error> {
        let global = self.variable_symbol(program, entity)?;
        let base = program.names.get(program.symbol(global).name).to_owned();
        let flag = self
            .table
            .allocate(program, &format!("{base}_constructed"), None, true)?;
        program.add_global(
            flag,
            crate::lowir::model::Global {
                name: program.symbol(flag).name,
                ty: Some(crate::lowir::vocabulary::Type::I8),
                metadata: Metadata {
                    binding: Some(crate::lowir::vocabulary::Binding::Internal),
                    ..Metadata::default()
                },
                init: crate::lowir::model::GlobalInit::Zero,
            },
        )?;
        Ok(flag)
    }
    pub(super) fn instance_guard(
        &mut self,
        program: &mut Program,
        entity: EntityId,
    ) -> Result<SymbolId, Error> {
        let global = self.variable_symbol(program, entity)?;
        let base = program.names.get(program.symbol(global).name).to_owned();
        let name = self.variable_abi_name(entity)?;
        let mut bytes = Vec::new();
        abi::encoder::Encoder::encode(
            &self.abi,
            abi::encoder::Target::GuardVariable(name),
            &mut bytes,
            self.budget,
        )?;
        let guard_name = program.names.intern_bytes(&bytes)?;
        let guard = self.table.allocate(
            program,
            &format!("__cppgm_guard__{base}"),
            Some(guard_name),
            false,
        )?;
        program.add_global(
            guard,
            crate::lowir::model::Global {
                name: program.symbol(guard).name,
                ty: Some(crate::lowir::vocabulary::Type::I64),
                metadata: Metadata {
                    binding: Some(crate::lowir::vocabulary::Binding::Weak),
                    object: Some(guard_name),
                    ..Metadata::default()
                },
                init: crate::lowir::model::GlobalInit::Zero,
            },
        )?;
        Ok(guard)
    }
    pub(super) fn variable_abi_name(&mut self, entity: EntityId) -> Result<NodeId, Error> {
        if let Some(&name) = self.variable_names.get(&entity) {
            return Ok(name);
        }
        let facts = self.graph.lower_variable(entity)?;
        let parent = self.scope(facts.home)?;
        let identifier = facts
            .name
            .or_else(|| self.graph.lower_first_named_member(facts.ty))
            .ok_or(crate::sema::Error::Invariant(
                "anonymous object without named member",
            ))?;
        let terminal = self.abi.string(self.cursor.name(identifier))?;
        let mut name = self.abi.intern(
            Node::Name {
                parent,
                terminal,
                internal: facts.name.is_some()
                    && facts.internal
                    && !self.graph.lower_anonymous_namespace(facts.home),
                occurrence: 0,
            },
            &[],
        )?;
        if let Some(arguments) = self.graph.lower_variable_arguments(entity) {
            let mut nodes = Vec::with_capacity(arguments.len());
            for &argument in arguments {
                nodes.push(self.abi_type(argument)?);
            }
            name = self.abi.intern(Node::Template(name), &nodes)?;
        }
        self.variable_names.insert(entity, name);
        Ok(name)
    }
    pub(super) fn tls_wrapper(
        &mut self,
        program: &mut Program,
        entity: EntityId,
        global: SymbolId,
    ) -> Result<Option<SymbolId>, Error> {
        if let Some(&wrapper) = self.tls_wrappers.get(&entity) {
            return Ok(Some(wrapper));
        }
        if program.symbol(global).declared()
            || self.graph.lower_variable(entity)?.definition.is_none()
        {
            return Ok(None);
        }
        self.tls_wrapper_symbol(program, entity, global).map(Some)
    }
    pub(super) fn tls_wrapper_symbol(
        &mut self,
        program: &mut Program,
        entity: EntityId,
        global: SymbolId,
    ) -> Result<SymbolId, Error> {
        if let Some(&wrapper) = self.tls_wrappers.get(&entity) {
            return Ok(wrapper);
        }
        let spelling = format!(
            "__cppgm_tls_wrapper__{}",
            program.names.get(program.symbol(global).name)
        );
        let name = self.variable_abi_name(entity)?;
        let mut bytes = Vec::new();
        abi::encoder::Encoder::encode(
            &self.abi,
            abi::encoder::Target::TlsWrapper(name),
            &mut bytes,
            self.budget,
        )?;
        let object = program.names.intern_bytes(&bytes)?;
        let internal =
            self.graph.lower_variable(entity)?.locality == crate::sema::lowering::Locality::Unit;
        let local_object =
            (!internal || !self.table.objects.contains_key(&object)).then_some(object);
        let wrapper = self
            .table
            .allocate(program, &spelling, Some(object), internal)?;
        let object = local_object;
        self.tls_wrappers.insert(entity, wrapper);
        // The wrapper's ABI identity is retained on its own symbol declaration.
        let signature = program.add_signature(crate::lowir::model::Signature {
            parameters: Vec::new(),
            result: crate::lowir::vocabulary::Type::Ptr,
            boundary: crate::lowir::model::Boundary::default(),
        })?;
        let name = program.symbol(wrapper).name;
        program.add(
            wrapper,
            crate::lowir::model::Function {
                name,
                signature,
                metadata: Metadata {
                    binding: Some(if internal {
                        crate::lowir::vocabulary::Binding::Internal
                    } else {
                        crate::lowir::vocabulary::Binding::Weak
                    }),
                    object,
                    tls_for: Some(global),
                    ..Metadata::default()
                },
                body: None,
                location: None,
            },
        )?;
        Ok(wrapper)
    }
    pub(super) fn tls_initialization_symbols(
        &mut self,
        program: &mut Program,
        entity: EntityId,
    ) -> Result<(SymbolId, SymbolId), Error> {
        use crate::lowir::{
            model::{Global, GlobalInit},
            vocabulary::{Binding, Storage},
        };
        let global = self.variable_symbol(program, entity)?;
        let base = program.names.get(program.symbol(global).name).to_owned();
        let guard =
            self.table
                .allocate(program, &format!("__cppgm_tls_guard__{base}"), None, true)?;
        program.add_global(
            guard,
            Global {
                name: program.symbol(guard).name,
                ty: Some(crate::lowir::vocabulary::Type::I64),
                init: GlobalInit::Zero,
                metadata: Metadata {
                    binding: Some(Binding::Internal),
                    storage: Some(Storage::ThreadLocal),
                    ..Metadata::default()
                },
            },
        )?;
        let init =
            self.table
                .allocate(program, &format!("__cppgm_tls_init__{base}"), None, true)?;
        Ok((guard, init))
    }
    pub(super) fn emit_tls_wrapper(
        &mut self,
        program: &mut Program,
        entity: EntityId,
        initializer: Option<SymbolId>,
    ) -> Result<bool, Error> {
        let Some(&wrapper) = self.tls_wrappers.get(&entity) else {
            return Ok(false);
        };
        let global = self.variable_symbol(program, entity)?;
        let name = program.names.get(program.symbol(wrapper).name).to_owned();
        let metadata = program
            .function(program.symbol(wrapper))
            .ok_or(Error::Semantic(crate::sema::Error::Invariant(
                "TLS wrapper without declaration",
            )))?
            .metadata;
        let mut builder = crate::lowir::builder::FunctionBuilder::new(
            program,
            &name,
            &[],
            crate::lowir::vocabulary::Type::Ptr,
        )?;
        builder.metadata(metadata);
        builder.block("entry")?;
        if let Some(initializer) = initializer {
            let arguments = builder.arguments(&[])?;
            builder.effect(crate::lowir::model::InstructionKind::Call(
                crate::lowir::model::Call {
                    result: crate::lowir::vocabulary::Type::Void,
                    callee: Value::Symbol(initializer),
                    arguments,
                    signature: None,
                    copy_elision: false,
                },
            ))?;
        }
        let address = builder.generated(crate::lowir::model::InstructionKind::Addr(
            Value::Symbol(global),
        ))?;
        builder.effect(crate::lowir::model::InstructionKind::Return(
            crate::lowir::vocabulary::Type::Ptr,
            Some(address),
        ))?;
        builder.finish()?;
        Ok(true)
    }
    pub(super) fn existing_local_storage(&self, entity: EntityId) -> Option<Value> {
        self.local_statics
            .get(&entity)
            .map(|symbols| Value::Symbol(symbols.object))
    }
    pub(super) fn local_storage_symbols(
        &mut self,
        program: &mut Program,
        entity: EntityId,
        function: EntityId,
        ordinal: u32,
        dynamic: bool,
    ) -> Result<super::local_storage::StorageSymbols, Error> {
        use crate::lowir::vocabulary::{Binding, Storage};
        if let Some(&symbols) = self.local_statics.get(&entity) {
            return Ok(symbols);
        }
        let facts = self.graph.lower_variable(entity)?;
        let owner = self.function_symbol(program, function)?;
        let owner_name = program.names.get(program.symbol(owner).name).to_owned();
        let weak = self.metadata(program, function, &owner_name)?.binding == Some(Binding::Weak);
        if facts.name.is_none() {
            let spelling = format!("__local_static__{owner_name}__decl{ordinal}__object");
            let object = self.table.allocate(program, &spelling, None, !weak)?;
            let metadata = Metadata {
                binding: Some(if weak {
                    Binding::Weak
                } else {
                    Binding::Internal
                }),
                ..Metadata::default()
            };
            let guard = if dynamic {
                Some((
                    self.table
                        .allocate(program, &format!("{spelling}__guard"), None, !weak)?,
                    metadata,
                ))
            } else {
                None
            };
            let symbols = super::local_storage::StorageSymbols {
                object,
                metadata,
                guard,
            };
            self.callees.insert(entity, Value::Symbol(object));
            self.local_statics.insert(entity, symbols);
            return Ok(symbols);
        }
        let name = self.identifier(facts.named()?)?;
        let spelling = format!("__local_static__{owner_name}__decl{ordinal}__{name}");
        let encoding = self.function_encoding(function)?;
        let parent = self.abi.intern(
            Node::FunctionScope {
                encoding,
                default_argument: 0,
            },
            &[],
        )?;
        let terminal = self.abi.string(self.cursor.name(facts.named()?))?;
        let count = self
            .local_counts
            .entry((function, facts.named()?))
            .or_default();
        let occurrence = *count;
        *count = count.checked_add(1).ok_or(crate::lowir::Error::Limit)?;
        let name = self.abi.intern(
            Node::Name {
                parent: Some(parent),
                terminal,
                internal: false,
                occurrence,
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
        let object = program.names.intern_bytes(&bytes)?;
        let local_object = (weak || !self.table.objects.contains_key(&object)).then_some(object);
        let symbol = self
            .table
            .allocate(program, &spelling, Some(object), !weak)?;
        self.callees.insert(entity, Value::Symbol(symbol));
        let metadata = Metadata {
            binding: Some(if weak {
                Binding::Weak
            } else {
                Binding::Internal
            }),
            storage: facts.thread_local.then_some(Storage::ThreadLocal),
            object: local_object,
            ..Metadata::default()
        };
        let guard = if dynamic {
            bytes.clear();
            abi::encoder::Encoder::encode(
                &self.abi,
                abi::encoder::Target::GuardVariable(name),
                &mut bytes,
                self.budget,
            )?;
            let object = program.names.intern_bytes(&bytes)?;
            let local_object =
                (weak || !self.table.objects.contains_key(&object)).then_some(object);
            let guard =
                self.table
                    .allocate(program, &format!("{spelling}__guard"), Some(object), !weak)?;
            Some((
                guard,
                Metadata {
                    object: local_object,
                    ..metadata
                },
            ))
        } else {
            None
        };
        let symbols = super::local_storage::StorageSymbols {
            object: symbol,
            guard,
            metadata,
        };
        self.local_statics.insert(entity, symbols);
        Ok(symbols)
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
            .filter(|&object| {
                facts.locality == crate::sema::lowering::Locality::Unit
                    || object != program.symbol(symbol).name
            });
        Ok(Metadata {
            linkage: if facts.c_linkage {
                Linkage::C
            } else {
                Linkage::Default
            },
            binding: Some(if facts.locality == crate::sema::lowering::Locality::Unit {
                crate::lowir::vocabulary::Binding::Internal
            } else if facts.emission == crate::sema::lowering::Emission::Demanded
                || self.graph.lower_variable_arguments(entity).is_some()
            {
                crate::lowir::vocabulary::Binding::Weak
            } else {
                crate::lowir::vocabulary::Binding::Strong
            }),
            object,
            storage: facts
                .thread_local
                .then_some(crate::lowir::vocabulary::Storage::ThreadLocal),
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
        if name == "main" && self.graph.lower_presentation_scope(facts.home).0.is_none() {
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
            flags: if self.graph.lower_inline_hint(entity) {
                16
            } else {
                0
            },
            ..Metadata::default()
        };
        if self.graph.lower_no_inline(entity) {
            metadata.flags |= 32;
        }
        if self.rooted_destructors.contains(&entity) {
            metadata.flags |= 4;
        }
        if self.graph.lower_function_local_class(entity) {
            metadata.flags |= 2;
        }
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
                B::Abort => {
                    metadata.role = Some(Role::Terminate);
                    metadata.object = Some(program.names.intern("abort")?);
                }
                B::Memcmp | B::Memset | B::Unreachable | B::Floating(_) => {
                    metadata.linkage = Linkage::C;
                }
            }
            return Ok(metadata);
        }
        if let Some(allocation) = facts.allocation
            && (!allocation.array || facts.definition.is_none())
        {
            metadata.role = Some(match allocation.role {
                crate::sema::lowering::AllocationRole::Allocate => Role::AllocateMemory,
                crate::sema::lowering::AllocationRole::Free => Role::FreeMemory,
            });
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
        let function = self.function_encoding(entity)?;
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
    pub(super) fn demand_entry(&mut self, entry: super::construction::Entry) {
        if self.seen.insert(entry) {
            self.pending.function(entry);
        }
    }
    pub(super) fn entry_symbol(
        &mut self,
        program: &mut Program,
        entry: super::construction::Entry,
    ) -> Result<SymbolId, Error> {
        use super::construction::{Entry, Variant};
        if let Entry::Function(entity) = entry {
            return self.function_symbol(program, entity);
        }
        if let Some(&symbol) = self.entries.get(&entry) {
            return Ok(symbol);
        }
        let (entity, variant) = match entry {
            Entry::Constructor(id, variant) => (self.graph.lower_constructor_entity(id), variant),
            Entry::DestructorBase(entity) => (Some(entity), Variant::Base),
            Entry::DestructorDeleting(entity) => (Some(entity), Variant::Deleting),
            Entry::Function(_) => return Err(Error::Missing("entry kind")),
        };
        if variant == Variant::Complete
            && let Some(entity) = entity
        {
            let symbol = self.function_symbol(program, entity)?;
            self.entries.insert(entry, symbol);
            return Ok(symbol);
        }
        let facts = entry.facts(self.graph)?;
        let mut components = vec![self.function_label(facts.name, facts.label)?];
        let mut scope = facts.home;
        while let (Some(parent), name) = self.graph.lower_presentation_scope(scope) {
            components.push(self.presentation_scope_label(scope, name)?);
            scope = parent;
        }
        components.reverse();
        let mut name = if let Some(entity) = entity {
            self.name(entity)?.to_string()
        } else {
            components.join("__")
        };
        if variant == Variant::Base {
            name.push_str("__base_entry");
        } else if variant == Variant::Deleting {
            name.push_str("__deleting_entry");
        }
        let metadata = self.entry_metadata(program, entry, &name)?;
        let object = if facts.internal
            && metadata
                .object
                .is_some_and(|object| self.table.objects.contains_key(&object))
        {
            None
        } else {
            metadata.object
        };
        let symbol = self
            .table
            .allocate(program, &name, metadata.object, facts.internal)?;
        self.entry_objects.insert(entry, object);
        self.entries.insert(entry, symbol);
        Ok(symbol)
    }
    pub(super) fn definition_metadata(
        &mut self,
        program: &mut Program,
        entry: super::construction::Entry,
        symbol: SymbolId,
    ) -> Result<Metadata, Error> {
        let name = program.names.get(program.symbol(symbol).name).to_owned();
        let mut metadata = self.entry_metadata(program, entry, &name)?;
        let facts = entry.facts(self.graph)?;
        if entry
            .entity(self.graph)
            .is_some_and(|entity| self.graph.lower_explicit_function_root(entity))
        {
            metadata.flags |= 4;
        }
        if facts.internal
            && !matches!(entry, super::construction::Entry::DestructorDeleting(_))
            && (matches!(facts.name, FunctionName::Destructor(_))
                || entry
                    .entity(self.graph)
                    .is_some_and(|entity| self.graph.lower_virtual_slot(entity).is_some()))
        {
            metadata.flags |= 4;
        }
        Ok(metadata)
    }
    pub(super) fn entry_metadata(
        &mut self,
        program: &mut Program,
        entry: super::construction::Entry,
        name: &str,
    ) -> Result<Metadata, Error> {
        use super::construction::{Entry, Variant};
        if let Entry::Function(entity) = entry {
            return self.metadata(program, entity, name);
        }
        let (entity, variant) = match entry {
            Entry::Constructor(id, variant) => (self.graph.lower_constructor_entity(id), variant),
            Entry::DestructorBase(entity) => (Some(entity), Variant::Base),
            Entry::DestructorDeleting(entity) => (Some(entity), Variant::Deleting),
            Entry::Function(_) => return Err(Error::Missing("entry kind")),
        };
        if variant == Variant::Complete
            && let Some(entity) = entity
        {
            let mut metadata = self.metadata(program, entity, name)?;
            if metadata.binding == Some(crate::lowir::vocabulary::Binding::Internal) {
                metadata.flags |= 4;
            }
            return Ok(metadata);
        }
        let facts = entry.facts(self.graph)?;
        let object = if let Some(&object) = self.entry_objects.get(&entry) {
            object
        } else {
            let function = self.encode_function(&facts, variant)?;
            let mut object = Vec::new();
            abi::encoder::Encoder::encode(
                &self.abi,
                abi::encoder::Target::Entity(function),
                &mut object,
                self.budget,
            )?;
            Some(program.names.intern_bytes(&object)?)
        };
        Ok(Metadata {
            object,
            binding: Some(if facts.internal {
                crate::lowir::vocabulary::Binding::Internal
            } else if facts.emission == crate::sema::lowering::Emission::Demanded {
                crate::lowir::vocabulary::Binding::Weak
            } else {
                crate::lowir::vocabulary::Binding::Strong
            }),
            flags: if entity.map_or(
                facts.emission == crate::sema::lowering::Emission::Demanded,
                |entity| self.graph.lower_inline_hint(entity),
            ) {
                16
            } else {
                0
            } | if entity.is_some_and(|entity| self.graph.lower_no_inline(entity)) {
                32
            } else {
                0
            } | if variant == Variant::Base || facts.internal && variant != Variant::Deleting
            {
                4
            } else {
                0
            } | if facts
                .object
                .is_some_and(|class| self.graph.lower_class_no_linkage(class))
            {
                2
            } else {
                0
            },
            ..Metadata::default()
        })
    }
    pub(super) fn finish_entry(
        &mut self,
        program: &mut Program,
        entry: super::construction::Entry,
        symbol: SymbolId,
    ) -> Result<(), Error> {
        use super::construction::{Entry, Variant};
        if let Some(function) = entry.entity(self.graph) {
            self.name_local_classes(function)?;
        }
        if let Entry::Function(entity) = entry
            && matches!(
                self.graph.lower_function(entity)?.name,
                FunctionName::Destructor(_)
            )
        {
            if self
                .entries
                .get(&Entry::DestructorBase(entity))
                .is_some_and(|&base| program.function_defined(base))
            {
                return Ok(());
            }
            let facts = self.graph.lower_function(entity)?;
            let encoding = self.encode_function(&facts, Variant::Base)?;
            let mut bytes = Vec::new();
            abi::encoder::Encoder::encode(
                &self.abi,
                abi::encoder::Target::Entity(encoding),
                &mut bytes,
                self.budget,
            )?;
            let object = program.names.intern_bytes(&bytes)?;
            self.table
                .entry_aliases
                .push(crate::lowir::model::ObjectAlias {
                    object,
                    target: symbol,
                });
        }
        if let Entry::Constructor(id, Variant::Complete) = entry {
            if let Some(&base) = self.entries.get(&Entry::Constructor(id, Variant::Base))
                && program.function_defined(base)
            {
                return Ok(());
            }
            let facts = entry.facts(self.graph)?;
            let encoding = self.encode_function(&facts, Variant::Base)?;
            let mut object = Vec::new();
            abi::encoder::Encoder::encode(
                &self.abi,
                abi::encoder::Target::Entity(encoding),
                &mut object,
                self.budget,
            )?;
            let object = program.names.intern_bytes(&object)?;
            self.table
                .entry_aliases
                .push(crate::lowir::model::ObjectAlias {
                    object,
                    target: symbol,
                });
        }
        Ok(())
    }
    pub(super) fn stream_definition(
        &mut self,
        program: &mut Program,
        output: &mut super::definition_order::Definitions,
        entry: super::construction::Entry,
    ) -> Result<(), Error> {
        let destructor = if matches!(entry.facts(self.graph)?.name, FunctionName::Destructor(_)) {
            entry.entity(self.graph)
        } else {
            None
        };
        let family = if let super::construction::Entry::Constructor(id, variant) = entry {
            let family = if let Some(&family) = self.entry_families.get(&id) {
                family
            } else {
                let facts = entry.facts(self.graph)?;
                let object = if facts.internal {
                    None
                } else {
                    let encoding =
                        self.encode_function(&facts, super::construction::Variant::Base)?;
                    let mut object = Vec::new();
                    abi::encoder::Encoder::encode(
                        &self.abi,
                        abi::encoder::Target::Entity(encoding),
                        &mut object,
                        self.budget,
                    )?;
                    Some(program.names.intern_bytes(&object)?)
                };
                let family = output.family(object)?;
                self.entry_families.insert(id, family);
                family
            };
            Some((family, variant))
        } else if let Some(entity) = destructor {
            let family = if let Some(&family) = self.destructor_families.get(&entity) {
                family
            } else {
                let facts = entry.facts(self.graph)?;
                let object = if facts.internal {
                    None
                } else {
                    let encoding =
                        self.encode_function(&facts, super::construction::Variant::Base)?;
                    let mut bytes = Vec::new();
                    abi::encoder::Encoder::encode(
                        &self.abi,
                        abi::encoder::Target::Entity(encoding),
                        &mut bytes,
                        self.budget,
                    )?;
                    Some(program.names.intern_bytes(&bytes)?)
                };
                let family = output.family(object)?;
                self.destructor_families.insert(entity, family);
                family
            };
            Some((
                family,
                match entry {
                    super::construction::Entry::DestructorBase(_) => {
                        super::construction::Variant::Base
                    }
                    super::construction::Entry::DestructorDeleting(_) => {
                        super::construction::Variant::Deleting
                    }
                    _ => super::construction::Variant::Complete,
                },
            ))
        } else {
            None
        };
        if let super::construction::Entry::Function(entity) = entry
            && self.destructor_inert(entity)?
        {
            // Later object declarations may root this destructor after its
            // source definition. Release its body now, defer only its header.
            output.defer_definition(program)
        } else {
            output.stream(program, family)?;
            if let Some(entity) = entry.entity(self.graph)
                && let Some(form) = self.graph.lower_copy_form(entity)?
            {
                let key = (form.class, form.assignment);
                let family = if let Some(&family) = self.copy_families.get(&key) {
                    family
                } else {
                    let ty = self.graph.lower_entity_type(form.class)?;
                    let object = if form.locality == crate::sema::lowering::Locality::Unit {
                        None
                    } else {
                        let node = self.abi_type(ty)?;
                        let mut bytes = Vec::new();
                        abi::encoder::Encoder::encode(
                            &self.abi,
                            abi::encoder::Target::Type(node),
                            &mut bytes,
                            self.budget,
                        )?;
                        Some(program.names.intern_bytes(&bytes)?)
                    };
                    let family = output.form_family(object, form.assignment)?;
                    self.copy_families.insert(key, family);
                    family
                };
                output.note_form(family, form.moving)?;
            }
            Ok(())
        }
    }
    fn function_encoding(&mut self, entity: EntityId) -> Result<NodeId, Error> {
        let facts = self.graph.lower_function(entity)?;
        self.encode_function(&facts, super::construction::Variant::Complete)
    }
    fn encode_function(
        &mut self,
        facts: &crate::sema::lowering::FunctionFacts,
        variant: super::construction::Variant,
    ) -> Result<NodeId, Error> {
        if facts.c_linkage
            || matches!(facts.name,
            FunctionName::Identifier(identifier) if self.cursor.name(identifier) == b"main"
                && self.graph.lower_presentation_scope(facts.home).0.is_none())
        {
            let spelling = self.source_function_spelling(facts.name)?;
            let terminal = self.abi.string(spelling.as_bytes())?;
            return Ok(self.abi.intern(
                Node::Name {
                    parent: None,
                    terminal,
                    internal: false,
                    occurrence: 0,
                },
                &[],
            )?);
        }
        let parent = self.scope(facts.home)?;
        let terminal = if matches!(
            facts.name,
            FunctionName::Constructor(_) | FunctionName::Destructor(_)
        ) {
            self.abi.intern(
                Node::Terminal {
                    kind: match (facts.name, variant) {
                        (FunctionName::Destructor(_), super::construction::Variant::Base) => {
                            abi::vocabulary::Terminal::DestructorBase
                        }
                        (FunctionName::Destructor(_), super::construction::Variant::Deleting) => {
                            abi::vocabulary::Terminal::DestructorDeleting
                        }
                        (FunctionName::Destructor(_), _) => {
                            abi::vocabulary::Terminal::DestructorComplete
                        }
                        (_, super::construction::Variant::Base) => {
                            abi::vocabulary::Terminal::ConstructorBase
                        }
                        _ => abi::vocabulary::Terminal::ConstructorComplete,
                    },
                    inherited: None,
                },
                &[],
            )?
        } else {
            self.function_terminal(facts.name, facts.ty, facts.object.is_some())?
        };
        let name = self
            .abi
            .intern(
                Node::Name {
                    parent,
                    terminal,
                    internal: facts.declared_linkage
                        == crate::sema::lowering::DeclaredLinkage::Internal
                        && !self.graph.lower_anonymous_namespace(facts.home),
                    occurrence: 0,
                },
                &[],
            )
            .map_err(Error::from)?;
        let mut name = name;
        let signature = if let Some(instance) = facts.template_instance {
            let (pattern, arguments) = self.graph.lower_template_signature(instance)?;
            let mut nodes = Vec::with_capacity(arguments.len());
            for (index, &argument) in arguments.iter().enumerate() {
                let mut node = self.abi_type(argument)?;
                if let Some(ty) = self.graph.lower_template_value_type(instance, index)
                    && self.declared_type_dependent(ty)?
                {
                    let ty = self.abi_type(ty)?;
                    let declaration = self.abi.intern(Node::NonTypeParameter(ty), &[])?;
                    node = self.abi.intern(
                        Node::TypedArgument {
                            declaration,
                            argument: node,
                        },
                        &[],
                    )?;
                }
                nodes.push(node);
            }
            name = self.abi.intern(Node::Template(name), &nodes)?;
            pattern
        } else {
            facts.ty
        };
        let TypeView::Function {
            result,
            parameters,
            variadic,
        } = self.graph.lower_type(signature)?
        else {
            return Err(Error::Semantic(crate::sema::Error::Invariant(
                "function without function type",
            )));
        };
        let mut args = Vec::with_capacity(parameters.len());
        for (index, &parameter) in parameters.iter().enumerate() {
            let parameter = if facts.template_instance.is_some()
                && self.unmangled_placeholder(parameter)?
            {
                let TypeView::Function { parameters, .. } = self.graph.lower_type(facts.ty)? else {
                    return Err(Error::Missing("specialization without function type"));
                };
                *parameters
                    .get(index)
                    .ok_or(Error::Missing("specialization parameter count"))?
            } else {
                parameter
            };
            args.push(self.abi_type(parameter)?);
        }
        let result = if facts.template_instance.is_some() {
            let result = if self.unmangled_placeholder(result)? {
                let TypeView::Function { result, .. } = self.graph.lower_type(facts.ty)? else {
                    return Err(Error::Missing("specialization without function type"));
                };
                result
            } else {
                result
            };
            Some(self.abi_type(result)?)
        } else {
            None
        };
        let function = self
            .abi
            .intern(
                Node::Function {
                    name,
                    result,
                    qualifiers: facts.qualifiers,
                    variadic,
                },
                &args,
            )
            .map_err(Error::from)?;
        Ok(function)
    }
}

impl Symbols<'_, '_> {
    pub(super) fn deleting_inlines_body(&mut self, entity: EntityId) -> Result<bool, Error> {
        if !self.destructor_body_inert(entity)? {
            return Ok(false);
        }
        let class =
            self.graph
                .lower_function(entity)?
                .object
                .ok_or(crate::sema::Error::Invariant(
                    "deleting entry without class",
                ))?;
        for &member in self.graph.lower_class_member_entities(class) {
            if self
                .class_destructor(self.graph.lower_entity_type(member)?)?
                .is_some()
            {
                return Ok(false);
            }
        }
        Ok(true)
    }
    pub(super) fn destructor_inert(&mut self, entity: EntityId) -> Result<bool, Error> {
        self.check_stack()?;
        if let Some(&inert) = self.inert_destructors.get(&entity) {
            return Ok(inert);
        }
        let graph = self.graph;
        let mut inert = self.destructor_body_inert(entity)?
            && graph
                .lower_function(entity)?
                .object
                .is_none_or(|class| !graph.lower_polymorphic(class));
        if inert && let Some(class) = graph.lower_function(entity)?.object {
            for base in graph.lower_class_bases(class) {
                if let Some(child) = graph.lower_destructor_entity(base) {
                    inert &= self.destructor_inert(child)?;
                }
            }
            for &member in graph.lower_class_member_entities(class) {
                let mut ty = graph.lower_entity_type(member)?;
                while let TypeView::Array { element, .. } = graph.lower_type(ty)? {
                    ty = element;
                }
                if let TypeView::Class(child) = graph.lower_type(ty)?
                    && let Some(child) = graph.lower_destructor_entity(child)
                {
                    inert &= self.destructor_inert(child)?;
                }
            }
        }
        self.inert_destructors.insert(entity, inert);
        Ok(inert)
    }
    fn destructor_body_inert(&self, entity: EntityId) -> Result<bool, Error> {
        use super::construction::Entry;
        // The C++ inertness query can precede instantiate_definition. Keep
        // that availability boundary even though Rust prepares all semantic
        // plans before lowering. Demand makes an outside or synthesized body visible;
        // an already memoized inertness result retains its original meaning.
        if self.graph.lower_definition_at_emission(entity)
            && ![
                Entry::Function(entity),
                Entry::DestructorBase(entity),
                Entry::DestructorDeleting(entity),
            ]
            .iter()
            .any(|entry| self.seen.contains(entry))
        {
            return Ok(false);
        }
        Ok(self.graph.lower_destructor_body_inert(entity, self.tree)?)
    }
}

impl Symbols<'_, '_> {
    pub(super) fn result_boundary(
        &mut self,
        ty: TypeId,
    ) -> Result<super::class_results::ResultBoundary, Error> {
        let mut boundary = super::class_results::ResultBoundary {
            ty: crate::lowir::vocabulary::Type::Void,
            indirect: false,
            classes: None,
        };
        if let TypeView::Class(class) = self.graph.lower_type(ty)? {
            // Merely naming a function returning an incomplete class does not
            // demand that class's layout or its special members.
            if !self.graph.lower_complete_object(ty)? {
                return Ok(boundary);
            }
            boundary.indirect = !self.graph.lower_direct_class_parameter(class)?
                || super::types::layout(self.graph, ty)?.0 > 16
                || self.class_result_memory(class)?;
            if boundary.indirect {
                return Ok(boundary);
            }
            boundary.classes = self.parameter_classes(ty)?.filter(|classes| {
                !matches!(classes, crate::lowir::vocabulary::ObjectClasses::Memory)
            });
        }
        boundary.ty = super::types::storage(self.graph, ty)?;
        Ok(boundary)
    }
}

#[cfg(test)]
pub(crate) use vtables::assert_table_demand;

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

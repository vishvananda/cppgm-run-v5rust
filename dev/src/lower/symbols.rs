// Translated from lower/symbol_lowering.cpp's procedural symbol and ABI construction. See NOTICE.
use super::Error;
#[path = "symbol_state.rs"]
mod state;
pub(super) use state::State;
#[path = "abi_tags.rs"]
mod abi_tags;
#[path = "argument_declarations.rs"]
mod argument_declarations;
#[path = "call_signatures.rs"]
mod call_signatures;
#[path = "callable_encoding.rs"]
mod callable_encoding;
#[path = "complex.rs"]
pub(super) mod complex;
#[path = "definition_families.rs"]
mod definition_families;
#[path = "exception_information.rs"]
mod exception_information;
#[path = "inline_definitions.rs"]
mod inline_definitions;
#[path = "lifetime_symbols.rs"]
mod lifetime_symbols;
#[path = "placeholder_mangling.rs"]
mod placeholder_mangling;
#[path = "runtime_types.rs"]
mod runtime_types;
#[path = "soft_float.rs"]
pub(super) mod soft_float;
#[path = "source_positions.rs"]
mod source_positions;
#[path = "specialization_labels.rs"]
mod specialization_labels;
#[path = "thread_local_symbols.rs"]
mod thread_local_symbols;
#[path = "type_information.rs"]
mod type_information;
#[path = "virtual_bases.rs"]
mod virtual_bases;
#[path = "vtables.rs"]
mod vtables;
use crate::{
    abi::{
        self,
        graph::{Modifier, Node, NodeId, ParameterKind, Qualifiers},
        vocabulary::Builtin as B,
    },
    lowir::model::{Linkage, Metadata, NameId, Program, SourceId, SymbolId, Value},
    sema::{
        Graph,
        lowering::{EntityId, FunctionName, ScopeId, TypeId, TypeView},
    },
    support::stack::StackBudget,
    syntax::{ast, cursor::Cursor},
    types::FundamentalType as F,
};
use std::{collections::HashMap, fmt::Write};
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum DebugLocations {
    #[default]
    Disabled,
    Enabled,
}
#[derive(Default)]
pub(super) struct SymbolTable {
    pub(super) work: super::observations::Work,
    deferred_globals: std::collections::HashSet<SymbolId>,
    pub(super) object_mode: bool,
    pub(super) debug_locations: DebugLocations,
    pub(super) host_objects: bool,
    objects: HashMap<NameId, Option<SymbolId>>,
    suffixes: HashMap<NameId, u32>,
    source: Option<SourceId>,
    predefined_variables: HashMap<EntityId, SymbolId>,
    string_literals: u32,
    reference_temporaries: u32,
    initializer_list_backings: u32,
    initializer: Option<SymbolId>,
    exception_helpers: Option<super::exception_helpers::Helpers>,
    terminate_pending: bool,
    bad_array_length: Option<SymbolId>,
    unexpected_function: Option<SymbolId>,
    hosted_runtime: [Option<SymbolId>; 3],
    atexit: Option<SymbolId>,
    guard_functions: [Option<SymbolId>; 3],
    thread_atexit: Option<SymbolId>,
    dso_handle: Option<SymbolId>,
    runtime_roles: Vec<super::runtime_roles::RoleSymbol>,
    runtime_entries: HashMap<crate::lowir::vocabulary::Role, SymbolId>,
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
impl Symbols<'_, '_> {
    pub(super) fn hosted_runtime_function(
        &mut self,
        program: &mut Program,
        function: super::runtime_intrinsics::RuntimeFunction,
    ) -> Result<SymbolId, Error> {
        if let Some(symbol) = self.state.table.hosted_runtime[function.index()] {
            return Ok(symbol);
        }
        let symbol = self
            .state
            .table
            .allocate(program, function.spelling().0, None, true)?;
        super::runtime_intrinsics::declare_runtime(program, symbol, function)?;
        self.state.table.hosted_runtime[function.index()] = Some(symbol);
        program.publish_runtime_declaration(symbol);
        Ok(symbol)
    }
}

impl State {
    pub(super) fn view<'a, 'source>(
        &'a mut self,
        graph: &'a Graph,
        tree: &'a ast::Tree,
        cursor: &'a Cursor<'source>,
    ) -> Symbols<'a, 'source> {
        Symbols::new(graph, tree, cursor, self)
    }
}

fn parameter_kind(parameter: &crate::sema::lowering::TemplateParameterView) -> ParameterKind {
    if parameter.value {
        ParameterKind::Type
    } else {
        match (parameter.template, parameter.pack) {
            (false, false) => ParameterKind::Type,
            (false, true) => ParameterKind::TypePack,
            (true, false) => ParameterKind::Template,
            (true, true) => ParameterKind::TemplatePack,
        }
    }
}
pub(super) struct Symbols<'a, 'source> {
    graph: &'a Graph,
    tree: &'a ast::Tree,
    cursor: &'a Cursor<'source>,
    state: &'a mut State,
}
impl std::ops::Deref for Symbols<'_, '_> {
    type Target = State;
    fn deref(&self) -> &State {
        self.state
    }
}
impl std::ops::DerefMut for Symbols<'_, '_> {
    fn deref_mut(&mut self) -> &mut State {
        self.state
    }
}
impl<'a, 'source> Symbols<'a, 'source> {
    pub(super) fn object_mode(&self) -> bool {
        self.state.table.object_mode
    }
    pub(super) fn host_objects(&self) -> bool {
        self.state.table.host_objects
    }
    // Text streaming publishes pending source declarations at unit boundaries.
    // They remain unresolved in C++ until finish(), unlike runtime declarations.
    pub(super) fn note_deferred_global(&mut self, symbol: SymbolId) {
        self.state.table.deferred_globals.insert(symbol);
    }
    pub(super) fn global_definition_conflicts(&self, program: &Program, symbol: SymbolId) -> bool {
        program.symbol(symbol).declared()
            && !(self.state.table.deferred_globals.contains(&symbol)
                && program
                    .global(program.symbol(symbol))
                    .is_some_and(|global| {
                        matches!(global.init, crate::lowir::model::GlobalInit::Declaration)
                    }))
    }
    pub(super) fn note_declaration_only_global(&mut self, entity: EntityId) {
        self.state.declaration_only_globals.insert(entity);
    }
    pub(super) fn declaration_only_global(&self, entity: EntityId) -> bool {
        self.state.declaration_only_globals.contains(&entity)
    }
    pub(super) fn new(
        graph: &'a Graph,
        tree: &'a ast::Tree,
        cursor: &'a Cursor<'source>,
        state: &'a mut State,
    ) -> Self {
        Self {
            graph,
            tree,
            cursor,
            state,
        }
    }
    pub(super) fn publish_finalizers(&mut self, output: &mut Vec<super::shutdown::Finalizer>) {
        output.append(&mut self.state.persistent_finalizers);
    }
    pub(super) fn persistent_finalizer(
        &mut self,
        program: &mut Program,
        global: SymbolId,
        ty: TypeId,
        local: Option<&mut Vec<super::shutdown::Finalizer>>,
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
        let flag = self.state.table.allocate(program, &name, None, true)?;
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
        if let Some(local) = local {
            local.push(finalizer);
        } else {
            self.state.persistent_finalizers.push(finalizer);
        }
        Ok(Some(flag))
    }
    pub(super) fn object_argument_classes(
        &mut self,
        ty: TypeId,
    ) -> Result<Option<crate::lowir::vocabulary::ObjectClasses>, Error> {
        self.state
            .object_classes
            .classify(self.graph, ty, self.state.budget)
    }
    pub(super) fn parameter_classes(
        &mut self,
        ty: TypeId,
    ) -> Result<Option<crate::lowir::vocabulary::ObjectClasses>, Error> {
        if matches!(self.graph.lower_type(ty)?, TypeView::Class(class) if !self.graph.lower_direct_class_parameter(class)?)
        {
            return Ok(None);
        }
        if self.object_mode() {
            return Ok(self
                .state
                .object_classes
                .classify(self.graph, ty, self.state.budget)?
                .map(|classes| match classes {
                    crate::lowir::vocabulary::ObjectClasses::X87
                    | crate::lowir::vocabulary::ObjectClasses::ComplexX87 => {
                        crate::lowir::vocabulary::ObjectClasses::Memory
                    }
                    other => other,
                }));
        }
        self.state
            .class_parameters
            .parameter(self.graph, ty, self.state.budget)
    }
    pub(super) fn class_result_memory(&mut self, class: EntityId) -> Result<bool, Error> {
        self.state
            .class_parameters
            .result_memory(self.graph, class, self.state.budget)
    }
    // program_lowering.cpp::parameter_name: syntax element identities remain
    // zero based; the emitted first parameter retains the original pack name.
    pub(super) fn parameter_name(
        &self,
        name: crate::lex::identifiers::NameId,
    ) -> Result<String, Error> {
        let spelling = self.identifier(name)?;
        let digits = spelling.trim_end_matches(|c: char| c.is_ascii_digit());
        if digits.len() == spelling.len() {
            return Ok(spelling.to_owned());
        }
        let Some(base) = digits
            .strip_suffix("__pack")
            .filter(|base| !base.is_empty())
        else {
            return Ok(spelling.to_owned());
        };
        let index = spelling.as_bytes()[digits.len()..]
            .iter()
            .fold(0u64, |index, digit| {
                index
                    .saturating_mul(10)
                    .saturating_add(u64::from(*digit - b'0'))
            });
        if index == 0 {
            Ok(base.to_owned())
        } else {
            let next = index.wrapping_add(1);
            Ok(format!("{base}__pack{next}"))
        }
    }
    pub(super) fn identifier(&self, name: crate::lex::identifiers::NameId) -> Result<&str, Error> {
        std::str::from_utf8(self.cursor.name(name))
            .map_err(|_| Error::Missing("local name spelling"))
    }
    pub(super) fn request_initialization(&mut self, program: &mut Program) -> Result<(), Error> {
        let symbol = if let Some(symbol) = self.state.table.initializer {
            symbol
        } else {
            let symbol = self
                .state
                .table
                .allocate(program, "__cppgm_init", None, true)?;
            self.state.table.initializer = Some(symbol);
            symbol
        };
        self.state.initialization = Some(symbol);
        Ok(())
    }
    pub(super) fn resolve_runtime_roles(
        &mut self,
        program: &mut Program,
        definitions: &mut super::definition_order::Definitions,
    ) -> Result<(), Error> {
        super::runtime_roles::resolve(
            program,
            definitions,
            &self.state.table.runtime_roles,
            self.state.table.object_mode,
        )
    }
    pub(super) fn bad_array_length(&mut self, program: &mut Program) -> Result<SymbolId, Error> {
        if let Some(symbol) = self.state.table.bad_array_length {
            return Ok(symbol);
        }
        let symbol = self.state.table.allocate(
            program,
            "__external_runtime____cxa_throw_bad_array_new_length",
            None,
            true,
        )?;
        super::exception_helpers::declare_bad_array_length(program, symbol)?;
        self.state.table.bad_array_length = Some(symbol);
        Ok(symbol)
    }
    pub(super) fn terminate_helper(&mut self, program: &mut Program) -> Result<SymbolId, Error> {
        if let Some(helpers) = self.state.table.exception_helpers {
            return Ok(helpers.helper);
        }
        let helpers = super::exception_helpers::Helpers {
            begin_catch: self
                .runtime_function(program, crate::lowir::vocabulary::Role::EhBeginCatch)?,
            terminate: self
                .state
                .table
                .allocate(program, "std_terminate", None, true)?,
            helper: self
                .state
                .table
                .allocate(program, "cppgm_call_terminate", None, true)?,
        };
        super::exception_helpers::declare(program, helpers)?;
        self.state
            .table
            .runtime_roles
            .push(super::runtime_roles::RoleSymbol {
                symbol: helpers.terminate,
                role: crate::lowir::vocabulary::Role::Terminate,
                origin: super::runtime_roles::Origin::Runtime,
            });
        self.state.table.exception_helpers = Some(helpers);
        self.state.table.terminate_pending = true;
        Ok(helpers.helper)
    }
    pub(super) fn take_terminate_helper(&mut self) -> Option<super::exception_helpers::Helpers> {
        if !self.state.table.terminate_pending {
            return None;
        }
        self.state.table.terminate_pending = false;
        self.state.table.exception_helpers
    }
    pub(super) fn aggregate_helper_symbol(
        &mut self,
        program: &mut Program,
        class: EntityId,
        count: usize,
        cv: u8,
    ) -> Result<SymbolId, Error> {
        let key = super::aggregate_helpers::Key { class, count, cv };
        if let Some(symbol) = self.state.aggregate_helpers.get(key) {
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
        let shapes = self.state.aggregate_helpers.shapes(class);
        if shapes != 0 {
            write!(spelling, "__sym{}", shapes + 1)
                .map_err(|_| Error::Missing("helper label formatting"))?;
        }
        let symbol = self.state.table.allocate(program, &spelling, None, true)?;
        super::aggregate_helpers::declare(program, self, self.graph, key, symbol)?;
        self.state.aggregate_helpers.insert(key, symbol);
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
        if self
            .graph
            .lower_function(entity)?
            .object
            .is_some_and(|class| !self.graph.lower_virtual_bases(class).is_empty())
        {
            self.demand_entry(super::construction::Entry::DestructorBase(entity));
        }
        self.state.rooted_destructors.insert(entity);
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
        if let Some(&known) = self.state.empty_list_constructions.get(&class) {
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
            self.state
                .empty_list_constructions
                .insert(class, constructs);
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
        if let Some(&properties) = self.state.class_storage.get(&class) {
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
        self.state.class_storage.insert(class, properties);
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
    fn presentation_scope_label(
        &mut self,
        scope: ScopeId,
        name: Option<crate::lex::identifiers::NameId>,
    ) -> Result<String, Error> {
        if let Some(class) = self.graph.lower_scope_class(scope) {
            return self.specialized_class_label(class);
        }
        if let Some((class, _, _)) = self.graph.lower_anonymous_scope(scope)? {
            return self.class_label(self.graph.lower_class_spelling(class)?);
        }
        Ok(name
            .map_or(Ok("_GLOBAL__N_1"), |name| self.identifier(name))?
            .to_owned())
    }
    fn class_label(&self, label: crate::sema::lowering::ClassSpelling) -> Result<String, Error> {
        use crate::sema::lowering::ClassSpelling;
        Ok(match label {
            ClassSpelling::Closure(ordinal) => format!("__lambda{ordinal}"),
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
                bytes.extend_from_slice(self.class_label(name)?.as_bytes());
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
                    if token.as_punctuator() == Some(crate::lex::pp_lexer::Punctuator::LeftParen) {
                        bytes.push(b')');
                    }
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
                return Ok(self.state.abi.string(self.cursor.name(name))?);
            }
            FunctionName::Conversion(target) => Node::Conversion(self.abi_type(target)?),
            FunctionName::Literal(suffix) => {
                Node::LiteralOperator(self.state.abi.string(self.cursor.name(suffix))?)
            }
            FunctionName::Operator { token, array } => {
                let TypeView::Function { parameters, .. } = self.graph.lower_type(ty)? else {
                    return Err(Error::Semantic(crate::sema::Error::Invariant(
                        "operator without function type",
                    )));
                };
                let parameters = parameters.to_vec();
                let parameters = parameters.as_slice();
                return self.abi_operator_terminal(
                    token,
                    array,
                    parameters.len() + usize::from(member),
                );
            }
        };
        Ok(self.state.abi.intern(node, &[])?)
    }
    pub(super) fn abi_operator_terminal(
        &mut self,
        token: crate::lex::vocabulary::SimpleToken,
        array: bool,
        operands: usize,
    ) -> Result<NodeId, Error> {
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
        Ok(self.state.abi.intern(
            Node::Terminal {
                kind: terminal.for_operands(operands),
                inherited: None,
            },
            &[],
        )?)
    }
    pub(super) fn abi_literal(&mut self, ty: TypeId, bits: u64) -> Result<Node, Error> {
        let unsigned_width = if matches!(self.graph.lower_type(ty)?, TypeView::Enum(_)) {
            self.graph.lower_unsigned_integral_width(ty)
        } else {
            None
        };
        let ty = self.abi_type(ty)?;
        Ok(if let Some(width) = unsigned_width {
            let bits = if width < 64 {
                bits & ((1_u64 << width) - 1)
            } else {
                bits
            };
            Node::UnsignedLiteral { ty, bits }
        } else {
            Node::Literal { ty, bits }
        })
    }
    pub(super) fn note_source_reference_storage(&mut self, symbol: SymbolId) {
        // define_reference publishes its source-shaped backing outside the
        // counted global-definition path; constant_temporary counts its own.
        self.state
            .table
            .work
            .source_reference_storage
            .insert(symbol);
    }
    pub(super) fn reference_temporary_symbol(
        &mut self,
        program: &mut Program,
        entity: EntityId,
        constant: bool,
    ) -> Result<(SymbolId, Option<NameId>), Error> {
        let spelling = if constant {
            self.state.table.reference_temporaries = self
                .state
                .table
                .reference_temporaries
                .checked_add(1)
                .ok_or(crate::lowir::Error::Limit)?;
            format!("__ref_tmp__{}", self.state.table.reference_temporaries)
        } else {
            format!(
                "__ref_tmp__{}",
                self.identifier(self.graph.lower_variable(entity)?.named()?)?
            )
        };
        self.named_reference_temporary(program, &spelling)
    }
    pub(super) fn constant_temporary_symbol(
        &mut self,
        program: &mut Program,
    ) -> Result<(SymbolId, Option<NameId>), Error> {
        self.state.table.reference_temporaries = self
            .state
            .table
            .reference_temporaries
            .checked_add(1)
            .ok_or(crate::lowir::Error::Limit)?;
        self.named_reference_temporary(
            program,
            &format!("__ref_tmp__{}", self.state.table.reference_temporaries),
        )
    }
    pub(super) fn dynamic_temporary_symbol(
        &mut self,
        program: &mut Program,
        ty: TypeId,
    ) -> Result<(SymbolId, Option<NameId>), Error> {
        let spelling = if let Some((owner, sequence)) = self.state.temporary_owner {
            let facts = self.graph.lower_variable(owner)?;
            if !matches!(self.graph.lower_type(facts.ty)?, TypeView::Reference { .. })
                && matches!(self.graph.lower_type(ty)?, TypeView::Array { .. })
            {
                self.state.table.initializer_list_backings = self
                    .state
                    .table
                    .initializer_list_backings
                    .checked_add(1)
                    .ok_or(crate::lowir::Error::Limit)?;
                format!(
                    "__cppgm_initlist_backing_{}",
                    self.state.table.initializer_list_backings
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
            self.state.table.reference_temporaries = self
                .state
                .table
                .reference_temporaries
                .checked_add(1)
                .ok_or(crate::lowir::Error::Limit)?;
            format!("__ref_tmp__{}", self.state.table.reference_temporaries)
        };
        let result = self.named_reference_temporary(program, &spelling)?;
        self.note_owned_temporary(result.0);
        Ok(result)
    }
    fn named_reference_temporary(
        &mut self,
        program: &mut Program,
        spelling: &str,
    ) -> Result<(SymbolId, Option<NameId>), Error> {
        let Some((owner, sequence)) = self.state.temporary_owner else {
            return Ok((
                self.state.table.allocate(program, spelling, None, true)?,
                None,
            ));
        };
        self.state.temporary_owner = Some((
            owner,
            sequence.checked_add(1).ok_or(crate::lowir::Error::Limit)?,
        ));
        let owner = if let Some(&name) = self.state.local_static_names.get(&owner) {
            name
        } else {
            self.variable_abi_name(owner)?
        };
        let mut bytes = Vec::new();
        self.state.table.work.mangled_names += 1;
        abi::encoder::Encoder::encode(
            &self.state.abi,
            abi::encoder::Target::ReferenceTemporary { owner, sequence },
            &mut bytes,
            self.state.budget,
        )?;
        let object = program.names.intern_bytes(&bytes)?;
        let local_object = (!self.state.table.objects.contains_key(&object)).then_some(object);
        let symbol = self
            .state
            .table
            .allocate(program, spelling, Some(object), true)?;
        Ok((symbol, local_object))
    }
    pub(super) fn replace_temporary_owner(
        &mut self,
        owner: Option<(EntityId, u32)>,
    ) -> Option<(EntityId, u32)> {
        std::mem::replace(&mut self.state.temporary_owner, owner)
    }
    pub(super) fn temporary_owner_is_reference(&self) -> Result<bool, Error> {
        let Some((owner, _)) = self.state.temporary_owner else {
            return Ok(false);
        };
        Ok(matches!(
            self.graph
                .lower_type(self.graph.lower_entity_type(owner)?)?,
            TypeView::Reference { .. }
        ))
    }
    pub(super) fn data_symbol(
        &mut self,
        program: &mut Program,
        image: Option<usize>,
    ) -> Result<SymbolId, Error> {
        let name = if let Some(index) = image {
            format!("__constexpr_template__{}", index + 1)
        } else {
            self.state.table.string_literals = self
                .state
                .table
                .string_literals
                .checked_add(1)
                .ok_or(crate::lowir::Error::Limit)?;
            format!("__strlit__{}", self.state.table.string_literals)
        };
        self.state.table.allocate(program, &name, None, true)
    }
    pub(super) fn check_stack(&self) -> Result<(), Error> {
        let marker = 0u8;
        if self
            .state
            .budget
            .exceeded(self.state.stack_base, std::ptr::from_ref(&marker).addr())
        {
            return Err(Error::Semantic(crate::sema::Error::Limit(
                "constructs nested too deeply",
            )));
        }
        Ok(())
    }
    fn scope(&mut self, scope: ScopeId) -> Result<Option<NodeId>, Error> {
        self.check_stack()?;
        if let Some(&id) =
            self.state
                .scopes
                .get(&(scope, self.state.bound_pattern, self.state.own_level))
        {
            return Ok(id);
        }
        if let Some(class) = self.graph.lower_scope_class(scope) {
            let id = Some(self.abi_type(self.graph.lower_entity_type(class)?)?);
            self.state
                .scopes
                .insert((scope, self.state.bound_pattern, self.state.own_level), id);
            return Ok(id);
        }
        if let Some(function) = self.graph.lower_local_scope(scope) {
            let facts = self.function_facts(function)?;
            let node = if facts.c_linkage
                || matches!(facts.name,
                FunctionName::Identifier(name) if self.cursor.name(name) == b"main")
            {
                let spelling = self.source_function_spelling(facts.name)?;
                let raw = format!("Z{}{spelling}E", spelling.len());
                Node::RawFunctionScope(self.state.abi.string(raw.as_bytes())?)
            } else {
                Node::FunctionScope {
                    encoding: if let Some((_, encoding)) = self
                        .state
                        .encodings_underway
                        .iter()
                        .rev()
                        .find(|(owner, _)| *owner == function)
                    {
                        *encoding
                    } else {
                        self.function_encoding(function)?
                    },
                    default_argument: 0,
                }
            };
            let id = Some(self.state.abi.intern(node, &[])?);
            self.state
                .scopes
                .insert((scope, self.state.bound_pattern, self.state.own_level), id);
            return Ok(id);
        }
        let (parent, name, namespace) = self.graph.lower_abi_scope(scope)?;
        let id = if let Some(parent) = parent {
            let parent = self.scope(parent)?;
            if namespace
                && parent.is_none()
                && name.is_some_and(|name| self.cursor.name(name) == b"std")
            {
                let id = Some(self.state.abi.intern(Node::Std, &[])?);
                self.state
                    .scopes
                    .insert((scope, self.state.bound_pattern, self.state.own_level), id);
                return Ok(id);
            }
            let terminal = if let Some(terminal) =
                self.state
                    .anonymous_names
                    .scope_terminal(self.graph, &mut self.state.abi, scope)?
            {
                terminal
            } else {
                self.state
                    .abi
                    .string(name.map_or(&b"_GLOBAL__N_1"[..], |name| self.cursor.name(name)))?
            };
            let occurrence = if let Some((class, function, declared)) =
                self.graph.lower_local_class_scope(scope)
            {
                self.local_discriminator(class, function, name, declared)?
            } else {
                0
            };
            let tags = self.graph.lower_namespace_abi_tags(scope);
            let mut nodes = Vec::with_capacity(tags.len());
            for tag in tags {
                nodes.push(self.state.abi.string(self.cursor.name(*tag))?);
            }
            Some(
                self.state
                    .abi
                    .intern(
                        Node::Name {
                            parent,
                            terminal,
                            internal: false,
                            hidden_tags: !nodes.is_empty(),
                            data_member: false,
                            occurrence,
                        },
                        &nodes,
                    )
                    .map_err(Error::from)?,
            )
        } else {
            None
        };
        self.state
            .scopes
            .insert((scope, self.state.bound_pattern, self.state.own_level), id);
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
        if let Some(&known) = self.state.local_discriminators.get(&entity) {
            return Ok(known);
        }
        let Some(name) = name else {
            return Ok(declared);
        };
        let count = self.state.local_counts.entry((function, name)).or_default();
        let occurrence = *count;
        *count = count.checked_add(1).ok_or(crate::lowir::Error::Limit)?;
        self.state.local_discriminators.insert(entity, occurrence);
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
    pub(super) fn name_template_id(
        &mut self,
        view: crate::sema::lowering::TemplateIdView<'_>,
    ) -> Result<NodeId, Error> {
        let name = self.abi_type(view.name)?;
        let mut nodes = Vec::with_capacity(view.arguments.len());
        for &argument in view.arguments {
            nodes.push(self.abi_type(argument)?);
        }
        if let Some(first) = view.pack {
            let pack = self.state.abi.intern(Node::ArgumentPack, &nodes[first..])?;
            nodes.truncate(first);
            nodes.push(pack);
        }
        self.state
            .abi
            .intern(Node::Template(name), &nodes)
            .map_err(Error::from)
    }
    fn abi_type(&mut self, ty: TypeId) -> Result<NodeId, Error> {
        self.check_stack()?;
        if let Some(&id) =
            self.state
                .types
                .get(&(ty, self.state.bound_pattern, self.state.own_level))
        {
            return Ok(id);
        }
        if let Some(aliased) = self.graph.lower_aliased_type(ty) {
            let node = self.abi_type(aliased)?;
            return self.finish_abi_type(ty, node);
        }
        let node = if let Some((_, home, name)) = self.graph.lower_template_name(ty) {
            let parent = self.scope(home)?;
            let terminal = self.state.abi.string(self.cursor.name(name))?;
            // A template used as an argument names the template without its
            // explicit tags; its class specializations retain their own tags.
            let node = self.state.abi.intern(
                Node::Name {
                    parent,
                    terminal,
                    internal: false,
                    hidden_tags: false,
                    data_member: false,
                    occurrence: 0,
                },
                &[],
            )?;
            return self.finish_abi_type(ty, node);
        } else if let Some(view) = self.graph.lower_template_id_name(ty) {
            let id = self.name_template_id(view)?;
            return self.finish_abi_type(ty, id);
        } else if let Some(elements) = self.graph.lower_argument_pack(ty) {
            let elements = elements.to_vec();
            let mut nodes = Vec::with_capacity(elements.len());
            for element in elements {
                nodes.push(self.abi_type(element)?);
            }
            let id = self.state.abi.intern(Node::ArgumentPack, &nodes)?;
            return self.finish_abi_type(ty, id);
        } else if let Some(pattern) = self.graph.lower_pack_expansion(ty) {
            let saved = self.state.bound_pattern;
            self.state.bound_pattern |= self.graph.lower_names_bound_pack(pattern);
            let result = self.expansion_abi_node(pattern);
            self.state.bound_pattern = saved;
            result?
        } else if self.graph.lower_parameter_level(ty).is_some() {
            let parameter = self.abi_parameter_level(ty)?;
            let kind = parameter_kind(&parameter);
            let node = self.state.abi.intern(
                Node::TemplateParam {
                    index: parameter.index,
                    level: parameter.level,
                    substitutable: !parameter.value,
                    kind,
                },
                &[],
            )?;
            if parameter.value {
                Node::ExpressionArgument(node)
            } else {
                return self.finish_abi_type(ty, node);
            }
        } else if let Some((owner, member)) = self.graph.lower_member_pointer_type(ty) {
            let owner = self.abi_type(owner)?;
            let member = self.abi_type(member)?;
            let node = self.state.abi.member_pointer(owner, member)?;
            return self.finish_abi_type(ty, node);
        } else if let Some((qualifier, name)) = self.graph.lower_dependent_member(ty) {
            Node::Name {
                parent: Some(self.abi_type(qualifier)?),
                terminal: self.state.abi.string(self.cursor.name(name))?,
                internal: false,
                hidden_tags: false,
                data_member: false,
                occurrence: 0,
            }
        } else if let Some((operand, _)) = self.graph.lower_template_operand(ty) {
            let Some(expression) = self.placeholder_node(ty)? else {
                let node = self.unencoded_placeholder(ty)?;
                return self.finish_abi_type(ty, node);
            };
            if matches!(self.state.abi.get(expression), Node::VendorType(_)) {
                return self.finish_abi_type(ty, expression);
            }
            let kind = self.tree.expressions.get(operand).kind;
            if self.graph.lower_value_argument(ty) {
                match self.state.abi.get(expression) {
                    Node::Literal { .. }
                    | Node::UnsignedLiteral { .. }
                    | Node::NullptrLiteral(_)
                    | Node::FloatingLiteral { .. } => {
                        return self.finish_abi_type(ty, expression);
                    }
                    Node::EntityReference(entity) => Node::EntityArgument {
                        entity,
                        address: false,
                    },
                    _ => Node::ExpressionArgument(expression),
                }
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
                TypeView::Address { ty, entity } => self.abi_address_argument(ty, entity)?,
                TypeView::Value { ty, bits } => self.abi_literal(ty, bits)?,
                TypeView::Vector { element, bound } => Node::Modified {
                    modifier: Modifier::Vector(bound),
                    operand: self.abi_type(element)?,
                },
                TypeView::Complex(part) => Node::Modified {
                    modifier: Modifier::Complex,
                    operand: self.abi_type(part)?,
                },
                TypeView::Scalar(scalar) => Node::Builtin(super::types::abi_scalar(scalar)),
                TypeView::Pointer(element) => Node::Modified {
                    modifier: Modifier::Pointer,
                    operand: self.abi_type(element)?,
                },
                TypeView::BlockPointer(element) => Node::Modified {
                    modifier: Modifier::Vendor(self.state.abi.string(b"block_pointer")?),
                    operand: self.abi_type(element)?,
                },
                TypeView::Array { element, bound } => Node::Modified {
                    modifier: if let Some(expression) = self
                        .graph
                        .lower_array_operand(ty)
                        .map(|operand| self.placeholder_node(operand))
                        .transpose()?
                        .flatten()
                    {
                        Modifier::ExpressionArray(expression)
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
                    let mut parent =
                        if let Some((qualifier, _)) = self.graph.lower_dependent_template(entity) {
                            Some(self.abi_type(qualifier)?)
                        } else {
                            self.scope(home)?
                        };
                    if self.state.table.object_mode {
                        parent = self.closure_context_scope(entity, parent)?;
                    }
                    let terminal = if self.graph.lower_closure(entity).is_some()
                        && self.graph.lower_numbered_closure(entity)
                    {
                        self.closure_abi_name(entity)?
                    } else if let Some(name) = name {
                        self.state.abi.string(self.cursor.name(name))?
                    } else {
                        self.state.anonymous_names.terminal(
                            self.graph,
                            &mut self.state.abi,
                            entity,
                        )?
                    };
                    let occurrence = if self.graph.lower_closure(entity).is_some() {
                        0
                    } else if let TypeView::Class(class) = self.graph.lower_type(ty)?
                        && let Some((function, declared)) = self.graph.lower_local_type(class)
                    {
                        self.local_discriminator(class, function, name, declared)?
                    } else {
                        0
                    };
                    let name = self.tagged_entity_name(
                        Node::Name {
                            parent,
                            terminal,
                            internal: false,
                            hidden_tags: false,
                            data_member: false,
                            occurrence,
                        },
                        Some(self.graph.lower_class_abi_tag_owner(entity)),
                    )?;
                    let id = if let Some(arguments) = self.graph.lower_class_arguments(entity) {
                        let mut nodes = Vec::with_capacity(arguments.len());
                        for argument in arguments {
                            nodes.push(self.abi_type(argument)?);
                        }
                        self.state.abi.abbreviated_template_id(name, &nodes)?
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
                    let id = self.state.abi.intern(node, &args)?;
                    self.state
                        .types
                        .insert((ty, self.state.bound_pattern, self.state.own_level), id);
                    return Ok(id);
                }
                TypeView::Unsupported => return Err(Error::Missing("aggregate ABI type lowering")),
            }
        };
        let id = self.state.abi.intern(node, &[]).map_err(Error::from)?;
        self.finish_abi_type(ty, id)
    }
    fn closure_abi_name(&mut self, class: EntityId) -> Result<NodeId, Error> {
        let closure = self
            .graph
            .lower_closure(class)
            .ok_or(crate::sema::Error::Invariant("closure ABI owner"))?;
        let facts = self.function_facts(closure.call)?;
        let TypeView::Function {
            parameters,
            variadic,
            ..
        } = self.graph.lower_type(facts.ty)?
        else {
            return Err(crate::sema::Error::Invariant("closure ABI signature").into());
        };
        let parameters = parameters.to_vec();
        let parameters = parameters.as_slice();
        let mut nodes = Vec::with_capacity(parameters.len());
        for &parameter in parameters {
            nodes.push(self.abi_type(parameter)?);
        }
        Ok(self.state.abi.intern(
            Node::ClosureName {
                number: u32::from(if self.state.table.object_mode {
                    closure.object_number
                } else {
                    closure.course_number
                }),
                variadic: self.state.table.object_mode && variadic,
            },
            &nodes,
        )?)
    }

    fn closure_context_scope(
        &mut self,
        class: EntityId,
        parent: Option<NodeId>,
    ) -> Result<Option<NodeId>, Error> {
        use crate::sema::lowering::ClosureNamingContext;
        let Some(context) = self
            .graph
            .lower_closure(class)
            .and_then(|facts| facts.naming_context)
        else {
            return Ok(parent);
        };
        let node = match context {
            ClosureNamingContext::DataMember(member) => {
                let facts = self.graph.lower_variable(member)?;
                let name = facts
                    .name
                    .ok_or(crate::sema::Error::Invariant("closure member name"))?;
                Node::Name {
                    parent,
                    terminal: self.state.abi.string(self.cursor.name(name))?,
                    internal: false,
                    hidden_tags: false,
                    data_member: true,
                    occurrence: 0,
                }
            }
            ClosureNamingContext::DefaultArgument { function, index } => {
                let facts = self.function_facts(function)?;
                let TypeView::Function { parameters, .. } = self.graph.lower_type(facts.ty)? else {
                    return Err(crate::sema::Error::Invariant("closure context signature").into());
                };
                let parameters = parameters.to_vec();
                let parameters = parameters.as_slice();
                let count =
                    u32::try_from(parameters.len()).map_err(|_| crate::lowir::Error::Limit)?;
                Node::FunctionScope {
                    encoding: self.function_encoding(function)?,
                    default_argument: count
                        .checked_sub(index)
                        .filter(|&number| number != 0)
                        .ok_or(crate::sema::Error::Invariant(
                            "closure default argument index",
                        ))?,
                }
            }
        };
        Ok(Some(self.state.abi.intern(node, &[])?))
    }

    fn expansion_abi_node(&mut self, pattern: TypeId) -> Result<Node, Error> {
        let operand = if self.graph.lower_value_argument(pattern) {
            if let Some(index) = self.graph.lower_template_parameter(pattern) {
                let parameter = self.abi_parameter_level(pattern)?;
                let node = Node::TemplateParam {
                    index,
                    level: parameter.level,
                    substitutable: false,
                    kind: ParameterKind::Type,
                };
                Some(self.state.abi.intern(node, &[])?)
            } else {
                self.placeholder_node(pattern)?
            }
        } else {
            None
        };
        if let Some(operand) = operand {
            let expression = self.state.abi.intern(
                Node::Operation(abi::vocabulary::Operation::PackExpansion),
                &[operand],
            )?;
            Ok(Node::ExpressionArgument(expression))
        } else {
            Ok(Node::Modified {
                modifier: Modifier::PackExpansion,
                operand: self.abi_type(pattern)?,
            })
        }
    }
    fn abi_parameter_level(
        &self,
        ty: TypeId,
    ) -> Result<crate::sema::lowering::TemplateParameterView, Error> {
        let mut parameter = self
            .graph
            .lower_parameter_level(ty)
            .ok_or(Error::Missing("ABI parameter level"))?;
        if !self.graph.lower_bound_parameter(ty) {
            parameter.level = if self.state.bound_pattern {
                parameter
                    .level
                    .checked_add(self.state.own_level)
                    .ok_or(Error::Semantic(crate::sema::Error::Limit(
                        "ABI parameter level",
                    )))?
            } else {
                0
            };
        }
        Ok(parameter)
    }
    fn finish_abi_type(&mut self, ty: TypeId, mut id: NodeId) -> Result<NodeId, Error> {
        if !matches!(self.graph.lower_type(ty)?, TypeView::Array { .. }) {
            if self.graph.lower_cv(ty) & 4 != 0 {
                let name = self.state.abi.string(b"_Atomic")?;
                id = self.state.abi.intern(
                    Node::Modified {
                        modifier: Modifier::Vendor(name),
                        operand: id,
                    },
                    &[],
                )?;
            }
            let mut cv = Qualifiers::default();
            if self.graph.lower_cv(ty) & 1 != 0 {
                cv = cv.union(Qualifiers::CONST);
            }
            if self.graph.lower_cv(ty) & 2 != 0 {
                cv = cv.union(Qualifiers::VOLATILE);
            }
            if !cv.is_empty() {
                id = self.state.abi.intern(
                    Node::Modified {
                        modifier: Modifier::Qualified(cv),
                        operand: id,
                    },
                    &[],
                )?;
            }
        }
        self.state
            .types
            .insert((ty, self.state.bound_pattern, self.state.own_level), id);
        Ok(id)
    }
    pub(super) fn demand_destruction(&mut self, entity: EntityId) -> Result<(), Error> {
        // Host objects demand the called complete entry. A vtable or a base
        // destruction call independently demands the other ABI entries.
        if self.host_objects() {
            self.demand(entity);
            return Ok(());
        }
        if self.graph.lower_virtual_slot(entity).is_some()
            || self
                .graph
                .lower_function(entity)?
                .object
                .is_some_and(|class| !self.graph.lower_virtual_bases(class).is_empty())
        {
            self.demand_entry(super::construction::Entry::DestructorBase(entity));
        }
        if self.graph.lower_virtual_slot(entity).is_some() {
            self.demand_entry(super::construction::Entry::DestructorDeleting(entity));
        }
        self.demand(entity);
        Ok(())
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
    pub(super) fn name(&mut self, entity: EntityId) -> Result<std::sync::Arc<str>, Error> {
        let entity = self.graph.lower_function_entity(entity);
        if let Some(name) = self.state.names.get(&entity) {
            return Ok(std::sync::Arc::clone(name));
        }
        if !self.state.ordinals.contains_key(&entity) {
            let facts = self.function_facts(entity)?;
            if matches!(facts.name, FunctionName::Constructor(_)) {
                let class = facts
                    .object
                    .ok_or(Error::Semantic(crate::sema::Error::Invariant(
                        "constructor without owner",
                    )))?;
                for (index, overload) in self.graph.lower_constructor_overloads(class).enumerate() {
                    self.state.ordinals.insert(overload, index + 1);
                }
            } else if matches!(facts.name, FunctionName::Conversion(_)) {
                let class = facts
                    .object
                    .ok_or(crate::sema::Error::Invariant("conversion without owner"))?;
                for (index, overload) in self.graph.lower_conversion_overloads(class).enumerate() {
                    self.state.ordinals.insert(overload, index + 1);
                }
            } else if !matches!(facts.name, FunctionName::Destructor(_)) {
                for (index, overload) in self.graph.lower_overloads(entity)?.enumerate() {
                    self.state.ordinals.insert(overload, index + 1);
                }
            }
        }
        let facts = self.function_facts(entity)?;
        let own = match facts.builtin {
            _ if self.graph.lower_is_closure_invoker(entity) => "_FUN".to_owned(),
            Some(crate::sema::lowering::BuiltinFunction::Memcmp) => "memcmp".to_owned(),
            Some(crate::sema::lowering::BuiltinFunction::Memset) => "memset".to_owned(),
            Some(crate::sema::lowering::BuiltinFunction::Library) => self
                .source_function_spelling(facts.name)?
                .strip_prefix("__builtin_")
                .ok_or(crate::sema::Error::Invariant("library builtin spelling"))?
                .to_owned(),
            Some(crate::sema::lowering::BuiltinFunction::LibraryNan(ty)) => match ty {
                F::Float => "nanf",
                F::LongDouble => "nanl",
                _ => "nan",
            }
            .to_owned(),
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
        let ordinal = self.state.ordinals.get(&entity).copied().unwrap_or(1);
        if ordinal > 1 && (!facts.c_linkage || facts.internal) {
            write!(name, "__ov{ordinal}").map_err(|_| {
                Error::Semantic(crate::sema::Error::Invariant("symbol formatting failure"))
            })?;
        }
        let name: std::sync::Arc<str> = name.into();
        self.state
            .names
            .insert(entity, std::sync::Arc::clone(&name));
        Ok(name)
    }
    pub(super) fn variable_symbol(
        &mut self,
        program: &mut Program,
        entity: EntityId,
    ) -> Result<SymbolId, Error> {
        if let Some(&Value::Symbol(symbol)) = self.state.callees.get(&entity) {
            self.request_variable_dependency(program, entity, symbol);
            return Ok(symbol);
        }
        let facts = self.graph.lower_variable(entity)?;
        let mut components = vec![if let Some(name) = facts.name {
            self.identifier(name)?.to_owned()
        } else {
            super::anonymous_names::decomposition_spelling(self.graph, self.cursor, entity)?
                .unwrap_or_else(|| "__anonymous_union_object".to_owned())
        }];
        let mut scope = facts.home;
        while let (Some(parent), name) = self.graph.lower_presentation_scope(scope) {
            components.push(self.presentation_scope_label(scope, name)?);
            scope = parent;
        }
        components.reverse();
        let spelling = components.join("__");
        // Ordinary named variables publish their ABI identity even when their
        // external spelling is direct. An explicit object label bypasses ABI
        // name construction, as variable_object does in the frozen owner.
        let named_identity = if (facts.name.is_some()
            || self.graph.lower_decomposition(entity).is_some())
            && self.graph.lower_asm_label(entity).is_none()
        {
            Some(self.variable_abi_name(entity)?)
        } else {
            None
        };
        let object = if let Some(label) = self.graph.lower_asm_label(entity) {
            Some(program.names.intern_bytes(label)?)
        } else if facts.name.is_none()
            && self.graph.lower_decomposition(entity).is_none()
            && self.graph.lower_first_named_member(facts.ty).is_none()
        {
            None
        } else if facts.name.is_some()
            && (facts.c_linkage
                || self.graph.lower_presentation_scope(facts.home).0.is_none()
                    && !facts.internal
                    && self.graph.lower_variable_arguments(entity).is_none()
                    && (!self.graph.lower_has_abi_tags()
                        || named_identity
                            .is_some_and(|name| self.state.abi.items(name).is_empty())))
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
            self.state.table.work.mangled_names += 1;
            abi::encoder::Encoder::encode(
                &self.state.abi,
                abi::encoder::Target::Entity(name),
                &mut bytes,
                self.state.budget,
            )?;
            Some(program.names.intern_bytes(&bytes)?)
        };
        let local_object = if facts.locality == crate::sema::lowering::Locality::Unit
            && (facts.c_linkage
                || object.is_some_and(|object| self.state.table.objects.contains_key(&object)))
        {
            None
        } else {
            object
        };
        let symbol = self.state.table.allocate(
            program,
            &spelling,
            object,
            facts.locality == crate::sema::lowering::Locality::Unit,
        )?;
        self.state.callees.insert(entity, Value::Symbol(symbol));
        self.state.objects.insert(entity, local_object);
        let metadata = self.variable_metadata(program, entity, symbol)?;
        program.set_forward_global(
            symbol,
            crate::lowir::model::ForwardGlobal {
                binding: metadata.binding,
                storage: metadata.storage,
                defined: facts.definition.is_some() && metadata.flags & 64 == 0,
            },
        );
        self.state.globals.push(entity);
        if facts.emission == crate::sema::lowering::Emission::Demanded
            && self.graph.lower_demanded_initializer(entity).is_some()
        {
            self.state.instance_variables.push_back(entity);
            let request = self.state.pending.variable();
            self.state.variable_requests.insert(entity, request);
            if let Some(owner) = self.state.variable_owner {
                self.state.variable_dependencies.insert((owner, entity));
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
        if let Some(owner) = self.state.variable_owner
            && self.state.variable_requests.contains_key(&entity)
            && !self.state.active_variables.contains(&entity)
            && program
                .global(program.symbol(symbol))
                .is_none_or(|g| matches!(g.init, crate::lowir::model::GlobalInit::Declaration))
            && self.state.variable_dependencies.insert((owner, entity))
        {
            // Its old reservation has no children until definition begins.
            // Fill a reservation under this owner so initializer-demanded
            // functions also precede the caller's later requests.
            let request = self.state.pending.variable();
            self.state.variable_requests.insert(entity, request);
            self.state.instance_variables.push_back(entity);
        }
    }
    pub(super) fn begin_variable_definition(&mut self, entity: EntityId) -> Option<EntityId> {
        self.state.active_variables.insert(entity);
        self.state.variable_owner.replace(entity)
    }
    pub(super) fn end_variable_definition(&mut self, entity: EntityId, previous: Option<EntityId>) {
        self.state.active_variables.remove(&entity);
        self.state.variable_owner = previous;
    }
    pub(super) fn variable_request(
        &self,
        entity: EntityId,
    ) -> Option<super::definition_demands::RequestId> {
        self.state.variable_requests.get(&entity).copied()
    }
    pub(super) fn budget(&self) -> StackBudget {
        self.state.budget
    }
    pub(super) fn instance_constructed(
        &mut self,
        program: &mut Program,
        entity: EntityId,
    ) -> Result<SymbolId, Error> {
        let global = self.variable_symbol(program, entity)?;
        let base = program.names.get(program.symbol(global).name).to_owned();
        let flag =
            self.state
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
        thread: bool,
    ) -> Result<SymbolId, Error> {
        let global = self.variable_symbol(program, entity)?;
        let base = program.names.get(program.symbol(global).name).to_owned();
        let name = self.variable_abi_name(entity)?;
        let mut bytes = Vec::new();
        self.state.table.work.mangled_names += 1;
        abi::encoder::Encoder::encode(
            &self.state.abi,
            abi::encoder::Target::GuardVariable(name),
            &mut bytes,
            self.state.budget,
        )?;
        let guard_name = program.names.intern_bytes(&bytes)?;
        let guard = self.state.table.allocate(
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
                    storage: thread.then_some(crate::lowir::vocabulary::Storage::ThreadLocal),
                    ..Metadata::default()
                },
                init: crate::lowir::model::GlobalInit::Zero,
            },
        )?;
        Ok(guard)
    }
    pub(super) fn variable_abi_name(&mut self, entity: EntityId) -> Result<NodeId, Error> {
        if let Some(&name) = self.state.variable_names.get(&entity) {
            return Ok(name);
        }
        let facts = self.graph.lower_variable(entity)?;
        let parent = self.scope(facts.home)?;
        let terminal = super::anonymous_names::variable_terminal(
            self.graph,
            self.cursor,
            &mut self.state.abi,
            entity,
        )?;
        if facts.c_linkage {
            let name = self.state.abi.name(None, terminal, &[])?;
            self.state.variable_names.insert(entity, name);
            return Ok(name);
        }
        let mut name = self.tagged_entity_name(
            Node::Name {
                parent,
                terminal,
                internal: (facts.name.is_some()
                    || self.graph.lower_decomposition(entity).is_some())
                    && facts.internal
                    && !self.graph.lower_anonymous_namespace(facts.home),
                hidden_tags: false,
                data_member: false,
                occurrence: 0,
            },
            Some(self.graph.lower_variable_abi_tag_owner(entity)),
        )?;
        if let Some(arguments) = self.graph.lower_variable_arguments(entity) {
            let mut nodes = Vec::with_capacity(arguments.len());
            for &argument in arguments {
                nodes.push(self.abi_type(argument)?);
            }
            name = self.state.abi.intern(Node::Template(name), &nodes)?;
        }
        name = self.with_implicit_tags(name, facts.ty)?;
        self.state.variable_names.insert(entity, name);
        Ok(name)
    }
    pub(super) fn existing_local_storage(&self, entity: EntityId) -> Option<Value> {
        self.state
            .local_statics
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
        tokens: Option<ast::TokenSpan<u32>>,
    ) -> Result<super::local_storage::StorageSymbols, Error> {
        use crate::lowir::vocabulary::{Binding, Storage};
        if let Some(&symbols) = self.state.local_statics.get(&entity) {
            return Ok(symbols);
        }
        let facts = self.graph.lower_variable(entity)?;
        let owner = self.function_symbol(program, function)?;
        if self
            .state
            .entries
            .contains_key(&super::construction::Entry::InlineCopy(function))
        {
            self.state.inline_static_owners.push(function);
        }
        let owner_name = program.names.get(program.symbol(owner).name).to_owned();
        let weak = self.metadata(program, function, &owner_name)?.binding == Some(Binding::Weak);
        let instantiated = self.graph.lower_function_instantiated(function);
        let owner_name =
            if instantiated || weak && self.graph.lower_static_owner_uses_abi_name(function) {
                let metadata = self.metadata(program, function, &owner_name)?;
                let object = metadata.object.ok_or(crate::sema::Error::Invariant(
                    "instantiated function without object name",
                ))?;
                let mut name = String::from("function_symbol_");
                for byte in program.names.get(object).bytes() {
                    write!(name, "{byte:02x}")
                        .map_err(|_| Error::Missing("static owner formatting"))?;
                }
                name
            } else {
                owner_name
            };
        if facts.name.is_none() && self.graph.lower_decomposition(entity).is_none() {
            let spelling = format!("__local_static__{owner_name}__decl{ordinal}__object");
            let object = self.state.table.allocate(program, &spelling, None, !weak)?;
            let metadata = Metadata {
                binding: Some(if weak {
                    Binding::Weak
                } else {
                    Binding::Internal
                }),
                section: self
                    .graph
                    .lower_variable_section(entity)
                    .map(|name| program.names.intern_bytes(name))
                    .transpose()?,
                ..Metadata::default()
            };
            let guard = if dynamic {
                Some((
                    self.state.table.allocate(
                        program,
                        &format!("{spelling}__guard"),
                        None,
                        !weak,
                    )?,
                    Metadata {
                        section: None,
                        ..metadata
                    },
                ))
            } else {
                None
            };
            let symbols = super::local_storage::StorageSymbols {
                object,
                metadata,
                guard,
            };
            self.state.callees.insert(entity, Value::Symbol(object));
            self.state.local_statics.insert(entity, symbols);
            return Ok(symbols);
        }
        let name = facts
            .name
            .map(|name| self.identifier(name))
            .transpose()?
            .unwrap_or("object");
        let spelling = if let Some(tokens) =
            tokens.filter(|span| !weak && !instantiated && span.end > span.begin)
        {
            format!(
                "__local_static__{owner_name}__{name}__tokens{}_{}",
                tokens.begin, tokens.end
            )
        } else {
            format!("__local_static__{owner_name}__decl{ordinal}__{name}")
        };
        let lexical = self.graph.lower_closure_body_owner(function);
        let encoding = self.function_encoding(lexical)?;
        let parent = self.state.abi.intern(
            Node::FunctionScope {
                encoding,
                default_argument: 0,
            },
            &[],
        )?;
        let terminal = super::anonymous_names::variable_terminal(
            self.graph,
            self.cursor,
            &mut self.state.abi,
            entity,
        )?;
        let occurrence = if let Some(identifier) = facts.name {
            let count = self
                .state
                .local_counts
                .entry((lexical, identifier))
                .or_default();
            let occurrence = *count;
            *count = count.checked_add(1).ok_or(crate::lowir::Error::Limit)?;
            occurrence
        } else {
            self.graph.lower_decomposition_discriminator(entity)
        };
        let name = self.tagged_entity_name(
            Node::Name {
                parent: Some(parent),
                terminal,
                internal: false,
                hidden_tags: false,
                data_member: false,
                occurrence,
            },
            Some(entity),
        )?;
        let name = self.with_implicit_tags(name, facts.ty)?;
        self.state.local_static_names.insert(entity, name);
        let mut bytes = Vec::new();
        if self.graph.lower_asm_label(entity).is_none() {
            self.state.table.work.mangled_names += 1;
        }
        abi::encoder::Encoder::encode(
            &self.state.abi,
            abi::encoder::Target::Entity(name),
            &mut bytes,
            self.state.budget,
        )?;
        let object = program
            .names
            .intern_bytes(self.graph.lower_asm_label(entity).unwrap_or(&bytes))?;
        let local_object =
            (weak || !self.state.table.objects.contains_key(&object)).then_some(object);
        // Each unit's parsed static owns storage identity, even when its weak
        // ABI name is shared. Unit-local copies can lower both declarations.
        let symbol = self
            .state
            .table
            .allocate(program, &spelling, Some(object), true)?;
        self.state.callees.insert(entity, Value::Symbol(symbol));
        let metadata = Metadata {
            binding: Some(if weak {
                Binding::Weak
            } else {
                Binding::Internal
            }),
            storage: facts.thread_local.then_some(Storage::ThreadLocal),
            object: local_object.filter(|&object| !program.names.get(object).is_empty()),
            section: self
                .graph
                .lower_variable_section(entity)
                .map(|name| program.names.intern_bytes(name))
                .transpose()?,
            ..Metadata::default()
        };
        let guard = if dynamic {
            bytes.clear();
            self.state.table.work.mangled_names += 1;
            abi::encoder::Encoder::encode(
                &self.state.abi,
                abi::encoder::Target::GuardVariable(name),
                &mut bytes,
                self.state.budget,
            )?;
            let object = program.names.intern_bytes(&bytes)?;
            let local_object =
                (weak || !self.state.table.objects.contains_key(&object)).then_some(object);
            let guard = self.state.table.allocate(
                program,
                &format!("{spelling}__guard"),
                Some(object),
                true,
            )?;
            Some((
                guard,
                Metadata {
                    object: local_object,
                    section: None,
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
        self.state.local_statics.insert(entity, symbols);
        Ok(symbols)
    }
    fn apply_weak_attribute(&self, entity: EntityId, metadata: &mut Metadata) {
        if self.graph.lower_symbol_weak(entity)
            && metadata.binding != Some(crate::lowir::vocabulary::Binding::Internal)
        {
            metadata.binding = Some(crate::lowir::vocabulary::Binding::Weak);
            metadata.flags |= 64;
        }
    }
    pub(super) fn variable_metadata(
        &self,
        program: &mut Program,
        entity: EntityId,
        symbol: SymbolId,
    ) -> Result<Metadata, Error> {
        let facts = self.graph.lower_variable(entity)?;
        let object = self
            .state
            .objects
            .get(&entity)
            .copied()
            .flatten()
            .filter(|&object| {
                !program.names.get(object).is_empty()
                    && (facts.locality == crate::sema::lowering::Locality::Unit
                        || object != program.symbol(symbol).name)
            });
        let mut metadata = Metadata {
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
            flags: if self.graph.lower_explicit_variable_root(entity) {
                4
            } else {
                0
            },
            ..Metadata::default()
        };
        self.apply_weak_attribute(entity, &mut metadata);
        metadata.section = self
            .graph
            .lower_variable_section(entity)
            .map(|name| program.names.intern_bytes(name))
            .transpose()?;
        Ok(metadata)
    }
    pub(super) fn metadata(
        &mut self,
        program: &mut crate::lowir::model::Program,
        entity: EntityId,
        name: &str,
    ) -> Result<Metadata, Error> {
        use crate::lowir::vocabulary::{Binding, Role};
        let entity = self.graph.lower_function_entity(entity);
        let facts = self.function_facts(entity)?;
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
        self.apply_weak_attribute(entity, &mut metadata);
        if self.graph.lower_no_inline(entity) {
            metadata.flags |= 32;
        }
        if self.state.rooted_destructors.contains(&entity) {
            metadata.flags |= 4;
        }
        if self.graph.lower_is_closure_invoker(entity) || self.graph.lower_function_used(entity) {
            metadata.flags |= 4;
        }
        if self
            .graph
            .lower_function_local_class(entity, self.object_mode())?
        {
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
                    if !self.state.table.object_mode {
                        metadata.role = Some(Role::Terminate);
                    }
                    metadata.object = Some(program.names.intern("abort")?);
                }
                B::Memcmp
                | B::Memset
                | B::Library
                | B::Unreachable
                | B::Expect
                | B::Integer(_)
                | B::Floating(_)
                | B::Stack(_)
                | B::Atomic(_) => {
                    metadata.linkage = Linkage::C;
                }
                B::LibraryNan(ty) => {
                    metadata.linkage = Linkage::C;
                    let object = match ty {
                        F::Float => "nanf",
                        F::LongDouble => "nanl",
                        _ => "nan",
                    };
                    if name != object {
                        metadata.object = Some(program.names.intern(object)?);
                    }
                }
                B::MakeComplex
                | B::Trap
                | B::SourceLocation(_)
                | B::Runtime(_)
                | B::Invoke
                | B::Allocation(_)
                | B::IsConstantEvaluated
                | B::ConstantP => {}
            }
            return Ok(metadata);
        }
        if let Some(allocation) = facts.allocation
            && (!allocation.array || !self.state.table.object_mode && facts.definition.is_none())
        {
            metadata.role = Some(match allocation.role {
                crate::sema::lowering::AllocationRole::Allocate => Role::AllocateMemory,
                crate::sema::lowering::AllocationRole::Free => Role::FreeMemory,
            });
        }
        if facts.c_linkage {
            metadata.linkage = Linkage::C;
        }
        if let Some(label) = self.graph.lower_asm_label(entity) {
            metadata.object = if let Some(&object) = self.state.objects.get(&entity) {
                object
            } else if label.is_empty() || label == name.as_bytes() {
                None
            } else {
                Some(program.names.intern_bytes(label)?)
            };
            return Ok(metadata);
        }
        if facts.c_linkage && !facts.internal {
            metadata.object = self.state.objects.get(&entity).copied().flatten();
            return Ok(metadata);
        }
        if let Some(&object) = self.state.objects.get(&entity) {
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
        self.state.table.work.mangled_names += 1;
        abi::encoder::Encoder::encode(
            &self.state.abi,
            abi::encoder::Target::Entity(function),
            &mut object,
            self.state.budget,
        )
        .map_err(Error::from)?;
        metadata.object = Some(program.names.intern_bytes(&object)?);
        Ok(metadata)
    }
    pub(super) fn demand_entry(&mut self, entry: super::construction::Entry) {
        if self.state.seen.insert(entry) {
            if let Some(entity) = entry.entity(self.graph)
                && !self.graph.lower_gnu_inline(entity)
                && let Some(owner) = self.graph.lower_enclosing_static_owner(entity)
            {
                self.demand(owner);
            }
            self.state.pending.function(entry);
        }
    }
    pub(super) fn has_entry_symbol(&self, entry: super::construction::Entry) -> bool {
        use super::construction::{Entry, Variant};
        self.state.entries.contains_key(&entry)
            || match entry {
                Entry::Function(entity) => self
                    .state
                    .callees
                    .contains_key(&self.graph.lower_function_entity(entity)),
                Entry::Constructor(id, Variant::Complete) => self
                    .graph
                    .lower_constructor_entity(id)
                    .is_some_and(|entity| self.state.callees.contains_key(&entity)),
                _ => false,
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
        if let Some(&symbol) = self.state.entries.get(&entry) {
            return Ok(symbol);
        }
        let (entity, variant) = match entry {
            Entry::Constructor(id, variant) => (self.graph.lower_constructor_entity(id), variant),
            Entry::DestructorBase(entity) => (Some(entity), Variant::Base),
            Entry::DestructorDeleting(entity) => (Some(entity), Variant::Deleting),
            Entry::Function(_) | Entry::InlineCopy(_) => return Err(Error::Missing("entry kind")),
        };
        if variant == Variant::Complete
            && let Some(entity) = entity
        {
            let symbol = self.function_symbol(program, entity)?;
            self.state.entries.insert(entry, symbol);
            return Ok(symbol);
        }
        let facts = self.entry_facts(entry)?;
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
                .is_some_and(|object| self.state.table.objects.contains_key(&object))
        {
            None
        } else {
            metadata.object
        };
        let symbol = self
            .state
            .table
            .allocate(program, &name, metadata.object, facts.internal)?;
        self.state.entry_objects.insert(entry, object);
        self.state.entries.insert(entry, symbol);
        if self.state.table.object_mode
            && !program.symbol(symbol).declared()
            && program.forward_boundary(symbol).is_none()
        {
            let signature =
                super::function::declaration_signature(program, self, self.graph, entry)?;
            program.set_forward_boundary(symbol, signature, metadata);
        }
        // An ancillary entry request may already have passed through the
        // worklist before this first actual use. Publish unavailable entries
        // here so a later synthesized body still gets its declaration.
        if facts.definition.is_none()
            && entity.is_some_and(|entity| {
                self.graph.lower_function_body(entity).is_none()
                    && !self.graph.lower_has_synthesized_body(entity)
            })
        {
            super::function::declare(program, self, self.graph, entry)?;
        }
        Ok(symbol)
    }
    pub(super) fn definition_metadata(
        &mut self,
        program: &mut Program,
        entry: super::construction::Entry,
        symbol: SymbolId,
    ) -> Result<Metadata, Error> {
        let name = program.names.get(program.symbol(symbol).name).to_owned();
        if let super::construction::Entry::InlineCopy(entity) = entry {
            return self.inline_copy_metadata(program, entity, symbol);
        }
        let mut metadata = self.entry_metadata(program, entry, &name)?;
        let facts = self.entry_facts(entry)?;
        let base = matches!(
            entry,
            super::construction::Entry::Constructor(_, super::construction::Variant::Base)
                | super::construction::Entry::DestructorBase(_)
        );
        if entry
            .entity(self.graph)
            .is_some_and(|entity| self.graph.lower_explicit_function_root(entity))
        {
            metadata.flags |= 4;
        }
        let special = matches!(
            facts.name,
            FunctionName::Constructor(_) | FunctionName::Destructor(_)
        );
        let shared_complete = special
            && !base
            && !matches!(entry, super::construction::Entry::DestructorDeleting(_))
            && facts
                .object
                .is_none_or(|class| self.graph.lower_virtual_bases(class).is_empty());
        let constexpr = entry
            .entity(self.graph)
            .is_some_and(|entity| self.graph.lower_constexpr(entity));
        if (base || facts.internal && shared_complete) && !constexpr
            || facts.internal
                && !matches!(facts.name, FunctionName::Destructor(_))
                && entry
                    .entity(self.graph)
                    .is_some_and(|entity| self.graph.lower_virtual_slot(entity).is_some())
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
        if let Entry::InlineCopy(entity) = entry {
            let symbol = self.state.entries[&entry];
            return self.inline_copy_metadata(program, entity, symbol);
        }
        let (entity, variant) = match entry {
            Entry::Constructor(id, variant) => (self.graph.lower_constructor_entity(id), variant),
            Entry::DestructorBase(entity) => (Some(entity), Variant::Base),
            Entry::DestructorDeleting(entity) => (Some(entity), Variant::Deleting),
            Entry::Function(_) | Entry::InlineCopy(_) => return Err(Error::Missing("entry kind")),
        };
        if variant == Variant::Complete
            && let Some(entity) = entity
        {
            return self.metadata(program, entity, name);
        }
        let facts = self.entry_facts(entry)?;
        let object = if let Some(&object) = self.state.entry_objects.get(&entry) {
            object
        } else if let Some(label) = entity.and_then(|entity| self.graph.lower_asm_label(entity)) {
            Some(program.names.intern_bytes(label)?)
        } else {
            let function = self.encode_function(&facts, variant)?;
            let mut object = Vec::new();
            self.state.table.work.mangled_names += 1;
            abi::encoder::Encoder::encode(
                &self.state.abi,
                abi::encoder::Target::Entity(function),
                &mut object,
                self.state.budget,
            )?;
            Some(program.names.intern_bytes(&object)?)
        };
        let mut metadata = Metadata {
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
            } | if facts
                .object
                .map(|class| self.graph.lower_class_no_linkage(class))
                .transpose()?
                .unwrap_or(false)
            {
                2
            } else {
                0
            },
            ..Metadata::default()
        };
        if let Some(entity) = entity {
            self.apply_weak_attribute(entity, &mut metadata);
        }
        Ok(metadata)
    }
    pub(super) fn finish_entry(
        &mut self,
        program: &mut Program,
        entry: super::construction::Entry,
        symbol: SymbolId,
    ) -> Result<(), Error> {
        use super::construction::{Entry, Variant};
        if entry
            .entity(self.graph)
            .is_some_and(|entity| self.graph.lower_asm_label(entity).is_some())
        {
            self.state.labeled_entries.insert(symbol, entry);
        }
        if let Some(function) = entry.entity(self.graph) {
            self.name_local_classes(function)?;
        }
        // HiddenBaseParameters: a class with virtual bases has distinct base
        // and complete ABI entries, even when only one is demanded here.
        if entry
            .facts(self.graph)?
            .object
            .is_some_and(|class| !self.graph.lower_virtual_bases(class).is_empty())
        {
            return Ok(());
        }
        if let Entry::Function(entity) = entry
            && matches!(
                self.function_facts(entity)?.name,
                FunctionName::Destructor(_)
            )
        {
            let object = self.base_entry_object(program, entry)?;
            self.state.families.note_alias(entry, object);
            self.state
                .table
                .entry_aliases
                .push(crate::lowir::model::ObjectAlias {
                    object,
                    target: symbol,
                });
        }
        if let Entry::Constructor(_, Variant::Complete) = entry {
            let object = self.base_entry_object(program, entry)?;
            self.state.families.note_alias(entry, object);
            self.state
                .table
                .entry_aliases
                .push(crate::lowir::model::ObjectAlias {
                    object,
                    target: symbol,
                });
        }
        Ok(())
    }
    // Explicit object labels bypass ABI-name work. Local-static guard names
    // remain separate and are still formed by their storage owner.
    fn base_entry_object(
        &mut self,
        program: &mut Program,
        entry: super::construction::Entry,
    ) -> Result<NameId, Error> {
        if let Some(label) = entry
            .entity(self.graph)
            .and_then(|entity| self.graph.lower_asm_label(entity))
        {
            return Ok(program.names.intern_bytes(label)?);
        }
        let facts = self.entry_facts(entry)?;
        let encoding = self.encode_function(&facts, super::construction::Variant::Base)?;
        let mut bytes = Vec::new();
        self.state.table.work.mangled_names += 1;
        abi::encoder::Encoder::encode(
            &self.state.abi,
            abi::encoder::Target::Entity(encoding),
            &mut bytes,
            self.state.budget,
        )?;
        Ok(program.names.intern_bytes(&bytes)?)
    }
    // An asm label can give one function's ABI variants the same symbol.
    // emit_definition still emits each requested variant. This provenance
    // permits rebuilding that symbol, never another entity's definition.
    pub(super) fn definition_replacement(
        &self,
        program: &Program,
        entry: super::construction::Entry,
        symbol: SymbolId,
    ) -> Result<bool, Error> {
        use super::construction::Entry;
        if !program.function_defined(symbol) {
            return Ok(false);
        }
        let shared = self
            .state
            .labeled_entries
            .get(&symbol)
            .is_some_and(|&previous| {
                previous != entry
                    && previous.entity(self.graph) == entry.entity(self.graph)
                    && matches!(
                        (previous, entry),
                        (Entry::Constructor(_, _), Entry::Constructor(_, _))
                            | (
                                Entry::DestructorBase(_),
                                Entry::Function(_) | Entry::DestructorDeleting(_)
                            )
                            | (Entry::DestructorDeleting(_), Entry::Function(_))
                    )
            });
        if shared {
            return Ok(true);
        }
        let declaration =
            self.entry_facts(entry)?
                .definition
                .ok_or(crate::sema::Error::Invariant(
                    "function definition without source",
                ))?;
        Err(Error::Semantic(crate::sema::Error::Diagnostic {
            location: self.tree.declarations.get(declaration).location,
            message: "redefinition of a function across translation units",
        }))
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
        if !self.graph.lower_virtual_bases(class).is_empty() {
            return Ok(false);
        }
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
        self.destructor_inert_in_context(entity, false)
    }
    fn destructor_inert_in_context(
        &mut self,
        entity: EntityId,
        contained: bool,
    ) -> Result<bool, Error> {
        self.check_stack()?;
        if let Some(&inert) = self.state.inert_destructors.get(&(entity, contained)) {
            return Ok(inert);
        }
        let graph = self.graph;
        let mut inert = self.destructor_body_inert(entity)?
            && graph
                .lower_function(entity)?
                .object
                .is_none_or(|class| contained || !graph.lower_polymorphic(class));
        if inert && let Some(class) = graph.lower_function(entity)?.object {
            for base in graph.lower_class_bases(class) {
                if let Some(child) = graph.lower_destructor_entity(base) {
                    inert &= self.destructor_inert_in_context(child, true)?;
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
                    inert &= self.destructor_inert_in_context(child, true)?;
                }
            }
        }
        self.state
            .inert_destructors
            .insert((entity, contained), inert);
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
            .any(|entry| self.state.seen.contains(entry))
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
                || !self.object_mode() && self.class_result_memory(class)?;
            if boundary.indirect {
                return Ok(boundary);
            }
            boundary.classes = if self.object_mode() {
                self.state
                    .object_classes
                    .classify(self.graph, ty, self.state.budget)?
            } else {
                self.parameter_classes(ty)?.filter(|classes| {
                    !matches!(classes, crate::lowir::vocabulary::ObjectClasses::Memory)
                })
            };
        } else if self.object_mode() {
            boundary.classes =
                self.state
                    .object_classes
                    .classify(self.graph, ty, self.state.budget)?;
        }
        boundary.ty = super::types::storage(self.graph, ty)?;
        Ok(boundary)
    }
}

#[cfg(test)]
pub(crate) use vtables::{
    assert_covariant_routes, assert_definition_destructor_targets, assert_hierarchy_flags,
    assert_table_demand,
};

#[cfg(test)]
pub(crate) use virtual_bases::assert_base_routes;

#[cfg(test)]
pub(crate) use vtables::assert_construction_context;

#[cfg(test)]
pub(crate) fn assert_label_name_demand(
    graph: &Graph,
    tree: &ast::Tree,
    cursor: &Cursor<'_>,
    declarations: crate::syntax::storage::List<ast::DeclId>,
) {
    let entities: Vec<_> = tree
        .decl_lists
        .get(declarations)
        .iter()
        .map(|&declaration| {
            let ast::DeclKind::Simple { declarators, .. } = tree.declarations.get(declaration).kind
            else {
                panic!("object declaration");
            };
            graph
                .lower_declared(tree.init_declarators.get(declarators)[0].declarator)
                .unwrap()
        })
        .collect();
    let syntax_count = tree.expressions.mark();
    let mut program = Program::default();
    let mut table = SymbolTable::default();
    let mut symbol_state = crate::lower::symbols::State::new(
        graph,
        StackBudget::default(),
        std::mem::take(&mut table),
    );
    let mut symbols = Symbols::new(graph, tree, cursor, &mut symbol_state);
    let first = symbols.variable_symbol(&mut program, entities[0]).unwrap();
    assert!(!symbols.variable_names.contains_key(&entities[0]));
    assert_eq!(
        symbols.variable_symbol(&mut program, entities[0]).unwrap(),
        first
    );
    symbols.variable_symbol(&mut program, entities[1]).unwrap();
    assert!(symbols.variable_names.contains_key(&entities[1]));
    assert_eq!(tree.expressions.mark(), syntax_count);
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

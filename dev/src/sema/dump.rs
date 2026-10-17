// Translated from sema/semantic_dump.cpp. See NOTICE.
//! Source-facing views borrow semantic facts; they are never phase transport.
use super::{
    graph::{BindingKind, Graph, ScopeKind},
    types::{RefKind, TypeId, TypeKind},
};
use crate::{
    lex::{identifiers::NameId, vocabulary::Keyword},
    syntax::cursor::Cursor,
};
use std::io::{self, Write};
impl Graph {
    pub(super) fn write_constructor_name(
        &self,
        class: super::graph::EntityId,
        cursor: &Cursor<'_>,
        output: &mut (impl Write + ?Sized),
        declaration: bool,
    ) -> io::Result<()> {
        self.write_class_qualifier(self.entity(class).home, cursor, output, declaration)?;
        self.write_class_name(class, cursor, output, false)?;
        output.write_all(b"::")?;
        if let Some(name) = self.entity(class).name {
            write_name(cursor, name, output)
        } else {
            self.write_class_name(class, cursor, output, false)
        }
    }
    pub(super) fn write_class_name(
        &self,
        class: super::graph::EntityId,
        cursor: &Cursor<'_>,
        output: &mut (impl Write + ?Sized),
        qualified: bool,
    ) -> io::Result<()> {
        let entity = self.entity(class);
        if qualified {
            self.write_class_qualifier(entity.home, cursor, output, true)?;
        }
        if let Some(ordinal) = entity.local_type_ordinal {
            write!(output, "__local_type{ordinal}")?;
        } else if let Some(name) = entity.name {
            write_name(cursor, name, output)?;
        } else if let Some(tokens) = entity
            .anonymous_class
            .and_then(super::anonymous_types::AnonymousClass::label)
        {
            write_anonymous_class(tokens, output)?;
        }
        Ok(())
    }
    pub(super) fn write_storage_name(
        &self,
        storage: super::graph::BindingId,
        output: &mut (impl Write + ?Sized),
    ) -> io::Result<()> {
        let ty = self
            .binding(storage)
            .ty
            .ok_or_else(|| io::Error::other("storage without type"))?;
        let TypeKind::Class(class) = self.types.get(ty).kind else {
            return Err(io::Error::other("anonymous storage without class"));
        };
        let tokens = self
            .entity(class)
            .anonymous_class
            .and_then(super::anonymous_types::AnonymousClass::label)
            .ok_or_else(|| io::Error::other("anonymous storage without provenance"))?;
        write!(
            output,
            "__anonymous_union_storage__{}_{}",
            tokens.begin, tokens.end
        )
    }
    pub(crate) fn print(
        &self,
        tree: &crate::syntax::ast::Tree,
        cursor: &Cursor<'_>,
        output: &mut (impl Write + ?Sized),
    ) -> io::Result<()> {
        writeln!(output, "translation-unit")?;
        let mut stack = vec![(self.global, 1usize)];
        while let Some((id, depth)) = stack.pop() {
            let scope = self.scope(id);
            indent(output, depth)?;
            let kind = match scope.kind {
                ScopeKind::Namespace => "namespace",
                ScopeKind::Enum => "enum",
                ScopeKind::Class => "class",
                ScopeKind::Function => "function",
                ScopeKind::Block => "block",
                ScopeKind::TemplateParameters => "template-parameters",
            };
            write!(output, "scope {kind}")?;
            if let Some(super::function_names::SpecialName::Operator { token, array }) = scope
                .owner
                .and_then(|id| self.entity(id).function)
                .filter(|props| props.allocation.is_some())
                .and_then(|props| props.special)
            {
                write!(
                    output,
                    " operator {}{}",
                    token.spelling(),
                    if array { "[]" } else { "" }
                )?;
            } else if let Some(name) = scope.owner.and_then(|id| self.entity(id).special_name) {
                output.write_all(b" ")?;
                crate::syntax::write_member_label(tree, cursor, name, output)?;
            } else if let Some(name) = scope.written_name {
                output.write_all(b" ")?;
                if scope.kind == ScopeKind::Function {
                    crate::syntax::write_member_label(tree, cursor, name, output)?;
                } else {
                    crate::syntax::write_expression_name(tree, cursor, name, output)?;
                }
            } else if let Some(name) = scope.name {
                output.write_all(b" ")?;
                write_name(cursor, name, output)?;
            } else if let Some(anonymous) = scope
                .owner
                .and_then(|owner| self.entity(owner).anonymous_class)
            {
                output.write_all(b" ")?;
                if let Some(tokens) = anonymous.label() {
                    write_anonymous_class(tokens, output)?;
                }
            } else if scope.kind == ScopeKind::Namespace {
                output.write_all(if id == self.global {
                    b" <global>"
                } else {
                    b" <unnamed>"
                })?;
            }
            writeln!(output)?;
            for &binding in &scope.bindings {
                let id = binding;
                let binding = self.binding(id);
                if binding.kind == BindingKind::TemplateValue {
                    continue;
                }
                if binding.kind == BindingKind::Type
                    && (self.entity(binding.entity).enumeration.is_some()
                        || self.entity(binding.entity).anonymous_class.is_some())
                    && self.entity(binding.entity).name.is_none()
                {
                    continue;
                }
                let label = match binding.kind {
                    BindingKind::Type => "type",
                    BindingKind::Alias => "type-alias",
                    BindingKind::Variable => "variable",
                    BindingKind::Function => "function",
                    BindingKind::Enumerator => "enumerator",
                    BindingKind::TemplateValue => "template-value",
                    BindingKind::Parameter => "parameter",
                    BindingKind::Namespace | BindingKind::NamespaceAlias => continue,
                };
                indent(output, depth + 1)?;
                write!(output, "{label} ")?;
                if let Some(&name) = self.special_labels.get(&id) {
                    crate::syntax::write_member_label(tree, cursor, name, output)?;
                } else if let Some(&name) = self.binding_labels.get(&id) {
                    crate::syntax::write_expression_name(tree, cursor, name, output)?;
                } else if let Some(name) = binding.name.or(self.entity(binding.entity).name) {
                    write_name(cursor, name, output)?;
                } else if let Some(super::function_names::SpecialName::Operator { token, array }) =
                    self.entity(binding.entity)
                        .function
                        .filter(|props| props.allocation.is_some())
                        .and_then(|props| props.special)
                {
                    write!(
                        output,
                        "operator{}{}",
                        token.spelling(),
                        if array { "[]" } else { "" }
                    )?;
                } else if let Some(ordinal) = self
                    .entity(binding.entity)
                    .enumeration
                    .and_then(|info| info.anonymous_ordinal)
                {
                    write!(output, "__anonymous_enum{ordinal}")?;
                }
                output.write_all(b" ")?;
                if binding.kind == BindingKind::Type {
                    if self.entity(binding.entity).template_parameter.is_some() {
                        self.write_type(
                            binding
                                .ty
                                .ok_or_else(|| io::Error::other("parameter without type"))?,
                            cursor,
                            output,
                        )?;
                    } else {
                        self.write_declared_type(id, tree, cursor, output)?;
                    }
                } else if binding.kind == BindingKind::Enumerator {
                    let declaration = self
                        .enum_declaration(id)
                        .ok_or_else(|| io::Error::other("enumerator without declaration"))?;
                    self.write_declared_type(declaration, tree, cursor, output)?;
                    let super::constants::ConstantState::Value(value) =
                        self.entity(binding.entity).constant
                    else {
                        return Err(io::Error::other("enumerator without value"));
                    };
                    let signed = self.integral_kind(value.ty).is_some_and(|kind| kind.signed);
                    if signed {
                        write!(output, " {}", super::integral::signed(value.bits))?;
                    } else {
                        write!(output, " {}", value.bits)?;
                    }
                } else {
                    let ty = if binding.kind == BindingKind::Variable {
                        self.entity(binding.entity).ty
                    } else {
                        binding.ty
                    };
                    self.write_type(
                        ty.ok_or_else(|| io::Error::other("binding without type"))?,
                        cursor,
                        output,
                    )?;
                }
                writeln!(output)?;
            }
            stack.extend(scope.children.iter().rev().map(|&id| (id, depth + 1)));
        }
        Ok(())
    }
    fn write_declared_type(
        &self,
        id: super::graph::BindingId,
        tree: &crate::syntax::ast::Tree,
        cursor: &Cursor<'_>,
        output: &mut (impl Write + ?Sized),
    ) -> io::Result<()> {
        let binding = self.binding(id);
        if binding.key == Keyword::Enum {
            output.write_all(
                if self
                    .entity(binding.entity)
                    .enumeration
                    .is_some_and(|info| info.scoped)
                {
                    b"enum class "
                } else {
                    b"enum "
                },
            )?;
        } else {
            write!(output, "{} ", class_key(binding.key))?;
        }
        if let Some(&name) = self.binding_labels.get(&id) {
            crate::syntax::write_expression_name(tree, cursor, name, output)?;
        } else if let Some(name) = self.entity(binding.entity).name {
            write_name(cursor, name, output)?;
        } else if let Some(ordinal) = self
            .entity(binding.entity)
            .enumeration
            .and_then(|info| info.anonymous_ordinal)
        {
            write!(output, "__anonymous_enum{ordinal}")?;
        }
        Ok(())
    }
    fn write_class_qualifier(
        &self,
        mut scope: Option<super::graph::ScopeId>,
        cursor: &Cursor<'_>,
        output: &mut (impl Write + ?Sized),
        types: bool,
    ) -> io::Result<()> {
        let mut parts = Vec::new();
        while let Some(id) = scope {
            let current = self.scope(id);
            if !matches!(
                current.kind,
                ScopeKind::Namespace
                    | ScopeKind::Class
                    | ScopeKind::Enum
                    | ScopeKind::TemplateParameters
            ) {
                break;
            }
            if matches!(current.kind, ScopeKind::Namespace | ScopeKind::Class)
                && current.parent.is_some()
            {
                parts.push(id);
            }
            scope = current.parent;
        }
        for scope in parts.into_iter().rev() {
            let node = self.scope(scope);
            if let Some(ordinal) = node.owner.and_then(|id| self.entity(id).local_type_ordinal) {
                write!(output, "__local_type{ordinal}")?;
            } else if let Some(name) = node.name {
                write_name(cursor, name, output)?;
            } else if node.kind == ScopeKind::Namespace {
                if !types {
                    continue;
                }
                output.write_all(b"_GLOBAL__N_1")?;
            } else if let Some(tokens) = node
                .owner
                .and_then(|id| self.entity(id).anonymous_class)
                .and_then(super::anonymous_types::AnonymousClass::label)
            {
                write_anonymous_class(tokens, output)?;
            }
            output.write_all(b"::")?;
        }
        Ok(())
    }
    pub(super) fn write_type(
        &self,
        ty: TypeId,
        cursor: &Cursor<'_>,
        output: &mut (impl Write + ?Sized),
    ) -> io::Result<()> {
        self.write_type_view(ty, cursor, output, false)
    }
    pub(super) fn write_semantic_type(
        &self,
        ty: TypeId,
        cursor: &Cursor<'_>,
        output: &mut (impl Write + ?Sized),
    ) -> io::Result<()> {
        self.write_type_view(ty, cursor, output, true)
    }
    fn write_type_view(
        &self,
        ty: TypeId,
        cursor: &Cursor<'_>,
        output: &mut (impl Write + ?Sized),
        qualified_classes: bool,
    ) -> io::Result<()> {
        enum Task {
            Type(TypeId),
            Text(&'static str),
        }
        let mut tasks = vec![Task::Type(ty)];
        while let Some(task) = tasks.pop() {
            let Task::Type(ty) = task else {
                if let Task::Text(text) = task {
                    output.write_all(text.as_bytes())?;
                }
                continue;
            };
            let node = self.types.get(ty);
            if node.cv.0 & 1 != 0 {
                output.write_all(b"const ")?;
            }
            if node.cv.0 & 2 != 0 {
                output.write_all(b"volatile ")?;
            }
            match &node.kind {
                TypeKind::Pack(elements) => {
                    output.write_all(b"pack of (")?;
                    tasks.push(Task::Text(")"));
                    for (index, &element) in elements.iter().enumerate().rev() {
                        tasks.push(Task::Type(element));
                        if index != 0 {
                            tasks.push(Task::Text(", "));
                        }
                    }
                }
                TypeKind::Expansion(pattern) => {
                    output.write_all(b"expansion of ")?;
                    tasks.push(Task::Type(*pattern));
                }
                TypeKind::BoundPack { arguments, .. } => {
                    output.write_all(b"bound ")?;
                    tasks.push(Task::Type(*arguments));
                }
                TypeKind::Value { ty, bits } => {
                    if self.address_parameter_type(*ty) {
                        output.write_all(b"nullptr")?;
                    } else if self.integral_kind(*ty).is_some_and(|kind| kind.signed) {
                        write!(output, "{}", super::integral::signed(*bits))?;
                    } else {
                        write!(output, "{bits}")?;
                    }
                }
                TypeKind::Auto => output.write_all(b"auto")?,
                TypeKind::Scalar(scalar) => output.write_all(scalar.name().as_bytes())?,
                TypeKind::Class(entity) => {
                    write!(output, "{} ", class_key(self.entity(*entity).key))?;
                    self.write_class_name(*entity, cursor, output, qualified_classes)?;
                }
                TypeKind::Template(entity) => {
                    output.write_all(b"template ")?;
                    self.write_class_name(*entity, cursor, output, false)?;
                }
                TypeKind::Enum(entity) => {
                    let entity = self.entity(*entity);
                    output.write_all(if entity.enumeration.is_some_and(|info| info.scoped) {
                        b"enum class "
                    } else {
                        b"enum "
                    })?;
                    if let Some(name) = entity.name {
                        write_name(cursor, name, output)?;
                    } else if let Some(ordinal) =
                        entity.enumeration.and_then(|info| info.anonymous_ordinal)
                    {
                        write!(output, "__anonymous_enum{ordinal}")?;
                    }
                }
                TypeKind::TemplateParameter { entity, template } => {
                    output.write_all(if *template {
                        b"template-parameter "
                    } else {
                        b"typename "
                    })?;
                    if let Some(name) = self.entity(*entity).name {
                        write_name(cursor, name, output)?;
                    }
                }
                TypeKind::Pointer(element) => {
                    output.write_all(b"pointer to ")?;
                    tasks.push(Task::Type(*element));
                }
                TypeKind::MemberPointer { owner, element } => {
                    output.write_all(b"member-pointer of ")?;
                    tasks.push(Task::Type(*element));
                    tasks.push(Task::Text(" to "));
                    tasks.push(Task::Type(*owner));
                }
                TypeKind::Reference { element, kind } => {
                    output.write_all(if *kind == RefKind::Lvalue {
                        b"lvalue-reference to "
                    } else {
                        b"rvalue-reference to "
                    })?;
                    tasks.push(Task::Type(*element));
                }
                TypeKind::Array { element, bound, .. } => {
                    write!(output, "array of {bound} ")?;
                    tasks.push(Task::Type(*element));
                }
                TypeKind::Function {
                    result,
                    parameters,
                    variadic,
                    cv,
                    reference,
                } => {
                    output.write_all(b"function of (")?;
                    tasks.push(Task::Type(*result));
                    tasks.push(Task::Text(" returning "));
                    if let Some(reference) = reference {
                        tasks.push(Task::Text(if *reference == RefKind::Lvalue {
                            " &"
                        } else {
                            " &&"
                        }));
                    }
                    if cv.0 & 2 != 0 {
                        tasks.push(Task::Text(" volatile"));
                    }
                    if cv.0 & 1 != 0 {
                        tasks.push(Task::Text(" const"));
                    }
                    tasks.push(Task::Text(")"));
                    if *variadic {
                        tasks.push(Task::Text(if parameters.is_empty() {
                            "..."
                        } else {
                            ", ..."
                        }));
                    }
                    for (index, &parameter) in parameters.iter().enumerate().rev() {
                        tasks.push(Task::Type(parameter));
                        if index != 0 {
                            tasks.push(Task::Text(", "));
                        }
                    }
                }
            }
        }
        Ok(())
    }
}
fn write_anonymous_class(
    tokens: crate::syntax::ast::TokenSpan,
    output: &mut (impl Write + ?Sized),
) -> io::Result<()> {
    write!(
        output,
        "__anonymous_union_type__{}_{}",
        tokens.begin, tokens.end
    )
}
fn class_key(key: Keyword) -> &'static str {
    match key {
        Keyword::Class => "class",
        Keyword::Union => "union",
        _ => "struct",
    }
}
fn write_name(
    cursor: &Cursor<'_>,
    name: NameId,
    output: &mut (impl Write + ?Sized),
) -> io::Result<()> {
    output.write_all(cursor.name(name))
}
fn indent(output: &mut (impl Write + ?Sized), depth: usize) -> io::Result<()> {
    for _ in 0..depth {
        output.write_all(b"  ")?;
    }
    Ok(())
}

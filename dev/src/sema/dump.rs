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
    pub(crate) fn print(
        &self,
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
                ScopeKind::Class => "class",
                ScopeKind::Function => "function",
                ScopeKind::Block => "block",
            };
            write!(output, "scope {kind}")?;
            if let Some(name) = scope.name {
                output.write_all(b" ")?;
                write_name(cursor, name, output)?;
            } else if scope.kind == ScopeKind::Namespace {
                output.write_all(if id == self.global {
                    b" <global>"
                } else {
                    b" <unnamed>"
                })?;
            }
            writeln!(output)?;
            for &binding in &scope.bindings {
                let binding = self.binding(binding);
                let label = match binding.kind {
                    BindingKind::Type => "type",
                    BindingKind::Alias => "type-alias",
                    BindingKind::Variable => "variable",
                    BindingKind::Function => "function",
                    BindingKind::Parameter => "parameter",
                    BindingKind::Namespace | BindingKind::NamespaceAlias => continue,
                };
                indent(output, depth + 1)?;
                write!(output, "{label} ")?;
                if let Some(name) = binding.name {
                    write_name(cursor, name, output)?;
                }
                output.write_all(b" ")?;
                if binding.kind == BindingKind::Type {
                    write!(output, "{} ", class_key(binding.key))?;
                    if let Some(name) = self.entity(binding.entity).name {
                        write_name(cursor, name, output)?;
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
    fn write_type(
        &self,
        ty: TypeId,
        cursor: &Cursor<'_>,
        output: &mut (impl Write + ?Sized),
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
                TypeKind::Scalar(scalar) => output.write_all(scalar.name().as_bytes())?,
                TypeKind::Class(entity) => {
                    let entity = self.entity(*entity);
                    write!(output, "{} ", class_key(entity.key))?;
                    if let Some(name) = entity.name {
                        write_name(cursor, name, output)?;
                    }
                }
                TypeKind::Pointer(element) => {
                    output.write_all(b"pointer to ")?;
                    tasks.push(Task::Type(*element));
                }
                TypeKind::Reference { element, kind } => {
                    output.write_all(if *kind == RefKind::Lvalue {
                        b"lvalue-reference to "
                    } else {
                        b"rvalue-reference to "
                    })?;
                    tasks.push(Task::Type(*element));
                }
                TypeKind::Array { element, bound } => {
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

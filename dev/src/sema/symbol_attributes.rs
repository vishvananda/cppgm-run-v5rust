// Translated from sema/analyzer_declarations.cpp::symbol_attributes. See NOTICE.
//! Entity-owned object attributes survive redeclarations; lowering reads facts only.
use super::{
    Error,
    analyzer::Analyzer,
    graph::{BindingKind, EntityId, Graph, ScopeKind},
    specifiers::{StaticSpecifier, ThreadStorage},
};
use crate::support::hash::HashMap;
use crate::syntax::{ast, storage::List};

#[derive(Default)]
struct Attributes {
    weak: bool,
    used: bool,
    section: Option<Box<[u8]>>,
    asm_label: Option<Box<[u8]>>,
}

#[derive(Default)]
pub(super) struct SymbolAttributes {
    values: HashMap<EntityId, Attributes>,
}

impl Analyzer<'_> {
    // analyzer_types.cpp::asm_label and new/agree_function/new_variable:
    // only the outer declarator contributes, and the first label wins. This
    // is spelling decoding, not constant evaluation of the literal's value.
    pub(super) fn note_asm_label(
        &mut self,
        entity: EntityId,
        expression: Option<ast::ExprId>,
    ) -> Result<(), Error> {
        let Some(expression) = expression else {
            return Ok(());
        };
        if self.graph.lower_asm_label(entity).is_some() {
            return Ok(());
        }
        let ast::ExprKind::Literal(literal) = self.tree.expressions.get(expression).kind else {
            return Err(Error::Invariant("asm label without string literal"));
        };
        let mut spelling = self
            .literal_bytes
            .get(literal.spelling(self.literal_bytes))
            .iter()
            .copied();
        let mut inside = false;
        let mut label = Vec::new();
        while let Some(byte) = spelling.next() {
            match byte {
                b'"' => inside = !inside,
                b'\\' if inside => {
                    if let Some(escaped) = spelling.next() {
                        label.push(escaped);
                    } else {
                        label.push(byte);
                    }
                }
                _ if inside => label.push(byte),
                _ => (),
            }
        }
        // The frozen store exposes a C string to object naming.
        if let Some(nul) = label.iter().position(|&byte| byte == 0) {
            label.truncate(nul);
        }
        self.graph
            .symbol_attributes
            .values
            .entry(entity)
            .or_default()
            .asm_label = Some(label.into_boxed_slice());
        Ok(())
    }
    pub(super) fn declarator_symbol_attributes(
        &mut self,
        declarator: ast::DeclaratorId,
        leading: List<ast::Attribute>,
    ) -> Result<(), Error> {
        let Some(binding) = self
            .graph
            .declared_bindings
            .get(declarator.index())
            .copied()
            .flatten()
        else {
            return Ok(());
        };
        let declared = binding;
        let binding = self.graph.binding(binding);
        // C++'s explicit variable-template path leaves init.declared null.
        // Rust retains that binding for other facts, but attributes still skip it.
        if self
            .graph
            .variable_instances
            .templates
            .explicit(binding.entity)
        {
            return Ok(());
        }
        self.apply_symbol_attributes(binding.entity, binding.kind, leading)?;
        let mut part = Some(declarator);
        while let Some(id) = part {
            let node = *self.tree.declarators.get(id);
            self.apply_symbol_attributes(binding.entity, binding.kind, node.attributes)?;
            part = node.nested;
        }
        self.declarator_abi_tags(declared, declarator, leading)
    }

    fn apply_symbol_attributes(
        &mut self,
        entity: EntityId,
        kind: BindingKind,
        list: List<ast::Attribute>,
    ) -> Result<(), Error> {
        for index in 0..self.tree.attributes.get(list).len() {
            let attribute = self.tree.attributes.get(list)[index];
            let weak = attribute.kind == ast::AttributeKind::Weak;
            if (!weak && attribute.kind != ast::AttributeKind::Section)
                || attribute.form == ast::AttributeForm::Standard
                    && attribute.site == ast::AttributeSite::Type
            {
                continue;
            }
            let variable = kind == BindingKind::Variable;
            if !variable && kind != BindingKind::Function {
                if !weak {
                    return Err(Error::at(
                        attribute.location,
                        "a section attribute on a declaration of no object",
                    ));
                }
                continue;
            }
            if !variable && !weak {
                continue;
            }
            let owner = self.graph.entity(entity);
            let home = self
                .graph
                .scope(
                    owner
                        .home
                        .ok_or(Error::Invariant("attributed entity without home"))?,
                )
                .kind;
            let (internal, static_storage, thread_local) = if variable {
                let object = owner
                    .variable
                    .ok_or(Error::Invariant("attributed variable without facts"))?;
                let thread_local = object.thread_storage == ThreadStorage::ThreadLocal;
                (
                    object.internal_linkage,
                    home == ScopeKind::Namespace
                        || object.static_specifier == StaticSpecifier::Present
                        || object.is_extern
                        || thread_local,
                    thread_local,
                )
            } else {
                (
                    owner
                        .function
                        .ok_or(Error::Invariant("attributed function without facts"))?
                        .internal_linkage,
                    true,
                    false,
                )
            };
            if weak {
                if internal
                    || !static_storage
                    || variable && !matches!(home, ScopeKind::Namespace | ScopeKind::Class)
                {
                    return Err(Error::at(
                        attribute.location,
                        "a weak declaration of an entity without external linkage",
                    ));
                }
                self.graph
                    .symbol_attributes
                    .values
                    .entry(entity)
                    .or_default()
                    .weak = true;
                continue;
            }
            if !static_storage {
                return Err(Error::at(
                    attribute.location,
                    "a section attribute on a variable without static storage duration",
                ));
            }
            if thread_local {
                return Err(Error::at(
                    attribute.location,
                    "a section attribute on a thread-local variable is not supported",
                ));
            }
            let expression = attribute
                .expression
                .ok_or(Error::Invariant("section without operand"))?;
            let ast::ExprKind::Literal(literal) = self.tree.expressions.get(expression).kind else {
                return Err(Error::Invariant("section without literal"));
            };
            let name =
                token_safe_section(self.literal_bytes.get(literal.spelling(self.literal_bytes)))
                    .ok_or_else(|| {
                        Error::at(
                            attribute.location,
                            "a section name outside ASCII letters, digits, `_` and `.`",
                        )
                    })?;
            let own = self
                .graph
                .symbol_attributes
                .values
                .entry(entity)
                .or_default();
            if own
                .section
                .as_deref()
                .is_some_and(|previous| previous != name)
            {
                return Err(Error::at(
                    attribute.location,
                    "a section attribute conflicting with an earlier declaration's",
                ));
            }
            if own.section.is_none() {
                own.section = Some(name.into_boxed_slice());
            }
        }
        Ok(())
    }
}

fn token_safe_section(spelling: &[u8]) -> Option<Vec<u8>> {
    let mut inside = false;
    let mut name = Vec::new();
    for &byte in spelling {
        if byte == b'"' {
            inside = !inside;
        } else if inside && (byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.')) {
            name.push(byte);
        } else if inside || !matches!(byte, b' ' | b'\t' | b'\n') {
            return None;
        }
    }
    (!name.is_empty()).then_some(name)
}

impl Graph {
    pub(super) fn note_function_used(&mut self, entity: EntityId) {
        self.symbol_attributes
            .values
            .entry(entity)
            .or_default()
            .used = true;
    }
    pub(crate) fn lower_function_used(&self, entity: EntityId) -> bool {
        // Function-template specialization copies effects/noinline but not
        // `used`; an instantiated class member receives its own declaration.
        self.symbol_attributes
            .values
            .get(&entity)
            .is_some_and(|own| own.used)
    }
    pub(crate) fn lower_asm_label(&self, entity: EntityId) -> Option<&[u8]> {
        self.symbol_attributes
            .values
            .get(&entity)
            .and_then(|own| own.asm_label.as_deref())
    }
    pub(crate) fn lower_symbol_weak(&self, entity: EntityId) -> bool {
        self.symbol_attributes
            .values
            .get(&entity)
            .is_some_and(|own| own.weak)
    }
    pub(crate) fn lower_variable_section(&self, entity: EntityId) -> Option<&[u8]> {
        if self.symbol_attributes.values.is_empty() {
            return None;
        }
        let mut current = Some(entity);
        while let Some(entity) = current {
            if let Some(section) = self
                .symbol_attributes
                .values
                .get(&entity)
                .and_then(|own| own.section.as_deref())
            {
                return Some(section);
            }
            current = self
                .variable_instances
                .member_source(entity)
                .or_else(|| self.variable_instances.templates.primary(entity))
                .filter(|&source| source != entity);
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        preprocess::unit::Preprocessor,
        support::{source::SourceBuffer, stack::StackBudget},
        syntax::SyntaxUnit,
    };

    #[test]
    fn used_is_an_entity_fact_with_frozen_specialization_and_lambda_rules() {
        let source = SourceBuffer::new(
            br"__attribute__((used)) int f(); int f(){return 7;}
                template<class T> __attribute__((used)) int g(){return sizeof(T);}
                int main(){auto l=[]() __attribute__((used,aligned(3))){return 1;};
                    return f()+g<int>()+l();}"
                .to_vec(),
        )
        .unwrap();
        let mut unit = SyntaxUnit::new(Preprocessor::with_name(&source, b"used-facts").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let f = cursor.intern_name(b"f").unwrap();
        let g = cursor.intern_name(b"g").unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let graph = analyzer.analyze(declarations).unwrap();
        let function = graph.functions_at(graph.global, f.into()).next().unwrap();
        assert!(graph.lower_function_used(graph.binding(function).entity));
        assert!(graph.function_templates.arguments.keys().any(|&entity| {
            graph.entity(entity).name == Some(g) && !graph.lower_function_used(entity)
        }));
        assert!(
            graph
                .closures
                .records
                .values()
                .any(|closure| { graph.lower_function_used(closure.call) })
        );
    }

    #[test]
    fn labels_belong_to_declared_entities_without_demanding_unused_types() {
        let source = SourceBuffer::new(
            br#"struct Unused { int a; };
                int f(int) asm("first"); int f(int) asm("second");
                int f(long) asm("long_" "label");
                template<class T>int h(T) asm("primary");
                template<class T>int h(T x){return x;}
                int use(){return h(3);}
                int x asm("object")=1; extern int x asm("later");
            "#
            .to_vec(),
        )
        .unwrap();
        let mut unit = SyntaxUnit::new(Preprocessor::with_name(&source, b"label-facts").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let f = cursor.intern_name(b"f").unwrap();
        let h = cursor.intern_name(b"h").unwrap();
        let x = cursor.intern_name(b"x").unwrap();
        let unused = cursor.intern_name(b"Unused").unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let graph = analyzer.analyze(declarations).unwrap();
        let labels: Vec<_> = graph
            .functions_at(graph.global, f.into())
            .map(|binding| {
                graph
                    .lower_asm_label(graph.binding(binding).entity)
                    .unwrap()
            })
            .collect();
        assert_eq!(labels, [b"first".as_slice(), b"long_label".as_slice()]);
        let object = graph.value_binding(graph.global, x.into()).unwrap();
        assert_eq!(
            graph.lower_asm_label(graph.binding(object).entity),
            Some(b"object".as_slice())
        );
        let mut instances = 0;
        for &entity in graph.function_templates.arguments.keys() {
            if graph.entity(entity).name == Some(h) {
                assert!(graph.lower_asm_label(entity).is_none());
                instances += 1;
            }
        }
        assert_eq!(instances, 1);
        let class = graph
            .scope(graph.global)
            .index
            .get(&unused)
            .unwrap()
            .ty
            .unwrap();
        assert!(matches!(
            graph.entity(graph.binding(class).entity).layout,
            super::super::layout::LayoutState::NotStarted
        ));
    }
}

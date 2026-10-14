// Translated from analyzer_declarations.cpp's holds_auto/deduced_type/replaced_auto.
// Deduction shares function_templates.cpp's argument matching. See NOTICE.
use super::{
    Error,
    analyzer::Analyzer,
    declarators::DeclaratorInfo,
    expression_facts::ValueCategory,
    graph::ScopeId,
    specifiers::{DeclarationRole, Specifiers},
    types::{Cv, TypeId, TypeKind},
};
use crate::{
    support::source::SourceOffset,
    syntax::ast::{self, ExprKind, InitializerKind},
};

#[derive(Default)]
pub(super) struct PlaceholderScratch {
    frames: Vec<(TypeKind, Cv)>,
}

impl Analyzer<'_> {
    pub(super) fn holds_auto(&self, mut ty: TypeId) -> bool {
        loop {
            match self.graph.types.get(ty).kind {
                TypeKind::Auto => return true,
                TypeKind::Pointer(element)
                | TypeKind::Reference { element, .. }
                | TypeKind::MemberPointer { element, .. } => ty = element,
                _ => return false,
            }
        }
    }

    pub(super) fn placeholder_declarator(
        &mut self,
        id: Option<ast::DeclaratorId>,
        specs: Specifiers,
        scope: ScopeId,
    ) -> Result<DeclaratorInfo, Error> {
        let mut environment = self.declarator_environment(id, scope)?;
        self.placeholder_environment(id, specs, &mut environment);
        self.declarator_in(id, specs.ty, environment)
    }

    pub(super) fn placeholder_environment(
        &self,
        id: Option<ast::DeclaratorId>,
        specs: Specifiers,
        environment: &mut super::declarators::DeclaratorEnvironment,
    ) {
        environment.auto_operators = specs.role != DeclarationRole::Typedef
            && matches!(self.graph.types.get(specs.ty).kind, TypeKind::Auto);
        if environment.auto_operators {
            let mut part = id;
            let mut own = None;
            while let Some(current) = part {
                let node = self.tree.declarators.get(current);
                if let Some(suffix) = self.tree.suffixes.get(node.suffixes).first() {
                    own = matches!(suffix, ast::Suffix::Function { .. }).then_some(current);
                } else if !node.pointers.is_empty() {
                    own = None;
                }
                part = node.nested;
            }
            environment.placeholder_function = own;
        }
    }

    pub(super) fn deduce_declared_object(
        &mut self,
        info: &mut DeclaratorInfo,
        specs: Specifiers,
        init: ast::InitDeclarator,
        home: ScopeId,
        location: SourceOffset,
        deduced: &mut Option<TypeId>,
    ) -> Result<(), Error> {
        if self.holds_auto(info.ty) {
            if self.graph.scope(home).kind == super::graph::ScopeKind::Class && !specs.is_static {
                return Err(Error::at(
                    location,
                    "auto in a non-static data member's declaration",
                ));
            }
            info.ty = self.deduce_object_type(
                info.ty,
                specs.auto_cv,
                init.initializer,
                info.lookup,
                self.tree.declarators.get(init.declarator).location,
                deduced,
            )?;
            super::record(
                &mut self.graph.declarator_types,
                init.declarator.index(),
                info.ty,
            );
        }
        Ok(())
    }

    pub(super) fn replace_auto(
        &mut self,
        declared: TypeId,
        replacing: TypeId,
        cv: Cv,
    ) -> Result<TypeId, Error> {
        self.placeholder_scratch.frames.clear();
        let result = self.replace_auto_inner(declared, replacing, cv);
        self.placeholder_scratch.frames.clear();
        result
    }

    fn replace_auto_inner(
        &mut self,
        mut declared: TypeId,
        replacing: TypeId,
        cv: Cv,
    ) -> Result<TypeId, Error> {
        loop {
            let node = self.graph.types.get(declared);
            match node.kind {
                TypeKind::Auto => break,
                TypeKind::Pointer(element)
                | TypeKind::Reference { element, .. }
                | TypeKind::MemberPointer { element, .. } => {
                    self.placeholder_scratch
                        .frames
                        .push((node.kind.clone(), node.cv));
                    declared = element;
                }
                _ => return Err(Error::Invariant("replacement in a type without auto")),
            }
        }
        let mut ty = self
            .graph
            .types
            .qualify(replacing, Cv(cv.0 | self.graph.types.get(declared).cv.0))?;
        while let Some((kind, cv)) = self.placeholder_scratch.frames.pop() {
            ty = match kind {
                TypeKind::Pointer(_) => self.graph.types.pointer(ty)?,
                TypeKind::Reference { kind, .. } => self.graph.types.reference(ty, kind)?,
                TypeKind::MemberPointer { owner, .. } => {
                    self.graph.types.member_pointer(owner, ty)?
                }
                _ => return Err(Error::Invariant("invalid auto replacement frame")),
            };
            ty = self.graph.types.qualify(ty, cv)?;
        }
        Ok(ty)
    }

    pub(super) fn deduce_object_type(
        &mut self,
        declared: TypeId,
        cv: Cv,
        initializer: Option<ast::Initializer>,
        scope: ScopeId,
        location: SourceOffset,
        known: &mut Option<TypeId>,
    ) -> Result<TypeId, Error> {
        let expression = initializer
            .and_then(|initializer| match initializer.kind {
                InitializerKind::Assign(id) | InitializerKind::Braced(id) => Some(id),
                InitializerKind::Parenthesized(arguments) => {
                    let arguments = self.tree.expr_lists.get(arguments);
                    (arguments.len() == 1).then(|| arguments[0])
                }
                _ => None,
            })
            .ok_or_else(|| {
                Error::at(
                    location,
                    "auto without an initializer of one expression (7.1.6.4p3)",
                )
            })?;
        if matches!(
            self.tree.expressions.get(expression).kind,
            ExprKind::Braced { .. }
        ) {
            return Err(Error::Missing("auto initializer-list deduction"));
        }
        let placeholder = self.graph.types.intern(TypeKind::Auto, Cv::default())?;
        let pattern = self.replace_auto(declared, placeholder, cv)?;
        let mut arguments = [None];
        let mut reuse = false;
        if self.semantic() && self.template_depth == 0 {
            let facts = self.typed_expression(expression, scope)?;
            if let Some(set) = facts.functions {
                // Bare auto first resolves an ordinary singleton, like typed_value.
                if matches!(self.graph.types.get(declared).kind, TypeKind::Auto) {
                    self.check_resolved_function(expression)?;
                    let facts = self.graph.expression_facts.get(expression)?;
                    self.deduce_call_argument(
                        pattern,
                        facts.ty,
                        facts.category,
                        0,
                        &mut arguments,
                    )?;
                } else {
                    self.deduce_overloaded_argument(pattern, set, 0, &mut arguments)?;
                }
            } else {
                if !self.deduce_call_argument(
                    pattern,
                    facts.ty,
                    facts.category,
                    0,
                    &mut arguments,
                )? {
                    arguments[0] = None;
                }
                reuse = true;
            }
        } else {
            let ty = self.decltype_type(expression, scope)?;
            let (ty, category) = match self.graph.types.get(ty).kind {
                TypeKind::Reference { element, kind } => (
                    element,
                    if kind == super::types::RefKind::Lvalue {
                        ValueCategory::Lvalue
                    } else {
                        ValueCategory::Prvalue
                    },
                ),
                _ => (
                    ty,
                    if matches!(self.tree.expressions.get(expression).kind, ExprKind::Id(_)) {
                        ValueCategory::Lvalue
                    } else {
                        ValueCategory::Prvalue
                    },
                ),
            };
            if !self.deduce_call_argument(pattern, ty, category, 0, &mut arguments)? {
                arguments[0] = None;
            }
        }
        let replacing = arguments[0].ok_or_else(|| {
            Error::at(
                location,
                "auto deduction fails for the initializer (7.1.6.4p6)",
            )
        })?;
        let canonical = self.graph.types.canonical_function(replacing)?;
        if known.is_some_and(|known| known != canonical) {
            return Err(Error::at(
                location,
                "auto deduced as different types by one declaration's declarators (7.1.6.4p7)",
            ));
        }
        *known = Some(canonical);
        let ty = self.replace_auto(declared, replacing, cv)?;
        if reuse {
            self.deduced_initializer = Some(expression);
        }
        Ok(ty)
    }
}

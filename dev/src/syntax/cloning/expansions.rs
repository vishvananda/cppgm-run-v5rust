// Translated from syntax/ast_clone.cpp::pattern_packs/bases/mem_initializers/
// expanded_arguments/expanded_exprs. See NOTICE.
//! Query-owned renaming while copying bound pack patterns into ordinary syntax.
use super::{
    Base, Cloner, ExprId, ExprKind, List, Lists, MemInitializer, NameKind, ParseError,
    TemplateArgument, TemplateArgumentKind,
};
use crate::{lex::identifiers::NameId, support::source::SourceOffset, syntax::ast};
use std::collections::{HashMap, HashSet};

pub(crate) trait PackExpander {
    fn pack_length(&self, name: NameId) -> Option<usize>;
    fn element_name(&mut self, name: NameId, index: usize) -> Result<NameId, ParseError>;
    fn mismatch(&mut self, location: SourceOffset) -> Result<(), ParseError>;
}

#[derive(Clone, Copy)]
enum Length {
    Bound(usize),
    Deferred,
}

#[derive(Clone, Copy)]
enum Shadow {
    Hidden,
    Pack(Length),
}

#[derive(Default)]
struct CollectedNames {
    uses: NameUses,
    seen: HashSet<NameId>,
    distinct: bool,
}

#[derive(Default)]
pub(crate) struct NameUses {
    pub names: Vec<NameId>,
    pub expansion: bool,
    pub pack_size: bool,
    pub this: bool,
}

#[derive(Default)]
pub(super) struct ExpansionState {
    collecting: Option<CollectedNames>,
    renames: HashMap<NameId, NameId>,
    shadows: HashMap<NameId, Shadow>,
    shadow_log: Vec<(NameId, Option<Shadow>)>,
    parameter_packs: Vec<(Option<NameId>, usize)>,
}
#[derive(Default)]
pub(crate) struct CopyState(ExpansionState);

enum ExpansionStore<'a> {
    Owned(Box<ExpansionState>),
    Borrowed(&'a mut ExpansionState),
}
pub(super) struct Expansion<'a> {
    expander: &'a mut dyn PackExpander,
    store: ExpansionStore<'a>,
}
impl std::ops::Deref for Expansion<'_> {
    type Target = ExpansionState;
    fn deref(&self) -> &Self::Target {
        match &self.store {
            ExpansionStore::Owned(state) => state,
            ExpansionStore::Borrowed(state) => state,
        }
    }
}
impl std::ops::DerefMut for Expansion<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match &mut self.store {
            ExpansionStore::Owned(state) => state,
            ExpansionStore::Borrowed(state) => state,
        }
    }
}
impl<'a> Expansion<'a> {
    pub(super) fn new(expander: &'a mut dyn PackExpander) -> Self {
        Self {
            expander,
            store: ExpansionStore::Owned(Box::default()),
        }
    }
    pub(super) fn borrowing(expander: &'a mut dyn PackExpander, state: &'a mut CopyState) -> Self {
        Self {
            expander,
            store: ExpansionStore::Borrowed(&mut state.0),
        }
    }
    fn length(&self, name: NameId) -> Option<Length> {
        if let Some(shadow) = self.shadows.get(&name) {
            match shadow {
                Shadow::Hidden => None,
                Shadow::Pack(length) => Some(*length),
            }
        } else if let Some((_, length)) = self
            .parameter_packs
            .iter()
            .find(|(id, _)| *id == Some(name))
        {
            Some(Length::Bound(*length))
        } else {
            self.expander.pack_length(name).map(Length::Bound)
        }
    }
    fn shadow(&mut self, name: NameId, shadow: Shadow) {
        let previous = self.shadows.insert(name, shadow);
        self.shadow_log.push((name, previous));
    }
    fn restore_shadows(&mut self, mark: usize) {
        while self.shadow_log.len() > mark {
            if let Some((name, previous)) = self.shadow_log.pop() {
                if let Some(previous) = previous {
                    self.shadows.insert(name, previous);
                } else {
                    self.shadows.remove(&name);
                }
            }
        }
    }
    fn rename_to(&mut self, names: &[NameId], index: usize) -> Result<(), ParseError> {
        self.renames.clear();
        for &name in names {
            let element = self.expander.element_name(name, index)?;
            self.renames.insert(name, element);
        }
        Ok(())
    }
}

#[derive(Clone, Copy)]
enum Pattern {
    Base(Base),
    Initializer(MemInitializer),
    Argument(TemplateArgument),
    Expression(ExprId),
    Type(ast::TypeId),
    Parameter(ast::Parameter),
}

impl Cloner<'_> {
    pub(super) fn closure_captures(
        &mut self,
        list: List<ast::LambdaCapture>,
    ) -> Result<List<ast::LambdaCapture>, ParseError> {
        if self.expansion.is_none() {
            return Ok(list);
        }
        let mut changes = false;
        for index in 0..self.tree.captures.get(list).len() {
            let capture = self.tree.captures.get(list)[index];
            let (ast::CaptureKind::Copy(name) | ast::CaptureKind::Reference(name)) = capture.kind
            else {
                continue;
            };
            if self.collecting() {
                if !capture.pack {
                    self.pack_name(name);
                }
            } else if let Some(state) = &self.expansion {
                changes |= if capture.pack {
                    matches!(state.length(name), Some(Length::Bound(_)))
                } else {
                    state.renames.contains_key(&name)
                };
            }
        }
        if !changes {
            return Ok(list);
        }
        let start = self.tree.captures.begin();
        for index in 0..self.tree.captures.get(list).len() {
            let capture = self.tree.captures.get(list)[index];
            let (ast::CaptureKind::Copy(name) | ast::CaptureKind::Reference(name)) = capture.kind
            else {
                self.tree.captures.push(capture);
                continue;
            };
            let state = self
                .expansion
                .as_mut()
                .ok_or(ParseError::Invariant("capture copy context"))?;
            let length = if capture.pack {
                state.length(name)
            } else {
                None
            };
            let make = |name, pack| ast::LambdaCapture {
                kind: if matches!(capture.kind, ast::CaptureKind::Reference(_)) {
                    ast::CaptureKind::Reference(name)
                } else {
                    ast::CaptureKind::Copy(name)
                },
                pack,
            };
            if let Some(Length::Bound(count)) = length {
                for element in 0..count {
                    let name = state.expander.element_name(name, element)?;
                    self.tree.captures.push(make(name, false));
                }
            } else {
                let name = if capture.pack {
                    name
                } else {
                    state.renames.get(&name).copied().unwrap_or(name)
                };
                self.tree.captures.push(make(name, capture.pack));
            }
        }
        self.tree.captures.finish(start)
    }
    pub(crate) fn select_pack(&mut self, name: NameId, index: usize) -> Result<(), ParseError> {
        if let Some(state) = self.expansion.as_mut() {
            let element = state.expander.element_name(name, index)?;
            state.renames.insert(name, element);
        }
        Ok(())
    }
    pub(super) fn collecting(&self) -> bool {
        self.expansion
            .as_ref()
            .is_some_and(|state| state.collecting.is_some())
    }
    pub(super) fn expanding(&self) -> bool {
        self.expansion
            .as_ref()
            .is_some_and(|state| state.collecting.is_none())
    }
    pub(super) fn pack_name(&mut self, name: NameId) -> NameId {
        let Some(state) = self.expansion.as_mut() else {
            return name;
        };
        let length = state.length(name);
        if let Some(names) = state.collecting.as_mut() {
            if length.is_some() && (!names.distinct || names.seen.insert(name)) {
                names.uses.names.push(name);
            }
            name
        } else {
            state.renames.get(&name).copied().unwrap_or(name)
        }
    }
    pub(super) fn shadow_template_packs(
        &mut self,
        parameters: ast::TemplateParameterClause,
    ) -> usize {
        let Some(state) = self.expansion.as_mut() else {
            return 0;
        };
        let mark = state.shadow_log.len();
        for parameter in self.tree.template_parameters.get(parameters.parameters) {
            if !parameter.pack {
                continue;
            }
            let name = match parameter.kind {
                ast::TemplateParameterKind::Type { name, .. } => name,
                ast::TemplateParameterKind::NonType { declarator, .. } => {
                    declarator_identifier(self.tree, declarator)
                }
            };
            if let Some(name) = name {
                state.shadow(name, Shadow::Pack(Length::Deferred));
            }
        }
        mark
    }
    pub(super) fn shadow_mark(&self) -> usize {
        self.expansion
            .as_ref()
            .map_or(0, |state| state.shadow_log.len())
    }
    pub(super) fn shadow_declarator(&mut self, declarator: Option<ast::DeclaratorId>) {
        if let Some(name) = declarator_identifier(self.tree, declarator)
            && let Some(state) = self.expansion.as_mut()
            && matches!(state.length(name), Some(Length::Bound(_)))
        {
            state.shadow(name, Shadow::Hidden);
        }
    }
    pub(super) fn shadow_declared(&mut self, id: ast::DeclId) {
        if let ast::DeclKind::Simple { declarators, .. } = self.tree.declarations.get(id).kind {
            for index in 0..self.tree.init_declarators.get(declarators).len() {
                let declarator = self.tree.init_declarators.get(declarators)[index].declarator;
                self.shadow_declarator(Some(declarator));
            }
        }
    }
    pub(super) fn restore_shadows(&mut self, mark: usize) {
        if let Some(state) = self.expansion.as_mut() {
            state.restore_shadows(mark);
        }
    }
    fn copy_pattern(&mut self, pattern: Pattern) -> Result<(), ParseError> {
        match pattern {
            Pattern::Base(node) => {
                self.base(node)?;
            }
            Pattern::Initializer(node) => {
                self.mem_initializer(node)?;
            }
            Pattern::Argument(node) => {
                self.argument(node)?;
            }
            Pattern::Expression(id) => {
                self.expression(id)?;
            }
            Pattern::Type(id) => {
                self.ty(id)?;
            }
            Pattern::Parameter(node) => {
                self.parameter(node)?;
            }
        }
        Ok(())
    }
    pub(crate) fn expression_pack_names(
        &mut self,
        expression: ExprId,
    ) -> Result<Vec<NameId>, ParseError> {
        self.collect_pattern_names(Pattern::Expression(expression), true)
            .map(|uses| uses.names)
    }
    pub(crate) fn expression_name_uses(
        &mut self,
        expression: ExprId,
    ) -> Result<NameUses, ParseError> {
        self.collect_pattern_names(Pattern::Expression(expression), false)
    }
    pub(super) fn collect_expression_use(&mut self, kind: ExprKind) {
        if let Some(names) = self
            .expansion
            .as_mut()
            .and_then(|state| state.collecting.as_mut())
        {
            names.uses.expansion |=
                matches!(kind, ExprKind::PackExpansion(_) | ExprKind::Fold { .. });
            names.uses.pack_size |= matches!(kind, ExprKind::SizeofPack(_));
            names.uses.this |= matches!(
                kind,
                ExprKind::Keyword(crate::lex::vocabulary::Keyword::This)
            );
        }
    }
    fn collect_pattern_names(
        &mut self,
        pattern: Pattern,
        distinct: bool,
    ) -> Result<NameUses, ParseError> {
        let state = self
            .expansion
            .as_mut()
            .ok_or(ParseError::Invariant("pack copy without context"))?;
        let saved = state.collecting.replace(CollectedNames {
            distinct,
            ..CollectedNames::default()
        });
        let renames = std::mem::take(&mut state.renames);
        let shadows = state.shadow_log.len();
        let mark = self.tree.mark();
        let result = self.copy_pattern(pattern);
        self.tree.rewind(&mark);
        let state = self
            .expansion
            .as_mut()
            .ok_or(ParseError::Invariant("pack copy lost context"))?;
        let names = state
            .collecting
            .take()
            .ok_or(ParseError::Invariant("pack copy lost collection"))?;
        state.collecting = saved;
        state.renames = renames;
        state.restore_shadows(shadows);
        result?;
        Ok(names.uses)
    }
    fn pattern_packs(
        &mut self,
        pattern: Pattern,
        location: SourceOffset,
    ) -> Result<Option<(Vec<NameId>, usize)>, ParseError> {
        if !self.expanding() {
            return Ok(None);
        }
        let names = self.collect_pattern_names(pattern, true)?.names;
        let state = self
            .expansion
            .as_mut()
            .ok_or(ParseError::Invariant("pack copy lost context"))?;
        let mut count = None;
        // Any pack declared in this copy defers the entire expansion, before
        // testing lengths of the bound packs that accompany it.
        if names.is_empty()
            || names
                .iter()
                .any(|&name| !matches!(state.length(name), Some(Length::Bound(_))))
        {
            return Ok(None);
        }
        for &name in &names {
            if let Some(Length::Bound(length)) = state.length(name) {
                if count.is_some_and(|count| count != length) {
                    state.expander.mismatch(location)?;
                }
                if count.is_none() {
                    count = Some(length);
                }
            }
        }
        Ok(count.map(|count| (names, count)))
    }
    // ast_clone.cpp::expanded_fold: expand into the ordinary syntax owned by
    // this instantiation; no second semantic tree or token replay is needed.
    pub(super) fn expanded_fold(
        &mut self,
        location: SourceOffset,
        mut op: ast::Operator,
        mut pattern: ExprId,
        mut init: Option<ExprId>,
        mut right: bool,
    ) -> Result<Option<ExprId>, ParseError> {
        let mut packs = self.pattern_packs(Pattern::Expression(pattern), location)?;
        if packs.is_none()
            && let Some(initializer) = init
        {
            packs = self.pattern_packs(Pattern::Expression(initializer), location)?;
            if packs.is_some() {
                init = Some(pattern);
                pattern = initializer;
                right = false;
            }
        }
        let Some((names, count)) = packs else {
            return Ok(None);
        };
        let saved = std::mem::take(
            &mut self
                .expansion
                .as_mut()
                .ok_or(ParseError::Invariant("fold copy without context"))?
                .renames,
        );
        let elements = (|| {
            let mut elements = Vec::with_capacity(count);
            for index in 0..count {
                self.expansion
                    .as_mut()
                    .ok_or(ParseError::Invariant("fold copy without context"))?
                    .rename_to(&names, index)?;
                elements.push(self.expression(pattern)?);
            }
            Ok::<_, ParseError>(elements)
        })();
        self.expansion
            .as_mut()
            .ok_or(ParseError::Invariant("fold copy lost context"))?
            .renames = saved;
        let mut elements = elements?;
        let mut accumulated = self.optional(init, Self::expression)?;
        if accumulated.is_none() && elements.is_empty() {
            return self.empty_fold(location, op);
        }
        // As in C++, a generated operator always uses its canonical spelling.
        op.alternative = false;
        let assignment = super::super::expressions::assignment_operator(op);
        if right {
            if accumulated.is_none() {
                accumulated = elements.pop();
            }
            elements.reverse();
        }
        for element in elements {
            accumulated = Some(if let Some(previous) = accumulated {
                let (left, right) = if right {
                    (element, previous)
                } else {
                    (previous, element)
                };
                self.tree.expressions.add(ast::Expression {
                    location,
                    kind: ExprKind::Binary {
                        op,
                        left,
                        right,
                        assignment,
                    },
                })?
            } else {
                element
            });
        }
        let inner = accumulated.ok_or(ParseError::Invariant("nonempty fold without expression"))?;
        self.tree
            .expressions
            .add(ast::Expression {
                location,
                kind: ExprKind::Paren(inner),
            })
            .map(Some)
    }
    fn empty_fold(
        &mut self,
        location: SourceOffset,
        op: ast::Operator,
    ) -> Result<Option<ExprId>, ParseError> {
        use crate::lex::{pp_lexer::Punctuator as P, vocabulary::Keyword as K};
        let kind = match op.token.as_punctuator() {
            Some(P::LogicalAnd) => ExprKind::Keyword(K::True),
            Some(P::LogicalOr) => ExprKind::Keyword(K::False),
            Some(P::Comma) => {
                let start = self.tree.specs.begin();
                self.tree.specs.push(ast::Specifier {
                    location,
                    kind: ast::SpecifierKind::Keyword(K::Void),
                });
                let specifiers = self.tree.specs.finish(start)?;
                ExprKind::FunctionalCast {
                    specifiers,
                    arguments: List::default(),
                    braced: None,
                }
            }
            _ => return Ok(None),
        };
        self.tree
            .expressions
            .add(ast::Expression { location, kind })
            .map(Some)
    }
    fn expanded_list<T: Copy>(
        &mut self,
        list: List<T>,
        store: fn(&mut ast::Tree) -> &mut Lists<T>,
        pattern: fn(&Self, T) -> Option<(Pattern, SourceOffset)>,
        each: fn(&mut Self, T, bool) -> Result<T, ParseError>,
    ) -> Result<List<T>, ParseError> {
        let start = store(self.tree).begin();
        for index in 0..store(self.tree).get(list).len() {
            let item = store(self.tree).get(list)[index];
            let expansion = pattern(self, item);
            if self.collecting() && expansion.is_some() {
                // A nested expansion owns its names.
                if let Some(names) = self
                    .expansion
                    .as_mut()
                    .and_then(|state| state.collecting.as_mut())
                {
                    names.uses.expansion = true;
                }
                store(self.tree).push(item);
                continue;
            }
            let expansion = expansion
                .map(|(pattern, location)| self.pattern_packs(pattern, location))
                .transpose()?
                .flatten();
            if let Some((names, count)) = expansion {
                let saved = std::mem::take(
                    &mut self
                        .expansion
                        .as_mut()
                        .ok_or(ParseError::Invariant("pack copy without context"))?
                        .renames,
                );
                let result: Result<(), ParseError> = (|| {
                    for index in 0..count {
                        let state = self
                            .expansion
                            .as_mut()
                            .ok_or(ParseError::Invariant("pack copy without context"))?;
                        state.rename_to(&names, index)?;
                        let copy = each(self, item, true)?;
                        store(self.tree).push(copy);
                    }
                    Ok(())
                })();
                self.expansion
                    .as_mut()
                    .ok_or(ParseError::Invariant("pack copy lost context"))?
                    .renames = saved;
                result?;
            } else {
                let copy = each(self, item, false)?;
                store(self.tree).push(copy);
            }
        }
        store(self.tree).finish(start)
    }
    pub(super) fn expanded_bases(&mut self, list: List<Base>) -> Result<List<Base>, ParseError> {
        self.expanded_list(
            list,
            |tree| &mut tree.bases,
            |_, node| node.pack.then_some((Pattern::Base(node), node.location)),
            |this, mut node, element| {
                if element {
                    node.pack = false;
                }
                this.base(node)
            },
        )
    }
    pub(super) fn expanded_initializers(
        &mut self,
        list: List<MemInitializer>,
    ) -> Result<List<MemInitializer>, ParseError> {
        self.expanded_list(
            list,
            |tree| &mut tree.mem_initializers,
            |_, node| {
                node.pack
                    .then_some((Pattern::Initializer(node), node.location))
            },
            |this, mut node, element| {
                if element {
                    node.pack = false;
                }
                this.mem_initializer(node)
            },
        )
    }
    pub(super) fn expanded_expressions(
        &mut self,
        list: List<ExprId>,
    ) -> Result<List<ExprId>, ParseError> {
        self.expanded_list(
            list,
            |tree| &mut tree.expr_lists,
            |this, id| {
                let node = this.tree.expressions.get(id);
                if let ExprKind::PackExpansion(pattern) = node.kind {
                    Some((Pattern::Expression(pattern), node.location))
                } else {
                    None
                }
            },
            |this, id, element| {
                let id = if element
                    && let ExprKind::PackExpansion(pattern) = this.tree.expressions.get(id).kind
                {
                    pattern
                } else {
                    id
                };
                this.expression(id)
            },
        )
    }
    pub(super) fn copied_arguments(
        &mut self,
        list: List<TemplateArgument>,
    ) -> Result<List<TemplateArgument>, ParseError> {
        self.expanded_list(
            list,
            |tree| &mut tree.template_arguments,
            |this, node| {
                let (pack, location) = match node.kind {
                    TemplateArgumentKind::Type(id) => {
                        let ty = this.tree.types.get(id);
                        (
                            ty.declarator
                                .is_some_and(|id| this.tree.declarators.get(id).pack),
                            ty.location,
                        )
                    }
                    TemplateArgumentKind::Expression(id) => {
                        (false, this.tree.expressions.get(id).location)
                    }
                };
                (node.pack || pack).then_some((Pattern::Argument(node), location))
            },
            |this, node, element| {
                let mut copy = this.argument(node)?;
                if element {
                    copy.pack = false;
                    if let TemplateArgumentKind::Type(id) = copy.kind {
                        this.clear_type_pack(id);
                    }
                }
                Ok(copy)
            },
        )
    }
    fn clear_type_pack(&mut self, id: ast::TypeId) {
        if let Some(declarator) = self.tree.types.get(id).declarator {
            let mut node = *self.tree.declarators.get(declarator);
            node.pack = false;
            self.tree.declarators.replace(declarator, node);
        }
    }
    pub(super) fn restore_parameter_packs(&mut self, mark: usize) {
        if let Some(state) = self.expansion.as_mut() {
            state.parameter_packs.truncate(mark);
        }
    }
    pub(super) fn for_declaration(
        &mut self,
        id: Option<ast::DeclId>,
    ) -> Result<Option<ast::DeclId>, ParseError> {
        let copy = id.map(|id| self.declaration(id)).transpose()?;
        if let Some(copy) = copy {
            self.shadow_declared(copy);
        }
        Ok(copy)
    }
    pub(crate) fn parameter_packs(&self) -> &[(Option<NameId>, usize)] {
        self.expansion
            .as_ref()
            .map_or(&[], |state| &state.parameter_packs)
    }

    pub(super) fn expanded_parameters(
        &mut self,
        list: List<ast::Parameter>,
    ) -> Result<List<ast::Parameter>, ParseError> {
        let start = self.tree.parameters.begin();
        for index in 0..self.tree.parameters.get(list).len() {
            let item = self.tree.parameters.get(list)[index];
            let expansion = if item
                .declarator
                .is_some_and(|id| self.tree.declarators.get(id).pack)
            {
                self.pattern_packs(Pattern::Parameter(item), item.location)?
            } else {
                None
            };
            if let Some((names, count)) = expansion {
                let name = declarator_identifier(self.tree, item.declarator);
                let state = self
                    .expansion
                    .as_mut()
                    .ok_or(ParseError::Invariant("parameter pack without context"))?;
                let saved = std::mem::take(&mut state.renames);
                let result = self.parameter_elements(item, &names, name, count);
                let state = self
                    .expansion
                    .as_mut()
                    .ok_or(ParseError::Invariant("parameter pack lost context"))?;
                state.renames = saved;
                result?;
                state.parameter_packs.push((name, count));
            } else {
                let copy = self.parameter(item)?;
                self.tree.parameters.push(copy);
            }
        }
        self.tree.parameters.finish(start)
    }
    fn parameter_elements(
        &mut self,
        item: ast::Parameter,
        names: &[NameId],
        name: Option<NameId>,
        count: usize,
    ) -> Result<(), ParseError> {
        for index in 0..count {
            let state = self
                .expansion
                .as_mut()
                .ok_or(ParseError::Invariant("parameter pack lost context"))?;
            state.rename_to(names, index)?;
            if let Some(name) = name {
                let element = state.expander.element_name(name, index)?;
                state.renames.insert(name, element);
            }
            let copy = self.parameter(item)?;
            if let Some(id) = copy.declarator {
                let mut node = *self.tree.declarators.get(id);
                node.pack = false;
                self.tree.declarators.replace(id, node);
            }
            self.tree.parameters.push(copy);
        }
        Ok(())
    }
    pub(super) fn expanded_types(
        &mut self,
        list: List<ast::TypeId>,
    ) -> Result<List<ast::TypeId>, ParseError> {
        self.expanded_list(
            list,
            |tree| &mut tree.type_lists,
            |this, id| {
                let node = this.tree.types.get(id);
                node.declarator
                    .is_some_and(|id| this.tree.declarators.get(id).pack)
                    .then_some((Pattern::Type(id), node.location))
            },
            |this, id, element| {
                let copy = this.ty(id)?;
                if element {
                    this.clear_type_pack(copy);
                }
                Ok(copy)
            },
        )
    }
}

fn declarator_identifier(
    tree: &ast::Tree,
    mut declarator: Option<ast::DeclaratorId>,
) -> Option<NameId> {
    while let Some(id) = declarator {
        let node = tree.declarators.get(id);
        if let Some(id) = node.name {
            let node = tree.names.get(id);
            if !node.global
                && tree.name_lists.get(node.components).len() == 1
                && let NameKind::Identifier(id) = tree.name_lists.get(node.components)[0].kind
            {
                return Some(id);
            }
        }
        declarator = node.nested;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Packs;
    impl PackExpander for Packs {
        fn mismatch(&mut self, _: SourceOffset) -> Result<(), ParseError> {
            Ok(())
        }

        fn pack_length(&self, _: NameId) -> Option<usize> {
            Some(3)
        }
        fn element_name(&mut self, name: NameId, _: usize) -> Result<NameId, ParseError> {
            Ok(name)
        }
    }

    struct EmptyOrRepeatedPacks(usize);
    impl PackExpander for EmptyOrRepeatedPacks {
        fn mismatch(&mut self, _: SourceOffset) -> Result<(), ParseError> {
            Ok(())
        }

        fn pack_length(&self, _: NameId) -> Option<usize> {
            Some(self.0)
        }
        fn element_name(&mut self, name: NameId, _: usize) -> Result<NameId, ParseError> {
            Ok(name)
        }
    }

    #[test]
    fn a_copy_session_retains_parameter_packs_across_borrowing_facades() {
        use crate::{
            preprocess::unit::Preprocessor,
            support::{source::SourceBuffer, stack::StackBudget},
            syntax::{
                SyntaxUnit,
                render::{Render, Renderer},
            },
        };
        struct TypePack(NameId);
        impl PackExpander for TypePack {
            fn pack_length(&self, name: NameId) -> Option<usize> {
                (name == self.0).then_some(3)
            }
            fn element_name(&mut self, name: NameId, _: usize) -> Result<NameId, ParseError> {
                Ok(name)
            }
            fn mismatch(&mut self, _: SourceOffset) -> Result<(), ParseError> {
                Ok(())
            }
        }
        let source = SourceBuffer::new(b"void f(T... args){sink(args...);}".to_vec()).unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"retained-copy-session").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let name = cursor.intern_name(b"T").unwrap();
        let ast::DeclKind::Function {
            declarator, body, ..
        } = tree
            .declarations
            .get(tree.decl_lists.get(declarations)[0])
            .kind
        else {
            panic!("function")
        };
        let ast::Suffix::Function { parameters, .. } =
            tree.suffixes.get(tree.declarators.get(declarator).suffixes)[0]
        else {
            panic!("parameters")
        };
        let mut packs = TypePack(name);
        let mut state = CopyState::default();
        let marker = 0_u8;
        let base = std::ptr::from_ref(&marker).addr();
        let parameters = Cloner::new(tree, StackBudget::default(), base)
            .with_expander_state(&mut packs, &mut state)
            .parameters(parameters)
            .unwrap();
        assert_eq!(tree.parameters.get(parameters.parameters).len(), 3);
        let body = Cloner::new(tree, StackBudget::default(), base)
            .with_expander_state(&mut packs, &mut state)
            .body(body)
            .unwrap();
        let ast::Body::Parsed(body) = tree.bodies.get(body) else {
            panic!("parsed body")
        };
        let mut renderer = Renderer::new(tree, cursor);
        renderer.render(Render::Statement(body.compound)).unwrap();
        assert_eq!(renderer.bytes, b"{sink(args,args,args);}");
    }

    #[test]
    fn a_soft_pack_mismatch_finishes_the_copy_using_the_first_length() {
        use crate::{
            preprocess::unit::Preprocessor,
            support::{source::SourceBuffer, stack::StackBudget},
            syntax::{
                SyntaxUnit,
                render::{Render, Renderer},
            },
        };
        struct UnequalPacks {
            names: [NameId; 3],
            mismatches: Vec<SourceOffset>,
        }
        impl PackExpander for UnequalPacks {
            fn pack_length(&self, name: NameId) -> Option<usize> {
                self.names
                    .iter()
                    .position(|&known| known == name)
                    .map(|index| if index == 0 { 2 } else { 3 })
            }
            fn element_name(&mut self, name: NameId, _: usize) -> Result<NameId, ParseError> {
                Ok(name)
            }
            fn mismatch(&mut self, location: SourceOffset) -> Result<(), ParseError> {
                self.mismatches.push(location);
                Ok(())
            }
        }
        let source =
            SourceBuffer::new(b"void f(){sink(triple<A,B,C>()...);tail();}".to_vec()).unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"soft-copy-mismatch").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let names = [b"A".as_slice(), b"B", b"C"].map(|name| cursor.intern_name(name).unwrap());
        let ast::DeclKind::Function { body, .. } = tree
            .declarations
            .get(tree.decl_lists.get(declarations)[0])
            .kind
        else {
            panic!("function")
        };
        let mut packs = UnequalPacks {
            names,
            mismatches: Vec::new(),
        };
        let marker = 0_u8;
        let copy = Cloner::new(
            tree,
            StackBudget::default(),
            std::ptr::from_ref(&marker).addr(),
        )
        .with_expander(&mut packs)
        .body(body)
        .unwrap();
        assert_eq!(packs.mismatches.len(), 2);
        assert_eq!(packs.mismatches[0], packs.mismatches[1]);
        let ast::Body::Parsed(copy) = tree.bodies.get(copy) else {
            panic!("parsed body")
        };
        let mut renderer = Renderer::new(tree, cursor);
        renderer.render(Render::Statement(copy.compound)).unwrap();
        assert_eq!(
            renderer.bytes,
            b"{sink(triple<A,B,C>(),triple<A,B,C>());tail();}"
        );
    }

    #[test]
    fn expanded_clause_presence_survives_empty_base_and_constructor_packs() {
        use crate::{
            preprocess::unit::Preprocessor,
            support::{source::SourceBuffer, stack::StackBudget},
            syntax::{
                SyntaxUnit, printer,
                render::{Render, Renderer},
            },
        };
        for (count, expected) in [
            (0, "struct C:{C():{}};"),
            (1, "struct C:B{C():B(){}};"),
            (2, "struct C:B,B{C():B(),B(){}};"),
        ] {
            let source =
                SourceBuffer::new(b"template<class... B>struct C:B...{C():B()...{}};".to_vec())
                    .unwrap();
            let mut unit =
                SyntaxUnit::new(Preprocessor::with_name(&source, b"expanded-clauses").unwrap());
            unit.parse().unwrap();
            let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
            let ast::DeclKind::Template { declaration, .. } = tree
                .declarations
                .get(tree.decl_lists.get(declarations)[0])
                .kind
            else {
                panic!("template class")
            };
            let marker = 0u8;
            let mut packs = EmptyOrRepeatedPacks(count);
            let copy = Cloner::new(
                tree,
                StackBudget::default(),
                std::ptr::from_ref(&marker).addr(),
            )
            .with_expander(&mut packs)
            .declaration(declaration)
            .unwrap();
            let ast::DeclKind::Class(specifier) = tree.declarations.get(copy).kind else {
                panic!("class")
            };
            let ast::SpecifierKind::Class(class) = specifier.kind else {
                panic!("class specifier")
            };
            let class = tree.classes.get(class);
            assert!(class.has_base_clause);
            assert_eq!(tree.bases.get(class.bases).len(), count);
            let ast::DeclKind::SpecialMember {
                body: Some(body), ..
            } = tree
                .declarations
                .get(tree.decl_lists.get(class.members)[0])
                .kind
            else {
                panic!("constructor")
            };
            let ast::Body::Parsed(body) = tree.bodies.get(body) else {
                panic!("parsed body")
            };
            assert!(body.initializer_location.is_some());
            assert_eq!(tree.mem_initializers.get(body.initializers).len(), count);
            let mut renderer = Renderer::new(tree, cursor);
            renderer.render(Render::Declaration(copy)).unwrap();
            assert_eq!(renderer.bytes, expected.as_bytes());
            let start = tree.decl_lists.begin();
            tree.decl_lists.push(copy);
            let declarations = tree.decl_lists.finish(start).unwrap();
            let mut bytes = Vec::new();
            printer::print(tree, cursor, declarations, &mut bytes).unwrap();
            let dump = String::from_utf8(bytes).unwrap();
            assert!(dump.contains("base-clause\n"));
            assert!(dump.contains("ctor-initializer\n"));
            assert_eq!(dump.matches("base-specifier").count(), count);
            assert_eq!(dump.matches("mem-initializer\n").count(), count);
        }
    }

    #[test]
    fn name_use_queries_retain_occurrences_and_nested_syntax_without_retaining_copies() {
        use crate::{
            preprocess::unit::Preprocessor,
            support::{source::SourceBuffer, stack::StackBudget},
            syntax::SyntaxUnit,
        };
        let source =
            SourceBuffer::new(b"void f(){unknown(x,x);this;sizeof...(Ts);g<Ts...>();}".to_vec())
                .unwrap();
        let mut unit = SyntaxUnit::new(Preprocessor::with_name(&source, b"name-uses").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let ast::DeclKind::Function { body, .. } = tree
            .declarations
            .get(tree.decl_lists.get(declarations)[0])
            .kind
        else {
            unreachable!()
        };
        let ast::Body::Parsed(body) = tree.bodies.get(body) else {
            unreachable!()
        };
        let ast::StmtKind::Compound(statements) = tree.statements.get(body.compound).kind else {
            unreachable!()
        };
        let statements = tree.stmt_lists.get(statements).to_vec();
        let marker = 0u8;
        let base = std::ptr::from_ref(&marker).addr();
        let mut packs = Packs;
        let expressions = tree.expressions.mark();
        let name_count = tree.names.mark();
        for (index, statement) in statements.into_iter().enumerate() {
            let ast::StmtKind::Expression(Some(expression)) = tree.statements.get(statement).kind
            else {
                unreachable!()
            };
            let uses = Cloner::new(tree, StackBudget::default(), base)
                .with_expander(&mut packs)
                .expression_name_uses(expression)
                .unwrap();
            assert_eq!(
                (uses.this, uses.pack_size, uses.expansion),
                (index == 1, index == 2, index == 3)
            );
            let names: Vec<_> = uses.names.iter().map(|&name| cursor.name(name)).collect();
            if index == 0 {
                assert_eq!(names, [b"unknown".as_slice(), b"x", b"x"]);
            } else if index == 3 {
                assert_eq!(names, [b"g".as_slice()]);
            } else {
                assert!(names.is_empty());
            }
            assert_eq!(tree.expressions.mark(), expressions);
            assert_eq!(tree.names.mark(), name_count);
        }
    }

    #[test]
    fn nested_shadows_restore_deferred_bound_and_absent_entries() {
        let mut identifiers = crate::lex::identifiers::Identifiers::default();
        let name = identifiers.intern(b"T").unwrap();
        let mut packs = Packs;
        let mut expansion = Expansion::new(&mut packs);
        assert!(matches!(expansion.length(name), Some(Length::Bound(3))));
        expansion.shadow(name, Shadow::Pack(Length::Deferred));
        let mark = expansion.shadow_log.len();
        expansion.shadow(name, Shadow::Hidden);
        assert!(expansion.length(name).is_none());
        expansion.restore_shadows(mark);
        assert!(matches!(expansion.length(name), Some(Length::Deferred)));
        expansion.restore_shadows(0);
        assert!(expansion.shadows.is_empty());
        assert!(matches!(expansion.length(name), Some(Length::Bound(3))));
    }
}

// Translated from syntax/ast_clone.cpp::pattern_packs/bases/mem_initializers/
// expanded_arguments/expanded_exprs. See NOTICE.
//! Query-owned renaming while copying bound pack patterns into ordinary syntax.
use super::{
    Base, Cloner, ExprId, ExprKind, List, Lists, MemInitializer, NameKind, ParseError,
    TemplateArgument, TemplateArgumentKind,
};
use crate::{lex::identifiers::NameId, support::source::SourceOffset, syntax::ast};
use std::collections::HashMap;

pub(crate) trait PackExpander {
    fn pack_length(&self, name: NameId) -> Option<usize>;
    fn element_name(&mut self, name: NameId, index: usize) -> Result<NameId, ParseError>;
}

#[derive(Clone, Copy)]
enum Length {
    Bound(usize),
    Deferred,
}

pub(super) struct Expansion<'a> {
    expander: &'a mut dyn PackExpander,
    collecting: Option<Vec<NameId>>,
    renames: HashMap<NameId, NameId>,
    shadows: Vec<(NameId, Option<Length>)>,
    parameter_packs: Vec<(Option<NameId>, usize)>,
}
impl<'a> Expansion<'a> {
    pub(super) fn new(expander: &'a mut dyn PackExpander) -> Self {
        Self {
            expander,
            collecting: None,
            renames: HashMap::new(),
            shadows: Vec::new(),
            parameter_packs: Vec::new(),
        }
    }
    fn length(&self, name: NameId) -> Option<Length> {
        if let Some((_, length)) = self.shadows.iter().rev().find(|(id, _)| *id == name) {
            *length
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
    fn rename_to(&mut self, names: &[NameId], index: usize) -> Result<(), ParseError> {
        self.renames.clear();
        for &name in names {
            self.renames
                .insert(name, self.expander.element_name(name, index)?);
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
    pub(crate) fn select_pack(&mut self, name: NameId, index: usize) -> Result<(), ParseError> {
        if let Some(state) = self.expansion.as_mut() {
            state
                .renames
                .insert(name, state.expander.element_name(name, index)?);
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
            if length.is_some() && !names.contains(&name) {
                names.push(name);
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
        let mark = state.shadows.len();
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
                state.shadows.push((name, Some(Length::Deferred)));
            }
        }
        mark
    }
    pub(super) fn shadow_mark(&self) -> usize {
        self.expansion
            .as_ref()
            .map_or(0, |state| state.shadows.len())
    }
    pub(super) fn shadow_declarator(&mut self, declarator: Option<ast::DeclaratorId>) {
        if let Some(name) = declarator_identifier(self.tree, declarator)
            && let Some(state) = self.expansion.as_mut()
            && matches!(state.length(name), Some(Length::Bound(_)))
        {
            state.shadows.push((name, None));
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
            state.shadows.truncate(mark);
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
    fn pattern_packs(
        &mut self,
        pattern: Pattern,
        location: SourceOffset,
    ) -> Result<Option<(Vec<NameId>, usize)>, ParseError> {
        if !self.expanding() {
            return Ok(None);
        }
        let state = self
            .expansion
            .as_mut()
            .ok_or(ParseError::Invariant("pack copy without context"))?;
        let saved = state.collecting.replace(Vec::new());
        let renames = std::mem::take(&mut state.renames);
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
        result?;
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
                    return Err(ParseError::Syntax {
                        location,
                        message: "a pack expansion of parameter packs of different lengths",
                    });
                }
                count = Some(length);
            }
        }
        Ok(count.map(|count| (names, count)))
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
                state
                    .renames
                    .insert(name, state.expander.element_name(name, index)?);
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

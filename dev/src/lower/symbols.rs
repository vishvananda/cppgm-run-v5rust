// Translated from lower/symbol_lowering.cpp's procedural symbol and ABI construction. See NOTICE.
use super::Error;
use crate::{
    abi::{
        self,
        graph::{Modifier, Node, NodeId, Qualifiers},
        vocabulary::Builtin as B,
    },
    lowir::model::{Linkage, Metadata},
    sema::{
        Graph,
        lowering::{EntityId, Scalar, ScopeId, TypeId, TypeView},
    },
    support::stack::StackBudget,
    syntax::cursor::Cursor,
    types::FundamentalType as F,
};
use std::{collections::HashMap, fmt::Write};
pub(super) struct Symbols<'a, 'source> {
    graph: &'a Graph,
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
    callees: HashMap<EntityId, crate::lowir::model::Value>,
}
impl<'a, 'source> Symbols<'a, 'source> {
    pub(super) fn new(graph: &'a Graph, cursor: &'a Cursor<'source>, budget: StackBudget) -> Self {
        let marker = 0u8;
        Self {
            stack_base: std::ptr::from_ref(&marker).addr(),
            graph,
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
        }
    }
    pub(super) fn identifier(&self, name: crate::lex::identifiers::NameId) -> Result<&str, Error> {
        std::str::from_utf8(self.cursor.name(name))
            .map_err(|_| Error::Missing("local name spelling"))
    }
    fn check_stack(&self) -> Result<(), Error> {
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
            _ => {
                return Err(Error::Missing(
                    "reference, enum and function ABI type lowering",
                ));
            }
        };
        let mut id = self.abi.intern(node, &[]).map_err(Error::from)?;
        for (bit, modifier) in [
            (2, Modifier::Qualified(Qualifiers::VOLATILE)),
            (1, Modifier::Qualified(Qualifiers::CONST)),
        ] {
            if self.graph.lower_cv(ty) & bit != 0 {
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
        let name = self.name(entity)?;
        let value = builder.symbol(&name)?;
        self.callees.insert(entity, value);
        Ok(value)
    }
    pub(super) fn name(&mut self, entity: EntityId) -> Result<std::sync::Arc<str>, Error> {
        if let Some(name) = self.names.get(&entity) {
            return Ok(std::sync::Arc::clone(name));
        }
        if !self.ordinals.contains_key(&entity) {
            for (index, overload) in self.graph.lower_overloads(entity)?.enumerate() {
                self.ordinals.insert(overload, index + 1);
            }
        }
        let facts = self.graph.lower_function(entity)?;
        let own = std::str::from_utf8(self.cursor.name(facts.name))
            .map_err(|_| Error::Missing("non-ASCII symbol rendering"))?;
        let mut components = vec![own];
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
    pub(super) fn metadata(
        &mut self,
        program: &mut crate::lowir::model::Program,
        entity: EntityId,
        name: &str,
    ) -> Result<Metadata, Error> {
        use crate::lowir::vocabulary::{Binding, Role};
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
        if facts.c_linkage {
            metadata.linkage = Linkage::C;
        }
        if facts.c_linkage && !facts.internal {
            return Ok(metadata);
        }
        let parent = self.scope(facts.home)?;
        let terminal = self
            .abi
            .string(self.cursor.name(facts.name))
            .map_err(Error::from)?;
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

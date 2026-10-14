// Translated from sema/analyzer_builtins.cpp. See NOTICE.
//! Course builtin call forms, recognized by interned reserved identifiers.
use super::{
    Error,
    analyzer::Analyzer,
    expression_facts::{ExprFacts, ValueCategory as V},
    graph::{Binding, BindingKind, ScopeId},
    lookup::LookupResult,
    types::{Cv, Scalar, TypeKind},
};
use crate::{
    lex::{PostTokenKind, identifiers::NameId, vocabulary::Keyword},
    syntax::{ast, storage::List},
    types::FundamentalType as F,
};
#[derive(Clone, Copy)]
pub(crate) enum BuiltinFunction {
    Abort,
    Memcpy,
    Memmove,
    Memcmp,
    Memset,
    Strlen,
}
#[derive(Clone, Copy)]
pub(super) struct BuiltinNames {
    pub abort: NameId,
    pub constant_p: NameId,
    pub memcpy: NameId,
    pub memmove: NameId,
    pub memcmp: NameId,
    pub memset: NameId,
    pub strlen: NameId,
}
impl Analyzer<'_> {
    pub(super) fn ensure_builtin(
        &mut self,
        key: NameId,
    ) -> Result<Option<super::graph::BindingId>, Error> {
        let names = self.builtin_names;
        let builtin = if key == names.abort {
            BuiltinFunction::Abort
        } else if key == names.memcpy {
            BuiltinFunction::Memcpy
        } else if key == names.memmove {
            BuiltinFunction::Memmove
        } else if key == names.memcmp {
            BuiltinFunction::Memcmp
        } else if key == names.memset {
            BuiltinFunction::Memset
        } else if key == names.strlen {
            BuiltinFunction::Strlen
        } else {
            return Ok(None);
        };
        if let Some(binding) = self.graph.value_binding(self.graph.global, key.into()) {
            return Ok(self
                .graph
                .entity(self.graph.binding(binding).entity)
                .builtin
                .map(|_| binding));
        }
        let void = self.graph.types.scalar(Scalar::Void)?;
        let (result, parameters) = if matches!(builtin, BuiltinFunction::Abort) {
            (void, Vec::new())
        } else {
            let size = self.graph.types.scalar(Scalar::Basic(F::UnsignedLongInt))?;
            if matches!(builtin, BuiltinFunction::Strlen) {
                let character = self.graph.types.scalar(Scalar::Basic(F::Char))?;
                let character = self.graph.types.qualify(character, Cv::CONST)?;
                let text = self
                    .graph
                    .types
                    .intern(TypeKind::Pointer(character), Cv::default())?;
                (size, vec![text])
            } else {
                let pointer = self
                    .graph
                    .types
                    .intern(TypeKind::Pointer(void), Cv::default())?;
                let constant = self.graph.types.qualify(void, Cv::CONST)?;
                let source = self
                    .graph
                    .types
                    .intern(TypeKind::Pointer(constant), Cv::default())?;
                let integer = self.graph.types.scalar(Scalar::Basic(F::Int))?;
                match builtin {
                    BuiltinFunction::Memcpy | BuiltinFunction::Memmove => {
                        (pointer, vec![pointer, source, size])
                    }
                    BuiltinFunction::Memcmp => (integer, vec![source, source, size]),
                    BuiltinFunction::Memset => (pointer, vec![pointer, integer, size]),
                    BuiltinFunction::Abort | BuiltinFunction::Strlen => {
                        return Err(Error::Invariant("memory builtin signature"));
                    }
                }
            }
        };
        let ty = self.graph.types.intern(
            TypeKind::Function {
                result,
                parameters,
                variadic: false,
                cv: Cv::default(),
                reference: None,
            },
            Cv::default(),
        )?;
        let entity = self.graph.add_entity(Some(ty), Some(key), Keyword::Int)?;
        self.graph.entity_mut(entity).builtin = Some(builtin);
        self.graph.entity_mut(entity).exceptions =
            super::exceptions::ExceptionState::Value(super::exceptions::ExceptionSet::None);
        self.graph.entity_mut(entity).function = Some(super::graph::FunctionProperties {
            internal_linkage: false,
            language: super::linkage::Language::C,
            denotes: None,
            is_static: false,
            special: None,
            allocation: None,
        });
        Ok(Some(self.graph.add_binding(
            self.graph.global,
            None,
            Binding {
                kind: BindingKind::Function,
                name: Some(key),
                entity,
                ty: Some(ty),
                key: Keyword::Int,
            },
        )?))
    }
    pub(super) fn builtin_call(
        &mut self,
        id: ast::ExprId,
        name: ast::Name,
        arguments: List<ast::ExprId>,
        scope: ScopeId,
        found: &mut LookupResult,
    ) -> Result<Option<ExprFacts>, Error> {
        let node = self.tree.names.get(name);
        let components = self.tree.name_lists.get(node.components);
        if components.len() != 1 || found.binding.is_some() {
            return Ok(None);
        }
        let ast::NameKind::Identifier(key) = components[0].kind else {
            return Ok(None);
        };
        if let Some(binding) = self.ensure_builtin(key)? {
            found.binding = Some(binding);
            found.functions.push(binding);
            return Ok(None);
        }
        if key != self.builtin_names.constant_p {
            return Ok(None);
        }
        let arguments = self.tree.expr_lists.get(arguments);
        if arguments.len() != 1 {
            return Err(Error::at(
                self.tree.expressions.get(id).location,
                "__builtin_constant_p takes one argument",
            ));
        }
        let operand = arguments[0];
        self.unevaluated += 1;
        let result = self.typed_expression(operand, scope);
        self.unevaluated -= 1;
        let facts = result?;
        self.check_resolved_function(operand)?;
        let address = self.is_address_expression(operand);
        let mut constant = if address {
            self.address_value(operand, scope)?.is_some_and(|value| {
                matches!(
                    value.root,
                    super::constant_addresses::Root::Null
                        | super::constant_addresses::Root::Literal(_)
                )
            })
        } else {
            self.constant_value(operand, scope, false)?.is_some()
        };
        // Floating, null and string-literal address constants are supported by this query.
        let mut inner = operand;
        while let ast::ExprKind::Paren(expr) = self.tree.expressions.get(inner).kind {
            inner = expr;
        }
        constant |= matches!(
            self.tree.expressions.get(inner).kind,
            ast::ExprKind::Keyword(Keyword::Nullptr)
                | ast::ExprKind::Literal(ast::Literal {
                    kind: PostTokenKind::String { .. }
                        | PostTokenKind::Literal {
                            ty: F::Float | F::Double | F::LongDouble,
                            ..
                        },
                    ..
                })
        );
        if !constant
            && !address
            && matches!(
                self.graph.types.get(facts.ty).kind,
                TypeKind::Scalar(Scalar::Basic(F::Float | F::Double | F::LongDouble))
            )
        {
            return Err(Error::Missing(
                "floating and address constant interpretation",
            ));
        }
        if matches!(
            self.graph.types.get(facts.ty).kind,
            TypeKind::Scalar(Scalar::Void)
        ) {
            constant = false;
        }
        let ty = self.graph.types.scalar(Scalar::Basic(F::Int))?;
        let mut facts = self.new_expression_facts(ty, V::Prvalue)?;
        facts.folded = Some(u64::from(constant));
        Ok(Some(facts))
    }
}

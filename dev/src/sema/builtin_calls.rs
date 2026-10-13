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
pub(super) struct BuiltinNames {
    pub abort: NameId,
    pub constant_p: NameId,
}
impl Analyzer<'_> {
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
        if key == self.builtin_names.abort {
            let void = self.graph.types.scalar(Scalar::Void)?;
            let ty = self.graph.types.intern(
                TypeKind::Function {
                    result: void,
                    parameters: Vec::new(),
                    variadic: false,
                    cv: Cv::default(),
                    reference: None,
                },
                Cv::default(),
            )?;
            let entity = self.graph.add_entity(Some(ty), Some(key), Keyword::Int)?;
            let binding = self.graph.add_binding(
                self.graph.global,
                None,
                Binding {
                    kind: BindingKind::Function,
                    name: Some(key),
                    entity,
                    ty: Some(ty),
                    key: Keyword::Int,
                },
            )?;
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
        let mut constant = self.constant_value(operand, scope, false)?.is_some();
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
        if !constant && self.is_address_expression(operand) {
            constant = self.address_value(operand, scope)?.is_some_and(|value| {
                matches!(
                    value.root,
                    super::constant_addresses::Root::Null
                        | super::constant_addresses::Root::Literal(_)
                )
            });
        } else if !constant
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

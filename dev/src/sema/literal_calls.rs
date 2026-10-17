// Translated from sema/analyzer_literals.cpp. See NOTICE.
//! Literal calls retain the selected entity and decoded operand shape.
use super::{
    Error,
    analyzer::Analyzer,
    expression_facts::{ExprFacts, ValueCategory},
    function_names::{SpecialName, ValueKey},
    function_resolution::CallTarget,
    graph::{BindingId, BindingKind, FunctionAvailability, ScopeId},
    lookup::Filter,
    types::{Cv, Scalar, TypeId, TypeKind},
};
use crate::{
    lex::{
        LiteralForm, PostTokenKind,
        number::{NumberDialect, NumberLiteral, classify_number, user_integer_value},
    },
    syntax::ast,
    types::FundamentalType,
};

#[derive(Clone, Copy, Debug)]
pub(crate) enum LiteralCall {
    Character { source: TypeId, bits: u64 },
    String { element: TypeId, units: u32 },
    Integer { source: TypeId, bits: u64 },
    Floating { digits: usize },
    Raw { digits: usize },
    Template,
}

impl Analyzer<'_> {
    pub(super) fn literal_expression(
        &mut self,
        id: ast::ExprId,
        literal: ast::Literal,
        scope: ScopeId,
    ) -> Result<ExprFacts, Error> {
        if literal.suffix_name.is_some() {
            if let PostTokenKind::UserDefinedNumber { form, suffix_begin } = literal.kind {
                return self.numeric_literal_call(id, literal, scope, form, suffix_begin);
            }
            return self.literal_call(id, literal, scope);
        }
        let (ty, category) = match literal.kind {
            PostTokenKind::Literal { ty, .. }
            | PostTokenKind::Character {
                ty,
                suffix_begin: None,
                ..
            } => (
                self.graph.types.scalar(Scalar::Basic(ty))?,
                ValueCategory::Prvalue,
            ),
            PostTokenKind::String { ty, elements, .. } => {
                let element = self.graph.types.scalar(Scalar::Basic(ty))?;
                let element = self.graph.types.qualify(element, Cv::CONST)?;
                (
                    self.graph.types.intern(
                        TypeKind::Array {
                            element,
                            bound: u64::from(elements),
                            operand: None,
                        },
                        Cv::default(),
                    )?,
                    ValueCategory::Lvalue,
                )
            }
            _ => return Err(Error::Missing("user-defined semantic literals")),
        };
        let mut facts = self.new_expression_facts(ty, category)?;
        if let PostTokenKind::Literal { ty, data, .. } = literal.kind {
            facts.null_literal = super::integral::IntegralKind::of(Scalar::Basic(ty)).is_some()
                && data.as_slice().iter().all(|&b| b == 0);
        }
        Ok(facts)
    }
    pub(super) fn literal_call(
        &mut self,
        id: ast::ExprId,
        literal: ast::Literal,
        scope: ScopeId,
    ) -> Result<ExprFacts, Error> {
        let location = self.tree.expressions.get(id).location;
        let suffix = literal
            .suffix_name
            .ok_or(Error::Invariant("literal call without suffix"))?;
        let (shape, first, second, missing) = match literal.kind {
            PostTokenKind::Character { ty, data, .. } => {
                let source = self.graph.types.scalar(Scalar::Basic(ty))?;
                let mut bytes = [0; 8];
                let data = data.as_slice();
                bytes[..data.len()].copy_from_slice(data);
                (
                    LiteralCall::Character {
                        source,
                        bits: u64::from_le_bytes(bytes),
                    },
                    ExprFacts::new(source, ValueCategory::Prvalue),
                    None,
                    "no literal operator for the user-defined character literal",
                )
            }
            PostTokenKind::String { ty, elements, .. } => {
                let element = self.graph.types.scalar(Scalar::Basic(ty))?;
                let unit = self.graph.types.qualify(element, Cv::CONST)?;
                let array = self.graph.types.intern(
                    TypeKind::Array {
                        element: unit,
                        bound: u64::from(elements),
                        operand: None,
                    },
                    Cv::default(),
                )?;
                let length = self
                    .graph
                    .types
                    .scalar(Scalar::Basic(FundamentalType::UnsignedLongInt))?;
                (
                    LiteralCall::String {
                        element,
                        units: elements,
                    },
                    ExprFacts::new(array, ValueCategory::Lvalue),
                    Some(ExprFacts::new(length, ValueCategory::Prvalue)),
                    "no literal operator for the user-defined string literal",
                )
            }
            _ => return Err(Error::Missing("numeric literal operator calls")),
        };
        self.lookup_location = location;
        let found = self.unqualified_result(
            scope,
            ValueKey::Special(SpecialName::Literal(suffix)),
            Filter::Ordinary,
        )?;
        if !found
            .binding
            .is_some_and(|binding| self.graph.binding(binding).kind == BindingKind::Function)
        {
            return Err(Error::at(location, missing));
        }
        let operands = [first, second.unwrap_or(first)];
        let binding = self.resolve_value_call(
            location,
            found.functions.as_slice(),
            &operands[..=usize::from(second.is_some())],
        )?;
        self.finish_literal_call(id, binding, shape)
    }

    fn numeric_literal_call(
        &mut self,
        id: ast::ExprId,
        literal: ast::Literal,
        scope: ScopeId,
        form: LiteralForm,
        digits: usize,
    ) -> Result<ExprFacts, Error> {
        let location = self.tree.expressions.get(id).location;
        let suffix = literal
            .suffix_name
            .ok_or(Error::Invariant("numeric literal without suffix"))?;
        self.lookup_location = location;
        let found = self.unqualified_result(
            scope,
            ValueKey::Special(SpecialName::Literal(suffix)),
            Filter::Ordinary,
        )?;
        if !found
            .binding
            .is_some_and(|binding| self.graph.binding(binding).kind == BindingKind::Function)
        {
            return Err(Error::at(
                location,
                "no literal operator for the user-defined literal",
            ));
        }
        let cooked = self
            .graph
            .types
            .scalar(Scalar::Basic(if form == LiteralForm::Integer {
                FundamentalType::UnsignedLongLongInt
            } else {
                FundamentalType::LongDouble
            }))?;
        let character = self
            .graph
            .types
            .scalar(Scalar::Basic(FundamentalType::Char))?;
        let character = self.graph.types.qualify(character, Cv::CONST)?;
        let raw = self.graph.types.pointer(character)?;
        // These are exact signature filters, not overload conversion ranking.
        // Every cooked candidate precedes every raw one, irrespective of order.
        let mut selected = None;
        for (parameter, is_raw) in [(cooked, false), (raw, true)] {
            for &binding in found.functions.as_slice() {
                let candidate = self.graph.entity(self.graph.binding(binding).entity);
                if candidate.template_head.is_some() {
                    continue;
                }
                let ty = candidate
                    .ty
                    .ok_or(Error::Invariant("literal operator without type"))?;
                let ty = self.graph.types.canonical_function(ty)?;
                if let TypeKind::Function {
                    parameters,
                    variadic: false,
                    ..
                } = &self.graph.types.get(ty).kind
                    && parameters.len() == 1
                    && self.bare_type(parameters[0])? == parameter
                {
                    selected = Some((binding, is_raw));
                    break;
                }
            }
            if selected.is_some() {
                break;
            }
        }
        let Some((binding, is_raw)) = selected else {
            if let Some(binding) =
                self.numeric_literal_template(found.functions.as_slice(), literal, digits)?
            {
                return self.finish_literal_call(id, binding, LiteralCall::Template);
            }
            return Err(Error::at(
                location,
                "no literal operator the user-defined literal can call",
            ));
        };
        let shape = if is_raw {
            LiteralCall::Raw { digits }
        } else if form == LiteralForm::Floating {
            LiteralCall::Floating { digits }
        } else {
            let spelling = self.literal_bytes.get(literal.spelling);
            let digits = spelling
                .get(..digits)
                .ok_or(Error::Invariant("numeric literal digits outside spelling"))?;
            let value = user_integer_value(digits).ok_or(Error::at(
                location,
                "a user-defined integer literal beyond unsigned long long",
            ))?;
            let source = if let NumberLiteral::Integer { ty, .. } =
                classify_number(digits, NumberDialect::Cxx11)
            {
                self.graph.types.scalar(Scalar::Basic(ty))?
            } else {
                cooked
            };
            LiteralCall::Integer {
                source,
                bits: value,
            }
        };
        self.finish_literal_call(id, binding, shape)
    }

    fn numeric_literal_template(
        &mut self,
        candidates: &[BindingId],
        literal: ast::Literal,
        digits: usize,
    ) -> Result<Option<BindingId>, Error> {
        let character = self
            .graph
            .types
            .scalar(Scalar::Basic(FundamentalType::Char))?;
        let mut pack = None;
        for &primary in candidates {
            let Some(head) = self
                .graph
                .entity(self.graph.binding(primary).entity)
                .template_head
            else {
                continue;
            };
            let parameters = self.graph.templates.parameters(head);
            if parameters.len() != 1
                || !self
                    .graph
                    .entity(parameters[0])
                    .template_parameter
                    .is_some_and(|info| {
                        info.is_pack()
                            && info.value_type.is_some_and(|ty| {
                                matches!(
                                    self.graph
                                        .types
                                        .get(self.graph.types.canonical_view(ty))
                                        .kind,
                                    TypeKind::Scalar(Scalar::Basic(FundamentalType::Char))
                                )
                            })
                    })
            {
                continue;
            }
            // Retain the original spelling, including base prefixes, zeroes,
            // exponent signs and separators. No numeric value is demanded.
            let argument = if let Some(pack) = pack {
                pack
            } else {
                let spelling = self.literal_bytes.get(literal.spelling);
                let spelling = spelling
                    .get(..digits)
                    .ok_or(Error::Invariant("numeric literal digits outside spelling"))?;
                let mut characters = Vec::with_capacity(spelling.len());
                let kind = super::integral::IntegralKind {
                    bits: 8,
                    signed: true,
                };
                for &byte in spelling {
                    characters.push(self.graph.types.intern(
                        TypeKind::Value {
                            ty: character,
                            bits: kind.convert(u64::from(byte)),
                        },
                        Cv::default(),
                    )?);
                }
                let argument = self
                    .graph
                    .types
                    .intern(TypeKind::Pack(characters), Cv::default())?;
                pack = Some(argument);
                argument
            };
            if let Some(binding) = self.complete_signature(primary, &mut [Some(argument)])? {
                return Ok(Some(binding));
            }
        }
        Ok(None)
    }

    fn finish_literal_call(
        &mut self,
        id: ast::ExprId,
        binding: BindingId,
        shape: LiteralCall,
    ) -> Result<ExprFacts, Error> {
        let location = self.tree.expressions.get(id).location;
        let entity = self.graph.binding(binding).entity;
        if self.graph.entity(entity).availability == FunctionAvailability::Deleted {
            return Err(Error::at(location, "use of a deleted function"));
        }
        self.demand_specialization_body(binding);
        self.demand_function(binding, true);
        let ty = self
            .graph
            .entity(entity)
            .ty
            .ok_or(Error::Invariant("literal operator without type"))?;
        let ty = self.graph.types.canonical_function(ty)?;
        let mut facts = self.call_result(ty)?;
        facts.call = Some(CallTarget::Direct(self.graph.function_origin(entity)?));
        facts.literal_call = Some(shape);
        self.graph.record_call_binding(id, binding);
        Ok(facts)
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

    fn return_literals(
        tree: &ast::Tree,
        declarations: crate::syntax::storage::List<ast::DeclId>,
    ) -> Vec<ast::ExprId> {
        let last = *tree.decl_lists.get(declarations).last().unwrap();
        let ast::DeclKind::Function { body, .. } = tree.declarations.get(last).kind else {
            panic!("main function");
        };
        let ast::Body::Parsed(body) = tree.bodies.get(body) else {
            panic!("function body");
        };
        let ast::StmtKind::Compound(statements) = tree.statements.get(body.compound).kind else {
            panic!("compound body");
        };
        let ast::StmtKind::Return(Some(expression)) =
            tree.statements.get(tree.stmt_lists.get(statements)[0]).kind
        else {
            panic!("return expression");
        };
        let mut work = vec![expression];
        let mut literals = Vec::new();
        while let Some(id) = work.pop() {
            match tree.expressions.get(id).kind {
                ast::ExprKind::Binary { left, right, .. } => work.extend([right, left]),
                ast::ExprKind::Literal(_) => literals.push(id),
                _ => panic!("literal sum"),
            }
        }
        literals
    }

    #[test]
    fn numeric_selection_retains_operand_types_without_synthetic_call_syntax() {
        let source = SourceBuffer::new(
            b"int operator\"\"_x(const char*){return 0;}\
              int operator\"\"_x(unsigned long long){return 1;}\
              int operator\"\"_x(long double){return 2;}\
              int main(){return 3_x+9223372036854775808_x+1.5_x;}"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"literal-selection").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let expressions = tree.expressions.mark();
        let literals = return_literals(tree, declarations);
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        analyzer
            .declarations(declarations, analyzer.graph.global)
            .unwrap();
        assert_eq!(analyzer.tree.expressions.mark(), expressions);
        let mut integers = Vec::new();
        let mut floating = 0;
        for id in literals {
            let Ok(facts) = analyzer.graph.expression_facts.get(id) else {
                continue;
            };
            let expected = match facts.literal_call {
                Some(LiteralCall::Integer { source, bits }) => {
                    integers.push((analyzer.graph.types.get(source).kind.clone(), bits));
                    FundamentalType::UnsignedLongLongInt
                }
                Some(LiteralCall::Floating { .. }) => {
                    floating += 1;
                    FundamentalType::LongDouble
                }
                None => continue,
                _ => panic!("raw candidate must not displace a cooked one"),
            };
            let CallTarget::Direct(binding) = facts.call.unwrap() else {
                panic!("direct literal call");
            };
            let ty = analyzer.graph.binding(binding).ty.unwrap();
            let parameter = analyzer.graph.types.function_parameter(ty, 0).unwrap();
            assert_eq!(
                analyzer.graph.types.get(parameter).kind,
                TypeKind::Scalar(Scalar::Basic(expected))
            );
            assert!(!facts.null_literal);
        }
        assert_eq!(floating, 1);
        assert_eq!(
            integers,
            [
                (TypeKind::Scalar(Scalar::Basic(FundamentalType::Int)), 3),
                (
                    TypeKind::Scalar(Scalar::Basic(FundamentalType::UnsignedLongLongInt)),
                    1u64 << 63
                ),
            ]
        );
    }

    #[test]
    fn raw_selection_does_not_demand_numeric_range_or_value_conversions() {
        let source = SourceBuffer::new(
            b"int operator\"\"_x(const char*){return 0;}\
              int main(){return 18446744073709551616_x+1e999999999_x;}"
                .to_vec(),
        )
        .unwrap();
        let mut unit = SyntaxUnit::new(Preprocessor::with_name(&source, b"raw-range").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let literals = return_literals(tree, declarations);
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        analyzer
            .declarations(declarations, analyzer.graph.global)
            .unwrap();
        let mut lengths = Vec::new();
        for id in literals {
            if let Ok(facts) = analyzer.graph.expression_facts.get(id)
                && let Some(LiteralCall::Raw { digits }) = facts.literal_call
            {
                lengths.push(digits);
                assert!(!facts.null_literal);
            }
        }
        assert_eq!(lengths, [20, 11]);
    }

    #[test]
    fn numeric_templates_share_canonical_character_packs_without_argument_syntax() {
        let source = SourceBuffer::new(
            b"template<char...C>int operator\"\"_x();\
              template<>int operator\"\"_x<'0','x','0','0'>();\
              int main(){return 0x00_x+0x00_x+0000_x+1.5e+02_x;}"
                .to_vec(),
        )
        .unwrap();
        let mut unit = SyntaxUnit::new(Preprocessor::with_name(&source, b"literal-packs").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let expressions = tree.expressions.mark();
        let literals = return_literals(tree, declarations);
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        analyzer
            .declarations(declarations, analyzer.graph.global)
            .unwrap();
        assert_eq!(analyzer.tree.expressions.mark(), expressions);
        let mut entities = Vec::new();
        let mut spellings = Vec::new();
        for id in literals {
            let facts = analyzer.graph.expression_facts.get(id).unwrap();
            assert!(matches!(facts.literal_call, Some(LiteralCall::Template)));
            assert!(!facts.null_literal);
            let CallTarget::Direct(binding) = facts.call.unwrap() else {
                panic!("direct template literal call");
            };
            let entity = analyzer.graph.binding(binding).entity;
            entities.push(entity);
            let arguments = &analyzer.graph.function_templates.arguments[&entity];
            assert_eq!(arguments.len(), 1);
            let mut spelling = Vec::new();
            for &argument in analyzer.graph.types.pack_elements(arguments[0]).unwrap() {
                let TypeKind::Value { ty, bits } = analyzer.graph.types.get(argument).kind else {
                    panic!("canonical character argument");
                };
                assert_eq!(
                    analyzer.graph.types.get(ty).kind,
                    TypeKind::Scalar(Scalar::Basic(FundamentalType::Char))
                );
                spelling.push(u8::try_from(bits).unwrap());
            }
            spellings.push(spelling);
        }
        assert_eq!(entities[0], entities[1]);
        assert_ne!(entities[0], entities[2]);
        assert!(
            analyzer
                .graph
                .function_templates
                .specializations
                .contains(entities[0])
        );
        assert!(
            !analyzer
                .graph
                .function_templates
                .specializations
                .contains(entities[2])
        );
        assert_eq!(
            spellings,
            [
                b"0x00".to_vec(),
                b"0x00".to_vec(),
                b"0000".to_vec(),
                b"1.5e+02".to_vec()
            ]
        );
        let mut output = Vec::new();
        let graph = analyzer.graph;
        graph
            .print_semantics(tree, cursor, declarations, &mut output)
            .unwrap();
        assert!(
            String::from_utf8(output)
                .unwrap()
                .contains("function-declaration operator\"\"_x function of () returning int")
        );
    }

    #[test]
    fn literal_reference_views_keep_referent_types_beside_ordinary_calls() {
        let source = SourceBuffer::new(
            b"int v;int& f(){return v;}\
              int&operator\"\"_x(unsigned long long){return v;}\
              int main(){3_x=4;3_x;f();return 3_x;}"
                .to_vec(),
        )
        .unwrap();
        let mut unit = SyntaxUnit::new(Preprocessor::with_name(&source, b"literal-view").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let graph = super::super::analyze_unit(
            tree,
            cursor,
            declarations,
            StackBudget::default(),
            true,
            false,
        )
        .unwrap();
        let mut output = Vec::new();
        graph
            .print_semantics(tree, cursor, declarations, &mut output)
            .unwrap();
        let output = String::from_utf8(output).unwrap();
        assert_eq!(output.matches("literal lvalue int 3_x").count(), 3);
        assert!(output.contains("call-expression lvalue lvalue-reference to int"));
    }
}

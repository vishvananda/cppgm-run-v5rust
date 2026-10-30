// Translated from analyzer_builtins.cpp and constant_expressions.cpp. See NOTICE.
//! Source queries retain their kind; evaluation owns the outermost use site.
use super::{
    Error,
    analyzer::Analyzer,
    constant_addresses::{Address, Root, TemporaryValue},
    constants::Constant,
    constexpr_calls::Returned,
    expression_facts::{ExprFacts, ValueCategory},
    types::{Cv, Scalar, TypeKind},
};
use crate::{
    lex::{PostTokenKind, identifiers::NameId},
    support::source::SourceOffset,
    syntax::{ast, cursor::Cursor, storage::List},
    types::FundamentalType as F,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceLocation {
    File,
    Line,
    Function,
    Column,
}

pub(super) fn names(cursor: &mut Cursor<'_>) -> Result<Vec<(NameId, SourceLocation)>, Error> {
    [
        ("__builtin_FILE", SourceLocation::File),
        ("__builtin_LINE", SourceLocation::Line),
        ("__builtin_FUNCTION", SourceLocation::Function),
        ("__builtin_COLUMN", SourceLocation::Column),
    ]
    .into_iter()
    .map(|(name, kind)| {
        Ok((
            cursor
                .intern_name(name.as_bytes())
                .map_err(Error::identifier_interning)?,
            kind,
        ))
    })
    .collect()
}

impl Analyzer<'_> {
    pub(super) fn source_location_builtin(
        &mut self,
        id: ast::ExprId,
        arguments: List<ast::ExprId>,
        kind: SourceLocation,
    ) -> Result<ExprFacts, Error> {
        if !arguments.is_empty() {
            return Err(Error::at(
                self.tree.expressions.get(id).location,
                "a source location builtin takes no arguments",
            ));
        }
        let ty = if matches!(kind, SourceLocation::File | SourceLocation::Function) {
            let character = self.graph.types.scalar(Scalar::Basic(F::Char))?;
            let character = self.graph.types.qualify(character, Cv::CONST)?;
            self.graph
                .types
                .intern(TypeKind::Pointer(character), Cv::default())?
        } else {
            self.graph.types.scalar(Scalar::Basic(F::Int))?
        };
        self.graph.expression_facts.set_source_location(id, kind);
        self.new_expression_facts(ty, ValueCategory::Prvalue)
    }

    pub(super) fn constant_source_location(
        &mut self,
        id: ast::ExprId,
        kind: SourceLocation,
    ) -> Result<Returned, Error> {
        let location = self
            .source_site
            .unwrap_or(self.tree.expressions.get(id).location);
        let value = match kind {
            SourceLocation::Line | SourceLocation::Column => {
                let bits = if kind == SourceLocation::Line {
                    let line = self
                        .positions
                        .source_line(location)
                        .map_err(|_| Error::Invariant("invalid source position"))?;
                    u64::try_from(line).map_err(|_| Error::Invariant("negative source line"))?
                } else {
                    0
                };
                TemporaryValue::Integer(Constant {
                    state: crate::sema::constant_addresses::ScalarState::Value,
                    bits,
                    high: 0,
                    ty: self.graph.expression_facts.get(id)?.ty,
                })
            }
            SourceLocation::File | SourceLocation::Function => {
                let text = if kind == SourceLocation::File {
                    self.positions.source_file(location).to_vec()
                } else {
                    use super::predefined_names::SourceFunction;
                    let function = if let Some(frame) = self.constexpr_frames.current_call() {
                        frame.source_function
                    } else {
                        self.body_context
                            .as_ref()
                            .and_then(|context| context.function.map(SourceFunction::Function))
                    };
                    function
                        .map(|function| {
                            super::predefined_names::function_name(
                                &self.graph,
                                self.identifiers,
                                function,
                                self.stack_budget,
                                self.stack_base,
                            )
                        })
                        .transpose()?
                        .unwrap_or_default()
                };
                let literal = self.source_text_literal(&text)?;
                let TypeKind::Array { element, .. } = self
                    .graph
                    .types
                    .get(self.graph.expression_facts.get(literal)?.ty)
                    .kind
                else {
                    return Err(Error::Invariant("source text without array"));
                };
                TemporaryValue::Pointer(Address::new(Root::Literal(literal), element))
            }
        };
        Ok(Returned::Value(value))
    }

    pub(super) fn source_text_literal(&mut self, text: &[u8]) -> Result<ast::ExprId, Error> {
        let key = self
            .identifiers
            .intern(text)
            .map_err(|_| Error::Limit("source text identifier limit"))?;
        if let Some(&id) = self.graph.analysis.text_literals.get(&key) {
            return Ok(id);
        }
        let mut spelling = vec![b'"'];
        for &byte in text {
            match byte {
                b'"' | b'\\' => {
                    spelling.push(b'\\');
                    spelling.push(byte);
                }
                0..=31 | 127 => spelling.extend_from_slice(&[
                    b'\\',
                    b'0' + (byte >> 6),
                    b'0' + ((byte >> 3) & 7),
                    b'0' + (byte & 7),
                ]),
                _ => spelling.push(byte),
            }
        }
        spelling.push(b'"');
        let mut data = text.to_vec();
        data.push(0);
        let elements = u32::try_from(data.len()).map_err(|_| Error::Limit("source text length"))?;
        let initializer_bound = text.iter().fold(1u64, |count, &byte| {
            count.wrapping_add(u64::from(byte & 0xc0 != 0x80))
        });
        let spelling = self
            .literal_bytes
            .add(&spelling)
            .map_err(|_| Error::Limit("source text spelling limit"))?;
        let data = self
            .literal_bytes
            .add(&data)
            .map_err(|_| Error::Limit("source text data limit"))?;
        let literal = self
            .literal_bytes
            .literal(ast::LiteralData {
                kind: PostTokenKind::String {
                    ty: F::Char,
                    elements,
                    initializer_bound,
                    suffix: None,
                },
                spelling,
                data,
                token_form: false,
                suffix_name: None,
            })
            .map_err(|_| Error::Limit("source literal limit"))?;
        let id = self
            .tree
            .expressions
            .add(ast::Expression {
                location: SourceOffset(0),
                kind: ast::ExprKind::Literal(literal),
            })
            .map_err(|_| Error::Limit("source text expression limit"))?;
        self.typed_expression(id, self.graph.global)?;
        self.graph.analysis.text_literals.insert(key, id);
        Ok(id)
    }

    pub(super) fn with_source_site<T>(
        &mut self,
        site: Option<SourceOffset>,
        action: impl FnOnce(&mut Self) -> Result<T, Error>,
    ) -> Result<T, Error> {
        let saved = self.source_site;
        if saved.is_none() {
            self.source_site = site.filter(|site| site.0 != 0);
        }
        let result = action(self);
        self.source_site = saved;
        result
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
    fn outer_site_survives_nested_queries_and_restores_after_errors() {
        let source = SourceBuffer::new(b"void f();".to_vec()).unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"source-context").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, _) = unit.semantic_inputs().unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        let result: Result<(), Error> =
            analyzer.with_source_site(Some(SourceOffset(7)), |analyzer| {
                analyzer.with_source_site(Some(SourceOffset(19)), |analyzer| {
                    assert_eq!(analyzer.source_site, Some(SourceOffset(7)));
                    Err(Error::Invariant("probe failure"))
                })
            });
        assert!(matches!(result, Err(Error::Invariant("probe failure"))));
        assert_eq!(analyzer.source_site, None);
        analyzer
            .with_source_site(Some(SourceOffset(0)), |analyzer| {
                assert_eq!(analyzer.source_site, None);
                Ok(())
            })
            .unwrap();
    }
    #[test]
    fn text_objects_are_interned_without_parsing_and_keep_byte_identity() {
        let source = SourceBuffer::new(Vec::new()).unwrap();
        let mut unit = SyntaxUnit::new(Preprocessor::with_name(&source, b"source-text").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, _) = unit.semantic_inputs().unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let text = b"\"\\\n\t\x7f\xff";
        let first = analyzer.source_text_literal(text).unwrap();
        let mark = analyzer.tree.expressions.mark();
        assert_eq!(first, analyzer.source_text_literal(text).unwrap());
        assert_eq!(mark, analyzer.tree.expressions.mark());
        let ast::ExprKind::Literal(literal) = analyzer.tree.expressions.get(first).kind else {
            panic!("source text without literal");
        };
        assert_eq!(
            analyzer
                .literal_bytes
                .get(literal.data(analyzer.literal_bytes)),
            b"\"\\\n\t\x7f\xff\0"
        );
        assert!(analyzer.graph.root_demands.as_slice().is_empty());
    }
}

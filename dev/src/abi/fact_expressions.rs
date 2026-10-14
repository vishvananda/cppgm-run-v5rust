//! Dependent expression facts from `abi_fact_definitions.cpp`. See NOTICE.
use super::{
    Error,
    facts::{CaseReader, DefinitionKind},
    graph::{Node, NodeId},
    vocabulary::{Builtin, ExpressionShape, Operation},
};

impl CaseReader<'_, '_> {
    pub(super) fn expression_definition(
        &mut self,
        line: usize,
        words: &[&[u8]],
    ) -> Result<NodeId, Error> {
        let operands = &words[1..];
        let node = match words[0] {
            b"template-param" if operands.len() == 1 => Node::TemplateParam {
                index: self.index_value(line, operands[0])?,
                substitutable: false,
            },
            b"function-param" if operands.len() == 1 => {
                Node::FunctionParam(self.index_value(line, operands[0])?)
            }
            b"literal" if operands.len() == 1 => {
                let ty = self.graph.intern(Node::Builtin(Builtin::Int), &[])?;
                // Unlike typed argument values, expression literals carry the
                // reader's full target long-long bit pattern without int-range checking.
                let value = self.integer_value(line, operands[0])?;
                Node::Literal {
                    ty,
                    bits: u64::from_ne_bytes(value.to_ne_bytes()),
                }
            }
            b"sizeof-type" if operands.len() == 1 => {
                Node::SizeofType(self.compact_type(line, operands[0])?)
            }
            b"pack" if operands.len() == 1 => {
                return self.fixed_operation(Operation::PackExpansion, operands);
            }
            b"entity-reference" if operands.len() == 1 => Node::EntityReference(self.reference(
                operands[0],
                DefinitionKind::Entity,
                "entity",
            )?),
            b"member" if operands.len() >= 3 => {
                let owner = self.compact_type(line, operands[0])?;
                let close_qualifier = self.flag(line, operands[1])?;
                let member = self.graph.string(operands[2])?;
                let args = self.argument_list(&operands[3..])?;
                return self.graph.intern(
                    Node::ScopedMember {
                        owner,
                        member,
                        close_qualifier,
                    },
                    &args,
                );
            }
            b"source-name" if operands.len() == 1 => {
                Node::UnresolvedName(self.graph.string(operands[0])?)
            }
            b"template-id" if !operands.is_empty() => {
                let args = self.argument_list(&operands[1..])?;
                let name = self.graph.string(operands[0])?;
                return self.graph.intern(Node::UnresolvedName(name), &args);
            }
            b"type-trait" if operands.len() >= 2 => {
                let mut types = Vec::with_capacity(operands.len() - 1);
                for word in &operands[1..] {
                    types.push(self.compact_type(line, word)?);
                }
                let name = self.graph.string(operands[0])?;
                return self.graph.intern(Node::VendorExpression(name), &types);
            }
            b"cast" if operands.len() == 3 => {
                let operation =
                    self.expression_operation(line, operands[0], ExpressionShape::Cast, "cast")?;
                let ty = self.compact_type(line, operands[1])?;
                let operand = self.expression_reference(operands[2])?;
                Node::Cast {
                    operation,
                    ty,
                    operand,
                }
            }
            b"object-member" if operands.len() >= 3 => {
                let operation = self.expression_operation(
                    line,
                    operands[0],
                    ExpressionShape::MemberAccess,
                    "member access",
                )?;
                let object = self.expression_reference(operands[1])?;
                let args = self.argument_list(&operands[3..])?;
                let member = self.graph.string(operands[2])?;
                return self.graph.intern(
                    Node::MemberAccess {
                        operation,
                        object,
                        member,
                    },
                    &args,
                );
            }
            b"unary" if operands.len() == 2 => {
                return self.operation_definition(line, operands, ExpressionShape::Unary);
            }
            b"binary" if operands.len() == 3 => {
                return self.operation_definition(line, operands, ExpressionShape::Binary);
            }
            b"conditional" if operands.len() == 3 => {
                return self.fixed_operation(Operation::Conditional, operands);
            }
            b"call" if !operands.is_empty() => {
                let items = self.expression_list(operands)?;
                return self.graph.intern(Node::Call, &items);
            }
            _ => {
                return Err(self.word_error(
                    line,
                    "unknown or malformed dependent expression form '",
                    words[0],
                    "'",
                ));
            }
        };
        self.graph.intern(node, &[])
    }
    fn expression_reference(&mut self, word: &[u8]) -> Result<NodeId, Error> {
        self.reference(word, DefinitionKind::Expression, "expression")
    }
    fn expression_list(&mut self, words: &[&[u8]]) -> Result<Vec<NodeId>, Error> {
        words
            .iter()
            .map(|word| self.expression_reference(word))
            .collect()
    }
    fn expression_operation(
        &self,
        line: usize,
        word: &[u8],
        shape: ExpressionShape,
        what: &str,
    ) -> Result<Operation, Error> {
        Operation::from_word(word, shape)
            .ok_or_else(|| self.word_error(line, &format!("unknown {what} operation '"), word, "'"))
    }
    // These grammar productions have at most three operands. Keep their
    // temporary edges on the stack; the graph owns the published slice.
    fn fixed_operation(&mut self, operation: Operation, words: &[&[u8]]) -> Result<NodeId, Error> {
        let Some((first, rest)) = words.split_first().filter(|_| words.len() <= 3) else {
            return Err(Error::StorageLimit);
        };
        let mut items = [self.expression_reference(first)?; 3];
        for (index, word) in rest.iter().enumerate() {
            items[index + 1] = self.expression_reference(word)?;
        }
        self.graph
            .intern(Node::Operation(operation), &items[..words.len()])
    }
    fn operation_definition(
        &mut self,
        line: usize,
        operands: &[&[u8]],
        shape: ExpressionShape,
    ) -> Result<NodeId, Error> {
        let operation = self.expression_operation(line, operands[0], shape, "expression")?;
        self.fixed_operation(operation, &operands[1..])
    }
}

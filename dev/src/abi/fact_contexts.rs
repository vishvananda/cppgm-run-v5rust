//! Local scopes and closure facts at the standalone input boundary. See NOTICE.
use super::{
    Error,
    facts::{CaseReader, DefinitionKind},
    graph::{Node, NodeId},
};

impl CaseReader<'_, '_> {
    pub(super) fn context_definition(
        &mut self,
        line: usize,
        words: &[&[u8]],
    ) -> Result<NodeId, Error> {
        match words[0] {
            b"raw" => {
                if words.len() != 2 {
                    return Err(self.error(line, "raw context takes one fragment"));
                }
                let fragment = self.graph.string(words[1])?;
                self.graph.intern(Node::RawFunctionScope(fragment), &[])
            }
            b"function" if words.len() >= 2 => {
                if words[1] == b"encoding" {
                    return Err(self.error(line, "a function entity needs a path"));
                }
                let function = self.function_target(line, &words[1..])?;
                let encoding = self.finish_function(line, function)?;
                self.graph.intern(
                    Node::FunctionScope {
                        encoding,
                        default_argument: 0,
                    },
                    &[],
                )
            }
            _ => Err(self.word_error(line, "unknown context form '", words[0], "'")),
        }
    }

    pub(super) fn local_entity(
        &mut self,
        line: usize,
        context: &[u8],
        name: &[u8],
        occurrence: &[u8],
    ) -> Result<NodeId, Error> {
        // C++ interns the source name before resolving the context binder.
        let terminal = self.graph.string(name)?;
        let parent = self.reference(context, DefinitionKind::Context, "context")?;
        let occurrence = self.index_value(line, occurrence)?;
        self.graph.intern(
            Node::Name {
                parent: Some(parent),
                terminal,
                internal: false,
                hidden_tags: false,
                data_member: false,
                occurrence,
            },
            &[],
        )
    }

    pub(super) fn closure_entity(
        &mut self,
        line: usize,
        context: &[u8],
        number: &[u8],
        signature: &[&[u8]],
    ) -> Result<NodeId, Error> {
        let parent = self.reference(context, DefinitionKind::Context, "context")?;
        let number = if number == b"-" {
            0
        } else {
            self.index_value(line, number)?.wrapping_add(1)
        };
        let mut types = Vec::new();
        for word in signature {
            types.push(self.compact_type(line, word)?);
        }
        self.graph.drop_void(&mut types);
        let terminal = self.graph.intern(
            Node::ClosureName {
                number,
                variadic: false,
            },
            &types,
        )?;
        self.graph.name(Some(parent), terminal, &[])
    }
}

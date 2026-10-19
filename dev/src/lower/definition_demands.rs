// Translated from lower/symbol_lowering.cpp::variable_symbol and
// program_lowering.cpp::demand_entry/emit_demanded. See NOTICE.
//! First-reference variable work reserves a position in the emission queue.
//! Its initializer's requests fill that position after the caller's body has
//! released its builder. Appending children never shifts unrelated requests.
use super::{Error, construction::Entry, symbols::Symbols};
use crate::lowir::model::{Program, SymbolId};
use crate::sema::lowering::EntityId;

// C++ note_argument_demand's namespace-context list grows while source
// definitions are emitted. Its one reverse traversal precedes emit_demanded;
// initializers reached by that later work do not retroactively join the roots.
pub(super) struct RootArguments {
    functions: Vec<EntityId>,
    seen: Option<std::collections::HashSet<EntityId>>,
}
impl RootArguments {
    pub(super) fn new(functions: impl Iterator<Item = EntityId>) -> Self {
        let mut roots = Self {
            functions: Vec::new(),
            seen: Some(std::collections::HashSet::new()),
        };
        for function in functions {
            roots.note(function);
        }
        roots
    }
    pub(super) fn note(&mut self, function: EntityId) {
        if self.seen.as_mut().is_some_and(|seen| seen.insert(function)) {
            self.functions.push(function);
        }
    }
    pub(super) fn finish(&mut self) -> std::iter::Rev<std::vec::IntoIter<EntityId>> {
        self.seen = None;
        std::mem::take(&mut self.functions).into_iter().rev()
    }
}

// Finalize the ABI entry before appending functions designated by this
// definition's template arguments, matching emit_definition's demand order.
pub(super) fn finish_function_definition(
    symbols: &mut Symbols<'_, '_>,
    graph: &crate::sema::Graph,
    program: &mut Program,
    entry: Entry,
    symbol: SymbolId,
) -> Result<(), Error> {
    symbols.finish_entry(program, entry, symbol)?;
    symbols.key_function_table(program, entry)?;
    if let Some(entity) = entry.entity(graph) {
        for function in graph.lower_argument_functions(Some(entity)) {
            symbols.demand(function);
        }
    }
    Ok(())
}

#[derive(Clone, Copy)]
pub(super) struct RequestId(usize);
#[derive(Default)]
struct Chain {
    first: Option<RequestId>,
    last: Option<RequestId>,
}
struct Request<T> {
    entry: Option<T>,
    next: Option<RequestId>,
    children: Chain,
}
#[derive(Default)]
struct Cursor {
    parent: Option<RequestId>,
    last: Option<RequestId>,
}
pub(super) struct Demands<T = Entry> {
    requests: Vec<Request<T>>,
    roots: Chain,
    owner: Option<RequestId>,
    cursors: Vec<Cursor>,
}
impl<T> Default for Demands<T> {
    fn default() -> Self {
        Self {
            requests: Vec::new(),
            roots: Chain::default(),
            owner: None,
            cursors: Vec::new(),
        }
    }
}
impl<T: Copy> Demands<T> {
    fn append(&mut self, entry: Option<T>) -> RequestId {
        let id = RequestId(self.requests.len());
        self.requests.push(Request {
            entry,
            next: None,
            children: Chain::default(),
        });
        let chain = if let Some(owner) = self.owner {
            &mut self.requests[owner.0].children
        } else {
            &mut self.roots
        };
        let previous = chain.last.replace(id);
        if chain.first.is_none() {
            chain.first = Some(id);
        }
        if let Some(previous) = previous {
            self.requests[previous.0].next = Some(id);
        }
        id
    }
    pub(super) fn function(&mut self, entry: T) {
        self.append(Some(entry));
    }
    pub(super) fn variable(&mut self) -> RequestId {
        self.append(None)
    }
    pub(super) fn replace_owner(&mut self, owner: Option<RequestId>) -> Option<RequestId> {
        std::mem::replace(&mut self.owner, owner)
    }
    pub(super) fn next(&mut self) -> Option<T> {
        if self.cursors.is_empty() {
            self.cursors.push(Cursor::default());
        }
        loop {
            let cursor = self.cursors.last_mut()?;
            let next = if let Some(last) = cursor.last {
                self.requests[last.0].next
            } else if let Some(parent) = cursor.parent {
                self.requests[parent.0].children.first
            } else {
                self.roots.first
            };
            let Some(next) = next else {
                if self.cursors.len() == 1 {
                    return None;
                }
                self.cursors.pop();
                continue;
            };
            cursor.last = Some(next);
            let request = &self.requests[next.0];
            if let Some(entry) = request.entry {
                return Some(entry);
            }
            self.cursors.push(Cursor {
                parent: Some(next),
                last: None,
            });
        }
    }
    #[cfg(test)]
    pub(super) fn len(&self) -> usize {
        self.requests.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn delayed_variable_children_keep_first_reference_order_without_replaying_bodies() {
        let entries = [1_u8, 2, 3];
        let mut demands = Demands::default();
        let variable = demands.variable();
        demands.function(entries[2]);
        let previous = demands.replace_owner(Some(variable));
        demands.function(entries[0]);
        let nested = demands.variable();
        demands.replace_owner(Some(nested));
        demands.function(entries[1]);
        demands.replace_owner(previous);
        assert_eq!(demands.next(), Some(entries[0]));
        assert_eq!(demands.next(), Some(entries[1]));
        assert_eq!(demands.next(), Some(entries[2]));
        assert_eq!(demands.next(), None);
        demands.function(entries[0]);
        assert_eq!(demands.next(), Some(entries[0]));
        assert_eq!(demands.next(), None);
    }
}

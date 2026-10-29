// Translated from syntax/name_conflicts.h and parser_statements.cpp::read_statement. See NOTICE.
//! Category readings retain typed syntax; shared deferred lambda bodies have query lifetimes.
use super::{
    ParseError,
    ast::{Attribute, Name, ReadingCategory, Statement, StatementReading, StmtId, StmtKind},
    parser::Parser,
    scopes::{Binding, Categories, Category, Guesses},
    storage::List,
};
use crate::support::hash::HashMap;

#[derive(Clone, Copy)]
pub(super) struct StatementConflict {
    pub name: Name,
    pub component: u32,
    pub category: Option<Category>,
}
#[derive(Default)]
pub(super) struct NameConflicts {
    pub active: bool,
    pub take: Option<Category>,
    pub seen: Categories,
    pub statement: Option<StatementConflict>,
    pub uses: u32,
    pub last: Option<Category>,
}
#[derive(Clone, Copy)]
pub(super) enum StatementReader {
    Ordinary,
    Value,
    Control,
}

#[derive(Clone, Copy)]
pub(super) struct BodyRegion {
    previous: Option<u64>,
    mark: usize,
}
#[derive(Default)]
pub(super) struct SharedLambdas {
    bodies: HashMap<(u64, u64), StmtId>,
    keys: Vec<(u64, u64)>,
    pub region: Option<u64>,
    next_region: u64,
}
impl SharedLambdas {
    pub(super) fn mark(&self) -> usize {
        self.keys.len()
    }
    pub(super) fn rewind(&mut self, mark: usize) {
        while self.keys.len() > mark {
            if let Some(key) = self.keys.pop() {
                self.bodies.remove(&key);
            }
        }
    }
    fn enter(&mut self) -> Result<BodyRegion, ParseError> {
        self.next_region = self
            .next_region
            .checked_add(1)
            .ok_or(ParseError::Limit("statement reading region limit"))?;
        let previous = self.region.replace(self.next_region);
        Ok(BodyRegion {
            previous,
            mark: self.keys.len(),
        })
    }
    fn leave(&mut self, region: BodyRegion) {
        self.rewind(region.mark);
        self.region = region.previous;
    }
    pub(super) fn lookup(&self, region: Option<u64>, begin: u64) -> Option<StmtId> {
        self.bodies.get(&(region?, begin)).copied()
    }
    pub(super) fn keep(&mut self, region: Option<u64>, begin: u64, body: StmtId) {
        if let Some(region) = region {
            let key = (region, begin);
            if let std::collections::hash_map::Entry::Vacant(entry) = self.bodies.entry(key) {
                entry.insert(body);
                self.keys.push(key);
            }
        }
    }
}
impl Parser<'_> {
    pub(super) fn guessed_component(&mut self, binding: Option<Binding>) -> Option<Binding> {
        let category = binding.map(|binding| binding.category);
        let mut alternatives =
            binding.map_or(Categories::default(), |binding| binding.alternatives);
        if alternatives.is_empty() {
            if !self.conflicts.active {
                return binding;
            }
            alternatives = match category {
                None => Categories::one(Category::Type).union(Categories::one(Category::Value)),
                Some(Category::Value) => Categories::one(Category::Type),
                Some(Category::Type) => Categories::one(Category::Value),
                _ => return binding,
            };
        }
        self.conflicts.uses = self.conflicts.uses.wrapping_add(1);
        self.conflicts.seen = self.conflicts.seen.union(alternatives);
        let selected = if let Some(take) = self
            .conflicts
            .take
            .filter(|take| alternatives.contains(*take))
        {
            Some(Binding {
                pack: false,
                category: take,
                scope: None,
                parameter: false,
                specialization: 0,
                alternatives: Categories::default(),
                guesses: Guesses::default(),
            })
        } else {
            binding
        };
        self.conflicts.last = selected.map(|binding| binding.category);
        selected
    }
    pub(super) fn with_conflict_scope<T>(
        &mut self,
        parse: impl FnOnce(&mut Self) -> Result<T, ParseError>,
    ) -> Result<T, ParseError> {
        let take = self.conflicts.take.take();
        let seen = std::mem::take(&mut self.conflicts.seen);
        let statement = self.conflicts.statement.take();
        let result = parse(self);
        self.conflicts.take = take;
        self.conflicts.seen = seen;
        self.conflicts.statement = statement;
        result
    }
    pub(super) fn read_statement(
        &mut self,
        reader: StatementReader,
        attributes: List<Attribute>,
    ) -> Result<StmtId, ParseError> {
        if !(self.scopes.has_conflicts()
            || (self.scopes.uncertain() && !self.scopes.in_template_definition()))
        {
            return self.statement_reading(reader, attributes);
        }
        let active = std::mem::replace(&mut self.conflicts.active, true);
        let region = self.shared_lambdas.enter()?;
        self.cursor.retain();
        let result =
            self.with_conflict_scope(|parser| parser.statement_alternatives(reader, attributes));
        self.cursor.release();
        self.shared_lambdas.leave(region);
        self.conflicts.active = active;
        result
    }
    fn statement_alternatives(
        &mut self,
        reader: StatementReader,
        attributes: List<Attribute>,
    ) -> Result<StmtId, ParseError> {
        let begin = self.cursor.position;
        let location = self.location()?;
        let mut end = begin;
        let mut readings: [Option<StatementReading>; 4] = [None; 4];
        let mut count = 0;
        let mut first = None;
        let mut seen = Categories::default();
        for take in [
            None,
            Some(Category::Value),
            Some(Category::Type),
            Some(Category::TypeTemplate),
            Some(Category::FunctionTemplate),
            Some(Category::Namespace),
        ] {
            if take.is_some_and(|take| !seen.contains(take)) {
                continue;
            }
            self.cursor.position = begin;
            self.conflicts.take = take;
            self.conflicts.statement = None;
            self.conflicts.seen = Categories::default();
            let parsed = self.attempt(|parser| {
                let statement = parser.statement_reading(reader, attributes)?;
                let conflict = parser.conflicts.statement;
                let category = ReadingCategory::of(conflict.and_then(|conflict| conflict.category));
                if count != 0
                    && (parser.cursor.position != end
                        || first.is_none()
                        || conflict.is_none()
                        || readings[..count]
                            .iter()
                            .flatten()
                            .any(|reading| reading.category == category))
                {
                    return Err(ParseError::Syntax {
                        location: parser.location()?,
                        message: "syntax error",
                    });
                }
                parser.conflicts.take = None;
                Ok((
                    StatementReading {
                        statement,
                        category,
                    },
                    conflict,
                    parser.cursor.position,
                ))
            })?;
            if take.is_none() {
                seen = self.conflicts.seen;
            }
            if let Some((reading, conflict, position)) = parsed {
                if count == 0 {
                    first = conflict;
                }
                let slot = readings
                    .get_mut(count)
                    .ok_or(ParseError::Invariant("too many statement readings"))?;
                *slot = Some(reading);
                count += 1;
                end = position;
            }
        }
        self.cursor.position = end;
        if count == 0 {
            return self.fail();
        }
        if count == 1 {
            return readings[0]
                .map(|reading| reading.statement)
                .ok_or(ParseError::Invariant("missing statement reading"));
        }
        let conflict = first.ok_or(ParseError::Invariant("missing statement conflict"))?;
        let name = self.tree.names.get(conflict.name);
        if self
            .tree
            .name_lists
            .get(name.components)
            .get(conflict.component as usize)
            .is_none()
        {
            return Err(ParseError::Invariant("invalid conflicted name component"));
        }
        let start = self.tree.statement_readings.begin();
        for reading in readings[..count].iter().flatten() {
            self.tree.statement_readings.push(*reading);
        }
        let readings = self.tree.statement_readings.finish(start)?;
        self.tree.statements.add(Statement {
            selection_init: false,
            location,
            kind: StmtKind::Ambiguous {
                name: conflict.name,
                component: conflict.component,
                readings,
            },
        })
    }
}

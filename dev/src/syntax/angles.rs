// Translated from syntax/parser_state.cpp and parser_names.cpp. See NOTICE.
//! Immutable delimiter fences and coordinated failure of nested speculative lists.
use super::scopes::{Category, ScopeId};
use super::{
    ParseError,
    ast::{ExprKind, TemplateArgument, TemplateArgumentKind},
    cursor::TokenKind,
    parser::Parser,
    storage::List,
};
use crate::lex::pp_lexer::Punctuator as P;
use crate::support::hash::HashSet;
use std::collections::BTreeMap;

#[derive(Clone, Copy)]
struct Fence {
    begin: u64,
    end: u64,
    found: bool,
}
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct TemplateListKey {
    position: u64,
    environment: u64,
    scope: Option<ScopeId>,
    category: Option<Category>,
    alternatives_active: bool,
    call_follows: bool,
}
#[derive(Default)]
pub(super) struct Angles {
    fences: BTreeMap<u64, Fence>,
    scan_runs: Vec<Fence>,
    failed: HashSet<TemplateListKey>,
    open: Vec<TemplateListKey>,
    abandoning: bool,
    open_base: usize,
    abandoned_error: Option<(u64, crate::support::source::SourceOffset)>,
}
impl Angles {
    pub(super) fn discard_caches(&mut self) {
        self.failed.clear();
        self.fences.clear();
    }
    #[cfg(test)]
    pub(super) fn cached_queries(&self) -> (usize, usize) {
        (self.failed.len(), self.fences.len())
    }
}
impl Parser<'_> {
    pub(super) fn without_angle<T>(
        &mut self,
        parse: impl FnOnce(&mut Self) -> Result<T, ParseError>,
    ) -> Result<T, ParseError> {
        self.with_angle(false, parse)
    }
    pub(super) fn with_angle<T>(
        &mut self,
        angle: bool,
        parse: impl FnOnce(&mut Self) -> Result<T, ParseError>,
    ) -> Result<T, ParseError> {
        let saved = self.angle;
        self.angle = angle;
        let result = parse(self);
        self.angle = saved;
        result
    }
    pub(super) fn speculative_list_boundary<T>(
        &mut self,
        parse: impl FnOnce(&mut Self) -> Result<T, ParseError>,
    ) -> Result<T, ParseError> {
        let open_base = self.angles.open_base;
        self.angles.open_base = self.angles.open.len();
        let abandoning = self.angles.abandoning;
        let abandoned_error = self.angles.abandoned_error.take();
        self.angles.abandoning = false;
        let result = parse(self);
        self.angles.open.truncate(self.angles.open_base);
        self.angles.open_base = open_base;
        self.angles.abandoning = abandoning;
        self.angles.abandoned_error = abandoned_error;
        result
    }
    pub(super) fn optional_template_arguments(
        &mut self,
        call_follows: bool,
    ) -> Result<Option<List<TemplateArgument>>, ParseError> {
        // Arguments establish their own angle mode. Name-category readings
        // and the member-call requirement can change the result even when
        // the token position and reversible scope environment are identical.
        let key = TemplateListKey {
            position: self.cursor.absolute_position()?,
            environment: self.scopes.environment(),
            scope: self.scopes.current,
            category: self.conflicts.take,
            alternatives_active: self.conflicts.active,
            call_follows,
        };
        if self.angles.abandoning
            || self.angles.failed.contains(&key)
            || !self.closing_angle_ahead()?
        {
            return Ok(None);
        }
        let outermost = self.angles.open.len() == self.angles.open_base;
        self.angles.open.push(key);
        let parsed = self.attempt(|parser| {
            let arguments = parser.template_arguments()?;
            if parser.angles.abandoning {
                return Err(ParseError::Syntax {
                    location: parser.location()?,
                    message: "syntax error",
                });
            }
            if call_follows
                && (!(parser.at(P::LeftParen)? || parser.at(P::Scope)?)
                    || !parser.member_arguments_plausible(arguments))
            {
                return parser.fail();
            }
            Ok(arguments)
        })?;
        if !outermost && parsed.is_none() {
            for &open in &self.angles.open[self.angles.open_base..] {
                self.angles.failed.insert(open);
            }
            if !self.angles.abandoning {
                self.angles.abandoned_error = Some(self.error_mark());
            }
            self.angles.abandoning = true;
        }
        if outermost {
            self.angles.open.truncate(self.angles.open_base);
            if self.angles.abandoning
                && let Some(mark) = self.angles.abandoned_error.take()
            {
                self.restore_error(mark);
            }
            self.angles.abandoning = false;
            if parsed.is_none() {
                self.angles.failed.insert(key);
            }
        } else if parsed.is_some() {
            self.angles.open.pop();
        } else {
            // Only the outermost speculative list may fall back to a value
            // reading. A nested failure abandons every enclosing open list.
            return Err(ParseError::Syntax {
                location: self.location()?,
                message: "syntax error",
            });
        }
        Ok(parsed)
    }
    fn member_arguments_plausible(&self, arguments: List<TemplateArgument>) -> bool {
        self.tree.template_arguments.get(arguments).iter().all(|argument| {
            let TemplateArgumentKind::Expression(expression) = argument.kind else { return true; };
            !matches!(self.tree.expressions.get(expression).kind, ExprKind::Binary { op, .. } if matches!(op.token.as_punctuator(), Some(P::LogicalAnd | P::LogicalOr)))
        })
    }
    fn closing_angle_ahead(&mut self) -> Result<bool, ParseError> {
        let base = self.cursor.absolute_position()?;
        if let Some((_, fence)) = self.angles.fences.range(..=base).next_back()
            && base <= fence.end
        {
            return Ok(fence.found);
        }
        self.angles.scan_runs.clear();
        let mut begin = base;
        let mut depth = 0usize;
        let mut ahead = 1usize;
        let found = loop {
            let token = self.cursor.peek(ahead)?;
            let punct = match token.kind {
                TokenKind::Simple(op) => op.token.as_punctuator(),
                _ => None,
            };
            let opener = matches!(punct, Some(P::LeftParen | P::LeftBracket | P::LeftBrace));
            let closer = matches!(punct, Some(P::RightParen | P::RightBracket | P::RightBrace));
            let position = base
                .checked_add(
                    u64::try_from(ahead).map_err(|_| ParseError::Limit("angle lookahead limit"))?,
                )
                .ok_or(ParseError::Limit("angle lookahead limit"))?;
            let found = depth == 0 && punct == Some(P::Greater);
            if matches!(token.kind, TokenKind::End)
                || found
                || (depth == 0 && (closer || punct == Some(P::Semicolon)))
            {
                if depth == 0 {
                    self.angles.scan_runs.push(Fence {
                        begin,
                        end: position - 1,
                        found: false,
                    });
                }
                break found;
            }
            if opener {
                if depth == 0 {
                    self.angles.scan_runs.push(Fence {
                        begin,
                        end: position,
                        found: false,
                    });
                }
                depth += 1;
            } else if closer {
                depth -= 1;
                if depth == 0 {
                    begin = position;
                }
            }
            ahead = ahead
                .checked_add(1)
                .ok_or(ParseError::Limit("angle lookahead limit"))?;
        };
        // Ordered interval keys are required for predecessor queries. New
        // disjoint runs cost O(log n) each; unrelated fences are never sorted.
        for fence in &mut self.angles.scan_runs {
            fence.found = found;
            self.angles.fences.insert(fence.begin, *fence);
        }
        Ok(found)
    }
}

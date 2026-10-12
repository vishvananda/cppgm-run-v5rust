// Translated from syntax/parser_expressions.cpp and parser.cpp. See NOTICE.
//! Lambda syntax and commit-time body demand; scopes follow source order.
use super::{
    ParseError,
    ast::{
        CaptureDefault, CaptureKind, ExprId, ExprKind, Expression, Lambda, LambdaBody,
        LambdaCapture, LambdaId, Qualifier, StmtId,
    },
    parser::Parser,
    scopes::ScopeId,
    storage::List,
};
use crate::lex::{pp_lexer::Punctuator as P, vocabulary::Keyword as K};

#[derive(Clone, Copy)]
pub(super) struct DeferredLambda {
    target: LambdaId,
    begin: usize,
    absolute: u64,
    end: usize,
    scope: ScopeId,
    log: usize,
}
impl Parser<'_> {
    pub(super) fn lambda(&mut self) -> Result<ExprId, ParseError> {
        let location = self.location()?;
        self.expect(P::LeftBracket)?;
        let capture_default = if self.at(P::BitAnd)?
            && (self.at_ahead(P::Comma, 1)? || self.at_ahead(P::RightBracket, 1)?)
        {
            self.cursor.advance()?;
            Some(CaptureDefault::Reference)
        } else if self.accept(P::Assign)? {
            Some(CaptureDefault::Copy)
        } else {
            None
        };
        let start = self.tree.captures.begin();
        let mut first = capture_default.is_none();
        while !self.at(P::RightBracket)? {
            if !first {
                self.expect(P::Comma)?;
            }
            first = false;
            let kind = if self.accept_keyword(K::This)? {
                CaptureKind::This
            } else if self.accept(P::BitAnd)? {
                CaptureKind::Reference(self.identifier()?)
            } else {
                CaptureKind::Copy(self.identifier()?)
            };
            let pack = self.accept(P::Ellipsis)?;
            self.tree.captures.push(LambdaCapture { kind, pack });
        }
        self.expect(P::RightBracket)?;
        let captures = self.tree.captures.finish(start)?;
        let pending = self.pending_template;
        let function = self.function_scope;
        self.pending_template = false;
        let result = self.lambda_after_captures(capture_default, captures);
        self.pending_template = pending;
        self.function_scope = function;
        let lambda = result?;
        self.tree.expressions.add(Expression {
            location,
            kind: ExprKind::Lambda(lambda),
        })
    }
    fn lambda_after_captures(
        &mut self,
        capture_default: Option<CaptureDefault>,
        captures: List<LambdaCapture>,
    ) -> Result<LambdaId, ParseError> {
        let mut lambda = Lambda {
            capture_default,
            captures,
            parameters: None,
            is_mutable: false,
            exception: None,
            trailing: None,
            attributes: List::default(),
            clause_token: self.lambda_token_ordinal()?,
            body_token: 0,
            body: LambdaBody::Deferred,
        };
        let mut scope = self.scopes.current;
        if self.at(P::LeftParen)? {
            lambda.parameters = Some(self.parameter_clause_member(true, None)?);
            scope = self
                .function_scope
                .ok_or(ParseError::Invariant("missing lambda parameter scope"))?;
            let before = self.collect_attributes(false)?;
            lambda.is_mutable = self.accept_keyword(K::Mutable)?;
            lambda.exception = if self.accept_keyword(K::Noexcept)? {
                let operand = if self.accept(P::LeftParen)? {
                    let operand = self.expression()?;
                    self.expect(P::RightParen)?;
                    Some(operand)
                } else {
                    None
                };
                Some(Qualifier::Noexcept(operand))
            } else if self.accept_keyword(K::Throw)? {
                Some(Qualifier::Throw(self.throw_types()?))
            } else {
                None
            };
            let after = self.collect_attributes(false)?;
            let start = self.tree.attributes.begin();
            self.tree.attributes.extend(before);
            self.tree.attributes.extend(after);
            lambda.attributes = self.tree.attributes.finish(start)?;
            if self.accept(P::Arrow)? {
                lambda.trailing = Some(self.type_id(true)?);
            }
        }
        lambda.body_token = self.lambda_token_ordinal()?;
        if lambda.parameters.is_none() {
            lambda.clause_token = lambda.body_token;
        }
        if self.speculation > 0 {
            let begin = self.cursor.position;
            let absolute = self.cursor.absolute_position()?;
            let log = self.scopes.log_position();
            if !self.at(P::LeftBrace)? {
                return self.fail();
            }
            self.skip_balanced()?;
            let target = self.tree.lambdas.add(lambda)?;
            self.lambdas.push(DeferredLambda {
                target,
                begin,
                absolute,
                end: self.cursor.position,
                scope,
                log,
            });
            Ok(target)
        } else {
            lambda.body = LambdaBody::Parsed(self.lambda_body(scope)?);
            self.tree.lambdas.add(lambda)
        }
    }
    fn lambda_token_ordinal(&self) -> Result<u32, ParseError> {
        u32::try_from(self.cursor.absolute_position()?)
            .map_err(|_| ParseError::Limit("lambda token ordinal limit"))
    }
    fn lambda_body(&mut self, scope: ScopeId) -> Result<StmtId, ParseError> {
        let enclosing = self.scopes.current;
        let pending = self.pending_template;
        self.scopes.current = scope;
        self.pending_template = false;
        let result = self.without_angle(|parser| {
            parser.without_declaration_attributes(Self::compound_statement)
        });
        self.scopes.current = enclosing;
        self.pending_template = pending;
        result
    }
    pub(super) fn speculation_boundary<T>(
        &mut self,
        parse: impl FnOnce(&mut Self) -> Result<T, ParseError>,
    ) -> Result<T, ParseError> {
        let saved = self.speculation;
        self.speculation = 0;
        let result = self.without_angle(parse);
        self.speculation = saved;
        result
    }
    pub(super) fn parse_deferred_lambdas(&mut self, first: usize) -> Result<(), ParseError> {
        let last = self.lambdas.len();
        if first == last {
            return Ok(());
        }
        let resume = self.cursor.position;
        let enclosing = self.scopes.current;
        let function = self.function_scope;
        let pending = self.pending_template;
        let region = self.shared_lambdas.region.take();
        let base = self.lambdas[first].log;
        let mut later = std::mem::take(&mut self.lambda_changes);
        self.scopes.detach(base, &mut later);
        let mut attached = 0;
        let result = (|| {
            for index in first..last {
                let deferred = self.lambdas[index];
                let next = deferred.log - base;
                self.scopes.reattach(&later[attached..next]);
                attached = next;
                let body = if let Some(body) = self.shared_lambdas.lookup(region, deferred.absolute)
                {
                    body
                } else {
                    self.cursor.position = deferred.begin;
                    self.deferred_lambdas += 1;
                    let body =
                        self.speculation_boundary(|parser| parser.lambda_body(deferred.scope))?;
                    if self.cursor.position != deferred.end {
                        return self.fail();
                    }
                    self.shared_lambdas.keep(region, deferred.absolute, body);
                    body
                };
                let mut lambda = *self.tree.lambdas.get(deferred.target);
                if !matches!(lambda.body, LambdaBody::Deferred) {
                    return Err(ParseError::Invariant("lambda body already parsed"));
                }
                lambda.body = LambdaBody::Parsed(body);
                self.tree.lambdas.replace(deferred.target, lambda);
            }
            Ok(())
        })();
        self.scopes.reattach(&later[attached..]);
        later.clear();
        self.lambda_changes = later;
        self.lambdas.truncate(first);
        self.cursor.position = resume;
        self.scopes.current = enclosing;
        self.function_scope = function;
        self.pending_template = pending;
        self.shared_lambdas.region = region;
        match result {
            Err(ParseError::Syntax { location, message }) => {
                Err(ParseError::CommittedSyntax { location, message })
            }
            other => other,
        }
    }
}

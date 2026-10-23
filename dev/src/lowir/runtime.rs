//! Runtime instruction grammar from C++ `text_parser_body.cpp`. See NOTICE.
use super::{
    Error,
    lexer::Kind,
    model::{Body, RuntimeInstruction as R, Value},
    reader::Parser,
    vocabulary::{AtomicUpdate, Fence, MemoryOrder, Type},
};

impl Parser<'_> {
    fn memory_order(&mut self) -> Result<MemoryOrder, Error> {
        let number = self.small_integer("memory order")?;
        MemoryOrder::from_number(number).ok_or_else(|| self.fail("invalid memory order"))
    }
    pub(super) fn runtime_rvalue(
        &mut self,
        body: &mut Body,
        head: &str,
    ) -> Result<Option<R>, Error> {
        if let Some(operation) = AtomicUpdate::parse(head) {
            let ty = self.parse_type()?;
            let address = self.value(body, Type::Ptr)?;
            self.expect(Kind::Comma, ",")?;
            let value = self.value(body, ty)?;
            self.expect(Kind::Comma, ",")?;
            return Ok(Some(R::AtomicUpdate(
                operation,
                ty,
                address,
                value,
                self.memory_order()?,
            )));
        }
        Ok(Some(match head {
            "atomic_load" => {
                let ty = self.parse_type()?;
                let address = self.value(body, Type::Ptr)?;
                self.expect(Kind::Comma, ",")?;
                R::AtomicLoad(ty, address, self.memory_order()?)
            }
            "atomic_compare_exchange" => {
                let ty = self.parse_type()?;
                let address = self.value(body, Type::Ptr)?;
                self.expect(Kind::Comma, ",")?;
                let expected = self.value(body, Type::Ptr)?;
                self.expect(Kind::Comma, ",")?;
                let desired = self.value(body, ty)?;
                self.expect(Kind::Comma, ",")?;
                let success = self.memory_order()?;
                self.expect(Kind::Comma, ",")?;
                R::AtomicCompareExchange {
                    ty,
                    address,
                    expected,
                    desired,
                    success,
                    failure: self.memory_order()?,
                }
            }
            "va_arg" => {
                let ty = self.parse_type()?;
                let list = self.value(body, Type::Void)?;
                let classes = if self.accept(Kind::LBracket)? {
                    if !self.object_facts || !self.at_word("classes") {
                        return Err(self.fail("invalid va_arg metadata"));
                    }
                    self.advance()?;
                    self.expect(Kind::Equals, "=")?;
                    let classes = self.object_classes(Type::Object {
                        bytes: 16,
                        align: 3,
                    })?;
                    self.expect(Kind::RBracket, "]")?;
                    Some(classes)
                } else {
                    None
                };
                R::VaArg(ty, list, classes)
            }
            "stack_alloc" => R::StackAlloc(self.value(body, Type::Void)?),
            "exception" => R::Exception(self.parse_type()?),
            "exception_selector" => R::ExceptionSelector(self.parse_type()?),
            _ => return Ok(None),
        }))
    }
    pub(super) fn runtime_effect(
        &mut self,
        body: &mut Body,
        head: &str,
    ) -> Result<Option<R>, Error> {
        if let Some(fence) = Fence::parse(head) {
            return Ok(Some(R::Fence(fence, self.memory_order()?)));
        }
        Ok(Some(match head {
            "atomic_store" => {
                let ty = self.parse_type()?;
                let value = self.value(body, ty)?;
                self.expect(Kind::Comma, ",")?;
                let address = self.value(body, Type::Ptr)?;
                self.expect(Kind::Comma, ",")?;
                R::AtomicStore(ty, value, address, self.memory_order()?)
            }
            "va_start" => R::VaStart(self.value(body, Type::Ptr)?),
            "eh_try" => R::EhTry(self.target(body)?),
            "eh_cleanup" => R::EhCleanup(if self.at(Kind::Block) {
                Some(self.target(body)?)
            } else {
                None
            }),
            "eh_catch" => {
                let symbol = self.symbol_reference("catch type symbol")?;
                let selector = self.optional_selector("catch selector")?;
                R::EhCatch(symbol, selector)
            }
            "eh_filter" => self.eh_filter(body)?,
            "eh_catch_all" => R::EhCatchAll(self.optional_selector("catch-all selector")?),
            "eh_end" => R::EhEnd,
            "throw" => {
                let ty = self.parse_type()?;
                R::Throw(ty, self.value(body, ty)?)
            }
            "resume" => R::Resume,
            _ => return Ok(None),
        }))
    }
    fn optional_selector(&mut self, what: &str) -> Result<Option<i64>, Error> {
        if self.accept(Kind::Comma)? {
            Ok(Some(self.small_integer(what)?))
        } else {
            Ok(None)
        }
    }
    fn eh_filter(&mut self, body: &mut Body) -> Result<R, Error> {
        self.arguments.clear();
        // The writer omits the grammar's initial comma for a selector-only filter.
        let mut has_selector =
            self.at(Kind::Number) || self.at(Kind::Minus) || self.accept(Kind::Comma)?;
        while !has_selector && self.at(Kind::Global) {
            // Unlike catch's expect-name path, C++ references filters before advancing.
            let token = self.lexer.token;
            let name = self.program.names.intern(token.word())?;
            let symbol = self.program.reference(name, self.source_id, token.line)?;
            self.advance()?;
            self.arguments.push(Value::Symbol(symbol));
            if !self.accept(Kind::Comma)? {
                break;
            }
            has_selector = !self.at(Kind::Global);
        }
        let selector = if has_selector {
            Some(self.small_integer("filter selector")?)
        } else {
            None
        };
        Ok(R::EhFilter(body.add_arguments(&self.arguments)?, selector))
    }
}

//! Unit storage, structured data and object alias grammar. See NOTICE.
use super::{
    Error,
    lexer::Kind,
    model::{Boundary, DataItem, Global, GlobalInit, Metadata, ObjectAlias, SymbolId},
    reader::Parser,
    vocabulary::{Storage, Type},
};
impl Parser<'_> {
    pub(super) fn global_item(&mut self, definition: bool) -> Result<(), Error> {
        let line = self.lexer.token.line;
        let name = self.name(Kind::Global, "global name")?;
        let symbol = self
            .program
            .reference(name, self.source_id, self.lexer.token.line)?;
        if self.program.symbol(symbol).declared() {
            return Err(self.fail_at(
                line,
                format!(
                    "duplicate top-level symbol: {}",
                    self.program.names.get(name)
                ),
            ));
        }
        let mut metadata = Metadata::default();
        if self.at(Kind::Word)
            && let Some(storage) = Storage::parse(self.lexer.token.word())
        {
            metadata.storage = Some(storage);
            self.lexer.advance();
        }
        let ty = if self.accept(Kind::Colon) {
            Some(self.parse_type()?)
        } else {
            None
        };
        self.metadata(
            Some(&mut metadata),
            &mut Boundary::default(),
            Type::Void,
            true,
        )?;
        let init = if definition {
            self.expect(Kind::Equals, "=")?;
            if self.at(Kind::LBrace) {
                if ty.is_some_and(|ty| !matches!(ty, Type::Object { .. })) {
                    return Err(
                        self.fail("a structured global's storage type must be an object type")
                    );
                }
                GlobalInit::Data(self.global_data()?)
            } else {
                let Some(ty) = ty else {
                    return Err(self.fail("scalar global requires type"));
                };
                if self.accept_word("zero") {
                    GlobalInit::Zero
                } else if self.accept_word("addr") {
                    let (target, addend) =
                        self.address_initializer("address initializer symbol")?;
                    GlobalInit::Address(target, addend)
                } else {
                    GlobalInit::Constant(self.constant(ty)?)
                }
            }
        } else {
            GlobalInit::Declaration
        };
        self.program.add_global(
            symbol,
            Global {
                name,
                ty,
                metadata,
                init,
            },
        )
    }
    fn global_data(&mut self) -> Result<Vec<DataItem>, Error> {
        self.expect(Kind::LBrace, "{")?;
        let mut items = Vec::new();
        while !self.accept(Kind::RBrace) {
            let item = if self.accept_word("zero") {
                if !self.at(Kind::Number) {
                    return Err(self.fail("expected positive integer"));
                }
                let amount = self.positive(self.lexer.token.word())?;
                // The C++ record stores the u64 count modulo 2^64 as signed i64.
                let amount = i64::from_le_bytes(amount.to_le_bytes());
                self.lexer.advance();
                DataItem::Zero(amount)
            } else {
                let ty = self.parse_type()?;
                if ty == Type::Ptr && self.accept_word("addr") {
                    let (target, addend) = self.address_initializer("address target")?;
                    DataItem::Address(target, addend)
                } else {
                    DataItem::Constant(ty, self.constant(ty)?)
                }
            };
            u32::try_from(items.len()).map_err(|_| Error::Limit)?;
            items.push(item);
        }
        if items.is_empty() {
            return Err(self.fail("structured global must contain data"));
        }
        Ok(items)
    }
    fn address_initializer(&mut self, what: &str) -> Result<(SymbolId, i64), Error> {
        let target = self.symbol_reference(what)?;
        let addend = if self.at(Kind::Plus) || self.at(Kind::Minus) {
            let negative = self.at(Kind::Minus);
            self.lexer.advance();
            let addend = self.small_integer("address addend")?;
            if negative {
                addend.wrapping_neg()
            } else {
                addend
            }
        } else {
            0
        };
        Ok((target, addend))
    }
    pub(super) fn symbol_reference(&mut self, what: &str) -> Result<SymbolId, Error> {
        let name = self.name(Kind::Global, what)?;
        self.program
            .reference(name, self.source_id, self.lexer.token.line)
    }
    pub(super) fn object_alias(&mut self) -> Result<(), Error> {
        if !self.accept_word("object") {
            return Err(self.fail_token("expected 'object', got '", "'"));
        }
        let line = self.lexer.token.line;
        if !self.at(Kind::Word) {
            return Err(self.fail("invalid object alias spelling"));
        }
        let object = self.name(Kind::Word, "object alias spelling")?;
        if self.program.has_alias(object) {
            return Err(self.fail_at(line, "duplicate object alias"));
        }
        self.expect(Kind::Equals, "=")?;
        let target = self.symbol_reference("alias target")?;
        if !self.program.add_alias(ObjectAlias { object, target }) {
            return Err(self.fail_at(line, "duplicate object alias"));
        }
        Ok(())
    }
}

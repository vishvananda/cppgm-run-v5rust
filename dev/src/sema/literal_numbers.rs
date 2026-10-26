// Translated from analyzer_literals.cpp and expression_lowering.cpp::literal. See NOTICE.
//! Cooked numeric operands own decoding, cached once at explicit value demand.
use super::Error;
use crate::{
    syntax::ast,
    types::{FundamentalType, floating::FloatingScanner},
};
use std::{
    cell::{OnceCell, RefCell},
    collections::HashMap,
};

#[derive(Clone, Copy, Debug)]
pub(crate) struct NumberId(u32);

#[derive(Clone, Copy, Debug)]
pub(crate) enum CookedFloating {
    Value(u128),
    OutsideRange,
    ScanLimit,
}

struct Number {
    digits: Box<[u8]>,
    value: OnceCell<CookedFloating>,
}

#[derive(Default)]
pub(super) struct Numbers {
    // Each original literal expression has immutable digits. All entries use
    // the selected cooked operator's long-double parameter precision.
    expressions: HashMap<ast::ExprId, NumberId>,
    entries: Vec<Number>,
    scanner: RefCell<FloatingScanner>,
}
impl Numbers {
    pub(super) fn register(
        &mut self,
        source: ast::ExprId,
        digits: &[u8],
    ) -> Result<NumberId, Error> {
        if let Some(&id) = self.expressions.get(&source) {
            if self.entries[id.0 as usize].digits.as_ref() != digits {
                return Err(Error::Invariant(
                    "cooked literal digits changed after registration",
                ));
            }
            return Ok(id);
        }
        let id = NumberId(
            u32::try_from(self.entries.len())
                .map_err(|_| Error::Limit("cooked numeric operands"))?,
        );
        self.entries.push(Number {
            digits: digits.into(),
            value: OnceCell::new(),
        });
        self.expressions.insert(source, id);
        Ok(id)
    }

    pub(super) fn value(&self, id: NumberId) -> CookedFloating {
        let number = &self.entries[id.0 as usize];
        *number.value.get_or_init(|| {
            match self
                .scanner
                .borrow_mut()
                .scan(&number.digits, FundamentalType::LongDouble)
            {
                Ok(Some(data)) => {
                    let mut bytes = [0; 16];
                    bytes[..data.as_slice().len()].copy_from_slice(data.as_slice());
                    CookedFloating::Value(u128::from_le_bytes(bytes))
                }
                Ok(None) => CookedFloating::OutsideRange,
                Err(_) => CookedFloating::ScanLimit,
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_and_range_failures_are_decoded_only_once_on_demand() {
        let mut numbers = Numbers::default();
        // Register through real syntax IDs without constructing another tree.
        let mut expressions = crate::syntax::storage::Store::<ast::Expression>::default();
        let mut ids = Vec::new();
        for digits in [b"1.5".as_slice(), b"1e99999"] {
            let source = expressions
                .add(ast::Expression {
                    location: crate::support::source::SourceOffset(0),
                    kind: ast::ExprKind::Keyword(crate::lex::vocabulary::Keyword::True),
                })
                .unwrap();
            let id = numbers.register(source, digits).unwrap();
            assert_eq!(numbers.register(source, digits).unwrap().0, id.0);
            assert!(numbers.entries[id.0 as usize].value.get().is_none());
            ids.push(id);
        }
        assert!(matches!(
            numbers.value(ids[0]),
            CookedFloating::Value(0x3fff_c000_0000_0000_0000)
        ));
        assert!(numbers.entries[ids[1].0 as usize].value.get().is_none());
        assert!(matches!(
            numbers.value(ids[1]),
            CookedFloating::OutsideRange
        ));
        assert!(matches!(
            numbers.value(ids[1]),
            CookedFloating::OutsideRange
        ));
    }
}

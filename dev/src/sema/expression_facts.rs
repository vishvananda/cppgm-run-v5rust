// Translated from sema/expression_facts.h and conversions.h. See NOTICE.
//! Dense facts on the original AST; conversions never overwrite source types.
use super::{Error, types::TypeId};
use crate::syntax::ast;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ValueCategory {
    Lvalue,
    Prvalue,
    Xvalue,
}
impl ValueCategory {
    pub(super) fn spelling(self) -> &'static str {
        match self {
            Self::Lvalue => "lvalue",
            Self::Prvalue => "prvalue",
            Self::Xvalue => "xvalue",
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum LvalueTransform {
    None,
    Read,
    ArrayToPointer,
    FunctionToPointer,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ValueConversion {
    None,
    IntegralPromotion,
    FloatingPromotion,
    Arithmetic,
    Boolean,
    PointerBoolean,
    NullPointer,
    Pointer,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ConversionRank {
    Exact,
    Promotion,
    Conversion,
}
#[derive(Clone, Copy, Debug)]
pub(super) struct Conversion {
    pub target: TypeId,
    pub result: TypeId,
    pub first: LvalueTransform,
    pub second: ValueConversion,
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "Standard-conversion rank is retained for the next overload-resolution group"
        )
    )]
    pub rank: ConversionRank,
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "Qualification ranking is retained independently of the value conversion for overload resolution"
        )
    )]
    pub qualification: bool,
    pub initialization: bool,
    pub temporary: bool,
}
#[derive(Clone, Copy, Debug)]
pub(super) struct ExprFacts {
    pub ty: TypeId,
    pub category: ValueCategory,
    pub null_literal: bool,
    pub conversion: Option<Conversion>,
    pub commuted: bool,
    pub folded: Option<u64>,
    pub reference_cast: Option<TypeId>,
    pub functional_call: bool,
}
impl ExprFacts {
    pub(super) fn new(ty: TypeId, category: ValueCategory) -> Self {
        Self {
            ty,
            category,
            null_literal: false,
            conversion: None,
            commuted: false,
            folded: None,
            reference_cast: None,
            functional_call: false,
        }
    }
}
#[derive(Default)]
pub(super) struct ExpressionFacts {
    records: Vec<Option<ExprFacts>>,
    // Parentheses share facts, as in the C++ owner. Chains are flattened at publication.
    aliases: Vec<Option<ast::ExprId>>,
}
impl ExpressionFacts {
    pub(super) fn root(&self, id: ast::ExprId) -> ast::ExprId {
        self.aliases
            .get(id.index())
            .copied()
            .flatten()
            .unwrap_or(id)
    }
    pub(super) fn get(&self, id: ast::ExprId) -> Result<ExprFacts, Error> {
        self.records
            .get(self.root(id).index())
            .copied()
            .flatten()
            .ok_or(Error::Invariant("expression without semantic facts"))
    }
    pub(super) fn set(&mut self, id: ast::ExprId, facts: ExprFacts) {
        let id = self.root(id);
        super::record(&mut self.records, id.index(), facts);
    }
    pub(super) fn share(&mut self, id: ast::ExprId, inner: ast::ExprId) {
        let root = self.root(inner);
        super::record(&mut self.aliases, id.index(), root);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        lex::vocabulary::Keyword, support::source::SourceOffset, syntax::storage::Store,
        types::FundamentalType,
    };
    #[test]
    fn parentheses_share_one_conversion_record_without_recursive_alias_lookup() {
        let mut types = super::super::types::Types::default();
        let int = types
            .scalar(super::super::types::Scalar::Basic(FundamentalType::Int))
            .unwrap();
        let long = types
            .scalar(super::super::types::Scalar::Basic(FundamentalType::LongInt))
            .unwrap();
        let mut syntax = Store::default();
        let leaf = syntax
            .add(ast::Expression {
                location: SourceOffset(0),
                kind: ast::ExprKind::Keyword(Keyword::True),
            })
            .unwrap();
        let mut facts = ExpressionFacts::default();
        facts.set(leaf, ExprFacts::new(int, ValueCategory::Lvalue));
        let mut outer = leaf;
        for _ in 0..4096 {
            let parent = syntax
                .add(ast::Expression {
                    location: SourceOffset(0),
                    kind: ast::ExprKind::Paren(outer),
                })
                .unwrap();
            facts.share(parent, outer);
            assert_eq!(facts.root(parent), leaf);
            outer = parent;
        }
        let mut updated = facts.get(outer).unwrap();
        updated.conversion = Some(Conversion {
            target: long,
            result: long,
            first: LvalueTransform::Read,
            second: ValueConversion::Arithmetic,
            rank: ConversionRank::Conversion,
            qualification: false,
            initialization: true,
            temporary: false,
        });
        facts.set(outer, updated);
        assert_eq!(facts.get(leaf).unwrap().ty, int);
        assert_eq!(facts.get(leaf).unwrap().category, ValueCategory::Lvalue);
        let conversion = facts.get(leaf).unwrap().conversion.unwrap();
        assert_eq!(conversion.target, long);
        assert_eq!(conversion.rank, ConversionRank::Conversion);
        assert!(!conversion.qualification);
        assert_eq!(
            facts.records.iter().filter(|fact| fact.is_some()).count(),
            1
        );
    }
}

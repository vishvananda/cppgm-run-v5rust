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
    DerivedToBase,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum ConversionRank {
    Exact,
    Promotion,
    Conversion,
    UserDefined,
    Ellipsis,
}
#[derive(Clone, Copy, Debug)]
pub(super) enum ReferenceSource {
    Other,
    FunctionLvalue,
}
impl ReferenceSource {
    pub(super) fn is_function_lvalue(self) -> bool {
        matches!(self, Self::FunctionLvalue)
    }
}
#[derive(Clone, Copy, Debug)]
pub(super) struct Conversion {
    pub target: TypeId,
    pub result: TypeId,
    pub first: LvalueTransform,
    pub second: ValueConversion,
    pub rank: ConversionRank,
    pub qualification: bool,
    pub initialization: bool,
    pub temporary: bool,
    pub reference_source: ReferenceSource,
}
#[derive(Clone, Copy, Debug)]
pub(crate) enum ObjectSource {
    Implicit,
    Explicit {
        expression: ast::ExprId,
        arrow: bool,
    },
}
#[derive(Clone, Copy, Debug)]
pub(super) struct ObjectArgument {
    pub source: ObjectSource,
    pub ty: TypeId,
    pub category: ValueCategory,
}
#[derive(Clone, Copy, Debug)]
pub(super) struct BaseCast {
    pub path: super::inheritance::BasePathId,
    pub downcast: bool,
    pub project_zero: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum CallForm {
    Ordinary,
    Operator,
}
#[derive(Clone, Copy, Debug)]
pub(super) enum DestructorTarget {
    Scalar,
    Class(super::graph::BindingId),
}
#[derive(Clone, Copy, Debug)]
pub(crate) enum GlvalueKind {
    Ordinary,
    BitField,
}
#[derive(Clone, Copy, Debug)]
pub(crate) enum ClassInitialization {
    Constructed(super::constructor_initialization::ConstructorArguments),
    Aggregate(crate::syntax::storage::List<ast::ExprId>),
    FromExpression(ast::ExprId),
}
#[derive(Clone, Copy, Debug)]
pub(super) struct ExprFacts {
    pub ty: TypeId,
    pub category: ValueCategory,
    pub glvalue: GlvalueKind,
    pub field_identity: Option<super::bit_fields::BitFieldId>,
    pub null_literal: bool,
    pub conversion: Option<Conversion>,
    pub commuted: bool,
    pub folded: Option<u64>,
    pub fold_style: FoldStyle,
    // Query constants retain their value without replacing the source query's
    // sizeof-expression presentation. Constant reads do not type its operand again.
    pub query_value: Option<u64>,
    pub reference_cast: Option<TypeId>,
    pub functional_call: bool,
    pub intrinsic: Option<super::floating_intrinsics::FloatingIntrinsic>,
    pub functions: Option<super::function_resolution::OverloadId>,
    pub function: Option<super::graph::BindingId>,
    pub call_form: CallForm,
    pub call: Option<super::function_resolution::CallTarget>,
    pub literal_call: Option<super::literal_calls::TextCall>,
    pub destructor: Option<DestructorTarget>,
    pub anonymous_member: Option<super::graph::BindingId>,
    // Explicit designation retains the selected entity independently of spelling.
    pub member: Option<super::graph::BindingId>,
    pub member_naming: Option<super::graph::EntityId>,
    pub object: Option<ObjectArgument>,
    pub base_path: Option<super::inheritance::BasePathId>,
    // The cast's route remains separate from a later conversion on its result.
    pub base_cast: Option<BaseCast>,
    pub member_path: Option<super::inheritance::BasePathId>,
    pub member_naming_path: Option<super::inheritance::BasePathId>,
    // Qualified address formation is distinct from parenthesized designations.
    pub member_address_scope: Option<super::graph::ScopeId>,
    pub constructor: Option<super::construction::ConstructorId>,
    pub class_initialization: Option<ClassInitialization>,
    pub class_copy: Option<super::copy_initialization::CopySelection>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum FoldStyle {
    Literal,
    StaticMember,
    EnclosingLocal,
}
impl ExprFacts {
    pub(super) fn is_bit_field(self) -> bool {
        matches!(self.glvalue, GlvalueKind::BitField)
    }
    pub(super) fn set_bit_field(&mut self, value: bool) {
        self.glvalue = if value {
            GlvalueKind::BitField
        } else {
            GlvalueKind::Ordinary
        };
    }

    pub(super) fn new(ty: TypeId, category: ValueCategory) -> Self {
        Self {
            ty,
            category,
            glvalue: GlvalueKind::Ordinary,
            field_identity: None,
            null_literal: false,
            conversion: None,
            commuted: false,
            folded: None,
            fold_style: FoldStyle::Literal,
            query_value: None,
            reference_cast: None,
            functional_call: false,
            intrinsic: None,
            functions: None,
            function: None,
            call_form: CallForm::Ordinary,
            call: None,
            literal_call: None,
            destructor: None,
            anonymous_member: None,
            member: None,
            member_naming: None,
            object: None,
            base_path: None,
            base_cast: None,
            member_path: None,
            member_naming_path: None,
            member_address_scope: None,
            constructor: None,
            class_initialization: None,
            class_copy: None,
        }
    }
}
#[derive(Clone, Copy)]
struct ClassConversion {
    constructor: super::construction::ConstructorId,
    arguments: super::constructor_initialization::ConstructorArguments,
    outer: Option<Conversion>,
}
#[derive(Default)]
pub(super) struct ExpressionFacts {
    records: Vec<Option<ExprFacts>>,
    // Only nonelided class copies own a lowering action. Selection remains
    // on the source facts, independently of its existing operand conversion.
    copy_actions: std::collections::HashMap<ast::ExprId, super::copy_initialization::CopyAction>,
    // Parentheses share facts, as in the C++ owner. Chains are flattened at publication.
    aliases: Vec<Option<ast::ExprId>>,
    // Only differing-class glvalue conditionals need a base-subobject route.
    conditional_paths:
        std::collections::HashMap<ast::ExprId, [Option<super::inheritance::BasePathId>; 2]>,
    // Converting class-subobject initializations are sparse use facts. Keep
    // them separate from the source's original type and argument conversion.
    class_conversions: std::collections::HashMap<ast::ExprId, ClassConversion>,
}
impl ExpressionFacts {
    pub(super) fn copy_action(
        &self,
        id: ast::ExprId,
    ) -> Option<super::copy_initialization::CopyAction> {
        self.copy_actions.get(&self.root(id)).copied()
    }
    pub(super) fn set_copy_action(
        &mut self,
        id: ast::ExprId,
        action: Option<super::copy_initialization::CopyAction>,
    ) {
        let id = self.root(id);
        if let Some(action) = action {
            self.copy_actions.insert(id, action);
        } else {
            self.copy_actions.remove(&id);
        }
    }
    pub(super) fn conditional_paths(
        &self,
        id: ast::ExprId,
    ) -> [Option<super::inheritance::BasePathId>; 2] {
        self.conditional_paths
            .get(&self.root(id))
            .copied()
            .unwrap_or([None; 2])
    }
    pub(super) fn set_conditional_paths(
        &mut self,
        id: ast::ExprId,
        paths: [Option<super::inheritance::BasePathId>; 2],
    ) {
        self.conditional_paths.insert(self.root(id), paths);
    }
    pub(super) fn class_conversion(
        &self,
        id: ast::ExprId,
    ) -> Option<(
        super::construction::ConstructorId,
        super::constructor_initialization::ConstructorArguments,
    )> {
        self.class_conversions
            .get(&self.root(id))
            .map(|record| (record.constructor, record.arguments))
    }
    pub(super) fn set_class_conversion(
        &mut self,
        id: ast::ExprId,
        selected: (
            super::construction::ConstructorId,
            super::constructor_initialization::ConstructorArguments,
        ),
    ) {
        self.class_conversions.insert(
            self.root(id),
            ClassConversion {
                constructor: selected.0,
                arguments: selected.1,
                outer: None,
            },
        );
    }
    pub(super) fn user_conversion(&self, id: ast::ExprId) -> Option<Conversion> {
        self.class_conversions
            .get(&self.root(id))
            .and_then(|record| record.outer)
    }
    pub(super) fn set_user_conversion(
        &mut self,
        id: ast::ExprId,
        constructor: super::construction::ConstructorId,
        outer: Conversion,
    ) {
        self.class_conversions.insert(
            self.root(id),
            ClassConversion {
                constructor,
                arguments: super::constructor_initialization::ConstructorArguments::Single(id),
                outer: Some(outer),
            },
        );
    }
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
            reference_source: ReferenceSource::Other,
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

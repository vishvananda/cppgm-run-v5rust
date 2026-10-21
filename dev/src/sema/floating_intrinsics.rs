// Translated from analyzer_builtins.cpp::floating_builtin and lex/gnu_builtins.cpp. See NOTICE.
//! Type-generic floating operations retain their selected operation on the source `ExprId`.
use super::{
    Error,
    analyzer::Analyzer,
    builtin_calls::BuiltinFunction,
    expression_facts::{ExprFacts, ValueCategory},
    graph::{Binding, BindingId, BindingKind, ScopeId},
    procedural_calls::Candidates,
    types::{Cv, Scalar, TypeKind},
};
use crate::{
    lex::{PostTokenKind, identifiers::NameId, vocabulary::Keyword},
    syntax::{ast, cursor::Cursor, storage::List},
    types::FundamentalType as F,
};
use std::cmp::Ordering::{Equal, Greater, Less};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FloatingIntrinsic {
    Constant,
    SignalingConstant,
    IsFinite,
    IsInf,
    IsNan,
    IsNormal,
    Classify,
    SignBit,
    IsInfSign,
    IsSignaling,
    IsGreater,
    IsGreaterEqual,
    IsLess,
    IsLessEqual,
    IsLessGreater,
    IsUnordered,
}
impl FloatingIntrinsic {
    pub(crate) fn comparison(self) -> bool {
        matches!(
            self,
            Self::IsGreater
                | Self::IsGreaterEqual
                | Self::IsLess
                | Self::IsLessEqual
                | Self::IsLessGreater
                | Self::IsUnordered
        )
    }
}
#[derive(Clone, Copy)]
pub(crate) enum FloatingOperation {
    Classification(FloatingIntrinsic),
    Constant(ConstantKind, F),
}
pub(super) fn names(cursor: &mut Cursor<'_>) -> Result<Vec<(NameId, FloatingOperation)>, Error> {
    use FloatingIntrinsic as I;
    let mut names: Vec<_> = [
        ("__builtin_isfinite", I::IsFinite),
        ("__builtin_isinf", I::IsInf),
        ("__builtin_isnan", I::IsNan),
        ("__builtin_isnormal", I::IsNormal),
        ("__builtin_fpclassify", I::Classify),
        ("__builtin_signbit", I::SignBit),
        ("__builtin_isinf_sign", I::IsInfSign),
        ("__builtin_issignaling", I::IsSignaling),
        ("__builtin_isgreater", I::IsGreater),
        ("__builtin_isgreaterequal", I::IsGreaterEqual),
        ("__builtin_isless", I::IsLess),
        ("__builtin_islessequal", I::IsLessEqual),
        ("__builtin_islessgreater", I::IsLessGreater),
        ("__builtin_isunordered", I::IsUnordered),
    ]
    .into_iter()
    .map(|(name, kind)| {
        Ok((
            cursor
                .intern_name(name.as_bytes())
                .map_err(|_| Error::Limit("semantic identifier limit"))?,
            FloatingOperation::Classification(kind),
        ))
    })
    .collect::<Result<_, Error>>()?;
    add_constant_names(cursor, &mut names)?;
    Ok(names)
}
impl Analyzer<'_> {
    pub(super) fn floating_builtin(
        &mut self,
        id: ast::ExprId,
        arguments: List<ast::ExprId>,
        operation: FloatingOperation,
        scope: ScopeId,
    ) -> Result<ExprFacts, Error> {
        match operation {
            FloatingOperation::Classification(kind) => {
                self.floating_intrinsic(id, arguments, kind, scope)
            }
            FloatingOperation::Constant(kind, ty) => {
                self.floating_constant(id, arguments, kind, ty, scope)
            }
        }
    }
    pub(super) fn constant_floating_intrinsic(
        &mut self,
        id: ast::ExprId,
        kind: FloatingIntrinsic,
        scope: ScopeId,
    ) -> Result<super::constants::Constant, Error> {
        let ast::ExprKind::Call { arguments, .. } = self.tree.expressions.get(id).kind else {
            return Err(Error::Invariant("constant intrinsic without call"));
        };
        let argument_buffer =
            super::call_arguments::Arguments::new(self.tree.expr_lists.get(arguments));
        let arguments = argument_buffer.as_slice();
        let result = if kind.comparison() {
            let left = self.constant_float_operand(arguments[0], scope)?;
            let right = self.constant_float_operand(arguments[1], scope)?;
            let order = left.compare(right);
            u64::from(match kind {
                FloatingIntrinsic::IsGreater => order == Some(Greater),
                FloatingIntrinsic::IsGreaterEqual => matches!(order, Some(Greater | Equal)),
                FloatingIntrinsic::IsLess => order == Some(Less),
                FloatingIntrinsic::IsLessEqual => matches!(order, Some(Less | Equal)),
                FloatingIntrinsic::IsLessGreater => matches!(order, Some(Less | Greater)),
                _ => order.is_none(),
            })
        } else {
            let argument = *arguments
                .last()
                .ok_or(Error::Invariant("constant intrinsic without operand"))?;
            let value = self.constant_float_operand(argument, scope)?;
            let category = value.category();
            match kind {
                FloatingIntrinsic::IsFinite => u64::from(!matches!(
                    category,
                    FloatCategory::Nan | FloatCategory::Infinite
                )),
                FloatingIntrinsic::IsInf => u64::from(category == FloatCategory::Infinite),
                FloatingIntrinsic::IsNan => u64::from(category == FloatCategory::Nan),
                FloatingIntrinsic::IsNormal => u64::from(category == FloatCategory::Normal),
                FloatingIntrinsic::SignBit => u64::from(value.negative()),
                FloatingIntrinsic::IsInfSign if category != FloatCategory::Infinite => 0,
                FloatingIntrinsic::IsInfSign => {
                    if value.negative() {
                        u64::MAX
                    } else {
                        1
                    }
                }
                FloatingIntrinsic::IsSignaling => {
                    u64::from(category == FloatCategory::Nan && !value.quiet())
                }
                FloatingIntrinsic::Classify => {
                    let index = match category {
                        FloatCategory::Nan => 0,
                        FloatCategory::Infinite => 1,
                        FloatCategory::Normal => 2,
                        FloatCategory::Subnormal => 3,
                        FloatCategory::Zero => 4,
                    };
                    let code = self.evaluate_constant(arguments[index], scope, false)?;
                    let ty = self.graph.expression_facts.get(id)?.ty;
                    let integer = self
                        .graph
                        .integral_kind(ty)
                        .ok_or(Error::Invariant("classification result is not integral"))?;
                    return Ok(super::constants::Constant {
                        bits: integer.convert(code.bits),
                        ty,
                    });
                }
                _ => return Err(Error::Invariant("constant classification kind")),
            }
        };
        Ok(super::constants::Constant {
            bits: result,
            ty: self.graph.expression_facts.get(id)?.ty,
        })
    }

    fn constant_float_operand(
        &mut self,
        id: ast::ExprId,
        scope: ScopeId,
    ) -> Result<FloatingBits, Error> {
        let facts = self.graph.expression_facts.get(id)?;
        let target = facts
            .conversion
            .map_or(facts.ty, |conversion| conversion.target);
        let Some(super::static_initialization::Value::Floating(kind, bits)) =
            self.static_scalar(id, target, scope)?
        else {
            let id = self.graph.expression_facts.root(id);
            if let ast::ExprKind::Id(name) = self.tree.expressions.get(id).kind
                && let Some(binding) = self.graph.resolved_name(name)
            {
                let binding = self.graph.binding(binding);
                let message = match binding.kind {
                    super::graph::BindingKind::Parameter => {
                        Some("a parameter is not usable in a constant expression")
                    }
                    super::graph::BindingKind::Variable
                        if !self.graph.entity(binding.entity).constexpr =>
                    {
                        Some("a variable is not usable in a constant expression")
                    }
                    _ => None,
                };
                if let Some(message) = message {
                    return Err(Error::NonConstant {
                        location: self.tree.expressions.get(id).location,
                        message,
                    });
                }
            }
            return Err(Error::Missing(
                "floating intrinsic constant operand interpretation",
            ));
        };
        Ok(FloatingBits { kind, bits })
    }

    pub(super) fn floating_intrinsic(
        &mut self,
        id: ast::ExprId,
        arguments: List<ast::ExprId>,
        kind: FloatingIntrinsic,
        scope: ScopeId,
    ) -> Result<ExprFacts, Error> {
        let argument_buffer =
            super::call_arguments::Arguments::new(self.tree.expr_lists.get(arguments));
        let values = argument_buffer.as_slice();
        let count = if kind.comparison() {
            2
        } else if kind == FloatingIntrinsic::Classify {
            6
        } else {
            1
        };
        let location = self.tree.expressions.get(id).location;
        if values.len() != count {
            return Err(Error::at(
                location,
                if kind.comparison() {
                    "wrong number of arguments to a floating comparison builtin"
                } else {
                    "wrong number of arguments to a floating classification builtin"
                },
            ));
        }
        let integer = self.graph.types.scalar(Scalar::Basic(F::Int))?;
        if kind.comparison() {
            let left = self.typed_expression(values[0], scope)?;
            self.check_resolved_function(values[0])?;
            let right = self.typed_expression(values[1], scope)?;
            self.check_resolved_function(values[1])?;
            if !self.is_arithmetic_type(left.ty) || !self.is_arithmetic_type(right.ty) {
                return Err(Error::at(
                    location,
                    "a floating comparison of operands of no floating common type",
                ));
            }
            let common = self.arithmetic_type(left.ty, right.ty)?;
            if !matches!(
                self.graph.types.get(common).kind,
                TypeKind::Scalar(Scalar::Basic(F::Float | F::Double | F::LongDouble))
            ) {
                return Err(Error::at(
                    location,
                    "a floating comparison of operands of no floating common type",
                ));
            }
            self.convert_operand_scalar(values[0], common, false)?;
            self.convert_operand_scalar(values[1], common, false)?;
        } else {
            for (index, &value) in values.iter().enumerate() {
                let facts = self.typed_expression(value, scope)?;
                self.check_resolved_function(value)?;
                if index + 1 < count {
                    self.convert_scalar(value, integer, false)?;
                } else {
                    if !matches!(
                        self.graph.types.get(facts.ty).kind,
                        TypeKind::Scalar(Scalar::Basic(F::Float | F::Double | F::LongDouble))
                    ) {
                        return Err(Error::at(
                            self.tree.expressions.get(value).location,
                            "a floating classification of a non-floating operand",
                        ));
                    }
                    let target = self.bare_type(facts.ty)?;
                    self.convert_operand_scalar(value, target, false)?;
                }
            }
        }
        let mut facts = self.new_expression_facts(integer, ValueCategory::Prvalue)?;
        facts.intrinsic = Some(kind);
        Ok(facts)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum FloatCategory {
    Nan,
    Infinite,
    Normal,
    Subnormal,
    Zero,
}
#[derive(Clone, Copy)]
struct FloatingBits {
    kind: F,
    bits: u128,
}
impl FloatingBits {
    fn fraction(&self) -> u32 {
        match self.kind {
            F::Float => 23,
            F::Double => 52,
            _ => 63,
        }
    }
    fn exponent_width(&self) -> u32 {
        match self.kind {
            F::Float => 8,
            F::Double => 11,
            _ => 15,
        }
    }
    fn sign_shift(&self) -> u32 {
        self.fraction() + self.exponent_width() + u32::from(self.kind == F::LongDouble)
    }
    fn negative(&self) -> bool {
        self.bits & (1 << self.sign_shift()) != 0
    }
    fn quiet(&self) -> bool {
        self.bits & (1 << (self.fraction() - 1)) != 0
    }
    fn category(&self) -> FloatCategory {
        let fraction = self.bits & ((1 << self.fraction()) - 1);
        let exponent = self.bits >> (self.sign_shift() - self.exponent_width())
            & ((1 << self.exponent_width()) - 1);
        if exponent == (1 << self.exponent_width()) - 1 {
            if fraction == 0 {
                FloatCategory::Infinite
            } else {
                FloatCategory::Nan
            }
        } else if exponent != 0 {
            FloatCategory::Normal
        } else if fraction != 0 {
            FloatCategory::Subnormal
        } else {
            FloatCategory::Zero
        }
    }
    fn compare(&self, other: Self) -> Option<std::cmp::Ordering> {
        use std::cmp::Ordering;
        if self.category() == FloatCategory::Nan || other.category() == FloatCategory::Nan {
            return None;
        }
        if self.category() == FloatCategory::Zero && other.category() == FloatCategory::Zero {
            return Some(Ordering::Equal);
        }
        if self.negative() != other.negative() {
            return Some(if self.negative() {
                Ordering::Less
            } else {
                Ordering::Greater
            });
        }
        let order = self.bits.cmp(&other.bits);
        Some(if self.negative() {
            order.reverse()
        } else {
            order
        })
    }
}

// Floating constants and literal payload rules from analyzer_builtins.cpp
// and string_literals.cpp. Operands retain their original syntax and conversions.
#[derive(Clone, Copy)]
pub(crate) enum ConstantKind {
    QuietNan,
    SignalingNan,
    Infinity,
}
fn add_constant_names(
    cursor: &mut Cursor<'_>,
    names: &mut Vec<(NameId, FloatingOperation)>,
) -> Result<(), Error> {
    for (stem, kind) in [
        ("__builtin_nan", ConstantKind::QuietNan),
        ("__builtin_nans", ConstantKind::SignalingNan),
        ("__builtin_inf", ConstantKind::Infinity),
        ("__builtin_huge_val", ConstantKind::Infinity),
    ] {
        for (suffix, ty) in [("", F::Double), ("f", F::Float), ("l", F::LongDouble)] {
            let key = cursor
                .intern_name(format!("{stem}{suffix}").as_bytes())
                .map_err(|_| Error::Limit("semantic identifier limit"))?;
            names.push((key, FloatingOperation::Constant(kind, ty)));
        }
    }
    Ok(())
}

impl Analyzer<'_> {
    pub(super) fn floating_constant(
        &mut self,
        id: ast::ExprId,
        arguments: List<ast::ExprId>,
        kind: ConstantKind,
        fundamental: F,
        scope: ScopeId,
    ) -> Result<ExprFacts, Error> {
        let args = super::call_arguments::Arguments::new(self.tree.expr_lists.get(arguments));
        let args = args.as_slice();
        let count = usize::from(!matches!(kind, ConstantKind::Infinity));
        if args.len() != count {
            return Err(Error::at(
                self.tree.expressions.get(id).location,
                "wrong number of arguments to a floating constant builtin",
            ));
        }
        let mut payload = 0;
        if let Some(&argument) = args.first() {
            self.typed_expression(argument, scope)?;
            self.check_resolved_function(argument)?;
            let parsed = self.nan_payload(argument);
            match (kind, parsed) {
                (ConstantKind::QuietNan, None) => {
                    return self.library_nan_call(id, argument, fundamental, scope);
                }
                (ConstantKind::SignalingNan, None) => {
                    return Err(Error::at(
                        self.tree.expressions.get(argument).location,
                        "__builtin_nans of no string literal spelling a payload",
                    ));
                }
                (_, Some(bits)) => payload = bits,
                (ConstantKind::Infinity, None) => {}
            }
            let character = self.graph.types.scalar(Scalar::Basic(F::Char))?;
            let character = self.graph.types.qualify(character, Cv::CONST)?;
            let text = self.graph.types.pointer(character)?;
            self.convert_scalar(argument, text, false)?;
        }
        let ty = self.graph.types.scalar(Scalar::Basic(fundamental))?;
        let format = crate::types::float_value::Format::of(fundamental)
            .ok_or(Error::Invariant("floating builtin type"))?;
        let value = crate::types::float_value::BinaryFloat::from_bits(
            format,
            constant_bits(kind, fundamental, payload),
        )
        .ok_or(Error::Invariant("floating builtin encoding"))?;
        let value = self
            .graph
            .floating_constants
            .insert(super::constant_floats::Floating { ty, value })?;
        self.graph.expression_facts.set_floating_value(id, value);
        let mut facts = self.new_expression_facts(ty, ValueCategory::Prvalue)?;
        facts.intrinsic = Some(if matches!(kind, ConstantKind::SignalingNan) {
            FloatingIntrinsic::SignalingConstant
        } else {
            FloatingIntrinsic::Constant
        });
        Ok(facts)
    }

    fn nan_payload(&self, mut id: ast::ExprId) -> Option<u64> {
        while let ast::ExprKind::Paren(inner) = self.tree.expressions.get(id).kind {
            id = inner;
        }
        let ast::ExprKind::Literal(literal) = self.tree.expressions.get(id).kind else {
            return None;
        };
        if !matches!(
            literal.kind,
            PostTokenKind::String {
                ty: F::Char,
                suffix: None,
                ..
            }
        ) || literal.suffix_name.is_some()
        {
            return None;
        }
        let (0, text) = self.literal_bytes.get(literal.data).split_last()? else {
            return None;
        };
        parse_payload(text)
    }

    fn library_nan_call(
        &mut self,
        id: ast::ExprId,
        argument: ast::ExprId,
        fundamental: F,
        scope: ScopeId,
    ) -> Result<ExprFacts, Error> {
        let binding = self.library_nan(fundamental)?;
        let facts = self.function_designation(binding, &[binding])?;
        let set = facts
            .functions
            .ok_or(Error::Invariant("library nan candidates"))?;
        let ast::ExprKind::Call { callee, .. } = self.tree.expressions.get(id).kind else {
            return Err(Error::Invariant("library nan without call syntax"));
        };
        let mut named = callee;
        while let ast::ExprKind::Paren(inner) = self.tree.expressions.get(named).kind {
            named = inner;
        }
        self.publish_named_callee(callee, named, facts, scope)?;
        let mut scratch = std::mem::take(&mut self.call_scratch);
        let selected = self.resolve_call(
            self.tree.expressions.get(id).location,
            Candidates::Functions(set),
            &[argument],
            None,
            &mut scratch,
        );
        let result = selected
            .and_then(|selected| self.finish_selected_call(id, facts, scope, selected, &scratch));
        scratch.clear();
        self.call_scratch = scratch;
        result
    }

    fn library_nan(&mut self, fundamental: F) -> Result<BindingId, Error> {
        let index = match fundamental {
            F::Float => 1,
            F::LongDouble => 2,
            _ => 0,
        };
        if let Some(binding) = self.library_nans[index] {
            return Ok(binding);
        }
        let key = self.builtin_names.floating.iter().find_map(|&(key, operation)| {
            matches!(operation, FloatingOperation::Constant(ConstantKind::QuietNan, ty) if ty == fundamental).then_some(key)
        }).ok_or(Error::Invariant("library nan registry identity"))?;
        let result = self.graph.types.scalar(Scalar::Basic(fundamental))?;
        let character = self.graph.types.scalar(Scalar::Basic(F::Char))?;
        let character = self.graph.types.qualify(character, Cv::CONST)?;
        let text = self.graph.types.pointer(character)?;
        let ty = self.graph.types.intern(
            TypeKind::Function {
                result,
                parameters: vec![text],
                variadic: false,
                cv: Cv::default(),
                reference: None,
            },
            Cv::default(),
        )?;
        let entity = self.graph.add_entity(Some(ty), Some(key), Keyword::Int)?;
        self.graph.entity_mut(entity).builtin = Some(BuiltinFunction::LibraryNan(fundamental));
        self.graph.entity_mut(entity).exceptions =
            super::exceptions::ExceptionState::Value(super::exceptions::ExceptionSet::None);
        self.graph.entity_mut(entity).function = Some(super::graph::FunctionProperties {
            language: super::linkage::Language::C,
            no_inline: false,
            internal_linkage: false,
            denotes: None,
            is_static: false,
            special: None,
            allocation: None,
        });
        let binding = self.graph.add_hidden_binding_in(
            self.graph.global,
            Binding {
                kind: BindingKind::Function,
                name: Some(key),
                entity,
                ty: Some(ty),
                key: Keyword::Int,
            },
        )?;
        self.library_nans[index] = Some(binding);
        Ok(binding)
    }
}

// Unlike C string parsing, the oracle checks every decoded unit before the
// terminator, including embedded NULs. A sign is ignored; accumulation wraps.
fn parse_payload(mut text: &[u8]) -> Option<u64> {
    while text
        .first()
        .is_some_and(|&c| c == b' ' || (b'\t'..=b'\r').contains(&c))
    {
        text = &text[1..];
    }
    if matches!(text.first(), Some(b'+' | b'-')) {
        text = &text[1..];
    }
    let mut base = 10;
    if text.first() == Some(&b'0') {
        base = 8;
        text = &text[1..];
        if matches!(text.first(), Some(b'x' | b'X')) {
            base = 16;
            text = &text[1..];
        }
    }
    text.iter().try_fold(0u64, |value, &c| {
        let digit = match c {
            b'0'..=b'9' => u64::from(c - b'0'),
            b'a'..=b'f' => u64::from(c - b'a' + 10),
            b'A'..=b'F' => u64::from(c - b'A' + 10),
            _ => return None,
        };
        (digit < base).then(|| value.wrapping_mul(base).wrapping_add(digit))
    })
}

fn constant_bits(kind: ConstantKind, ty: F, payload: u64) -> u128 {
    let (width, infinity) = match ty {
        F::Float => (22, 0x7f80_0000),
        F::Double => (51, 0x7ff0_0000_0000_0000),
        _ => (62, (0x7fff << 64) | (1 << 63)),
    };
    let fraction = u128::from(payload) & ((1 << width) - 1);
    infinity
        | match kind {
            ConstantKind::Infinity => 0,
            ConstantKind::QuietNan => (1 << width) | fraction,
            ConstantKind::SignalingNan => {
                if fraction == 0 {
                    1 << (width - 1)
                } else {
                    fraction
                }
            }
        }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        preprocess::unit::Preprocessor,
        support::{source::SourceBuffer, stack::StackBudget},
        syntax::SyntaxUnit,
    };

    #[test]
    fn payload_parser_keeps_full_literal_and_target_wrap() {
        for (text, value) in [
            (b"".as_slice(), 0),
            (b" \t\r-0X", 0),
            (b"+077", 63),
            (b"18446744073709551616", 0),
            (b"0xfffffffffffffffff", u64::MAX),
        ] {
            assert_eq!(parse_payload(text), Some(value));
        }
        for text in [b"08".as_slice(), b"0xg", b"1 ", b"1\0", b"- 1"] {
            assert_eq!(parse_payload(text), None);
        }
    }

    #[test]
    fn library_nan_identity_stays_private_and_reuses_parsed_operands() {
        let source = SourceBuffer::new(b"double f(const char*p){__builtin_nan(p);__builtin_nan(\"1\");return __builtin_nan(p);}".to_vec()).unwrap();
        let mut unit = SyntaxUnit::new(Preprocessor::with_name(&source, b"nan-identity").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let expressions = tree.expressions.mark();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let global = analyzer.graph.global;
        analyzer.declarations(declarations, global).unwrap();
        let binding = analyzer.library_nans[0].unwrap();
        assert!(analyzer.library_nans[1..].iter().all(Option::is_none));
        let key = analyzer.graph.binding(binding).name.unwrap();
        let public: Vec<_> = analyzer.graph.functions_at(global, key.into()).collect();
        assert_eq!(public.len(), 1);
        assert_ne!(public[0], binding);
        assert!(matches!(
            analyzer
                .graph
                .entity(analyzer.graph.binding(public[0]).entity)
                .builtin,
            Some(BuiltinFunction::Floating(_))
        ));
        assert!(matches!(
            analyzer
                .graph
                .entity(analyzer.graph.binding(binding).entity)
                .builtin,
            Some(BuiltinFunction::LibraryNan(F::Double))
        ));
        assert_eq!(analyzer.library_nan(F::Double).unwrap(), binding);
        assert_eq!(analyzer.tree.expressions.mark(), expressions);
    }
}

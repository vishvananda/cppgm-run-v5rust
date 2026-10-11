// Translated from preprocess/expr/pp_arithmetic.{h,cpp}. See NOTICE.
//! Explicit x86-64 preprocessing arithmetic, independent of host overflow policy.

const SIGN_BIT: u64 = 1 << 63;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PpValue {
    pub bits: u64,
    pub is_unsigned: bool,
}

impl PpValue {
    #[must_use]
    pub fn boolean(value: bool) -> Self {
        Self {
            bits: u64::from(value),
            is_unsigned: false,
        }
    }

    fn negative(self) -> bool {
        !self.is_unsigned && self.bits & SIGN_BIT != 0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Operator {
    Plus,
    Minus,
    LogicalNot,
    Complement,
    Multiply,
    Divide,
    Modulo,
    Add,
    Subtract,
    ShiftLeft,
    ShiftRight,
    Less,
    Greater,
    LessEqual,
    GreaterEqual,
    Equal,
    NotEqual,
    BitAnd,
    BitXor,
    BitOr,
    LogicalAnd,
    LogicalOr,
}

impl Operator {
    pub(super) fn unary(self) -> bool {
        matches!(
            self,
            Self::Plus | Self::Minus | Self::LogicalNot | Self::Complement
        )
    }

    pub(super) fn precedence(self) -> u8 {
        match self {
            Self::Plus | Self::Minus | Self::LogicalNot | Self::Complement => 11,
            Self::Multiply | Self::Divide | Self::Modulo => 10,
            Self::Add | Self::Subtract => 9,
            Self::ShiftLeft | Self::ShiftRight => 8,
            Self::Less | Self::Greater | Self::LessEqual | Self::GreaterEqual => 7,
            Self::Equal | Self::NotEqual => 6,
            Self::BitAnd => 5,
            Self::BitXor => 4,
            Self::BitOr => 3,
            Self::LogicalAnd => 2,
            Self::LogicalOr => 1,
        }
    }

    pub(super) fn skips_right(self, left: PpValue) -> bool {
        (self == Self::LogicalAnd && left.bits == 0) || (self == Self::LogicalOr && left.bits != 0)
    }
}

// A failed operation still publishes its type and the frozen intermediate
// bits: unevaluated operands participate in later conditional conversions.
#[derive(Clone, Copy)]
pub(super) struct ArithmeticResult {
    pub value: PpValue,
    pub valid: bool,
}

fn magnitude(bits: u64) -> u64 {
    if bits & SIGN_BIT != 0 {
        bits.wrapping_neg()
    } else {
        bits
    }
}

pub(super) fn apply_unary(op: Operator, operand: PpValue) -> ArithmeticResult {
    let mut result = ArithmeticResult {
        value: operand,
        valid: true,
    };
    match op {
        Operator::Plus => {}
        Operator::Minus => {
            result.value.bits = operand.bits.wrapping_neg();
            result.valid = operand.is_unsigned || operand.bits != SIGN_BIT;
        }
        Operator::LogicalNot => result.value = PpValue::boolean(operand.bits == 0),
        Operator::Complement => result.value.bits = !operand.bits,
        _ => result.valid = false,
    }
    result
}

pub(super) fn apply_binary(op: Operator, left: PpValue, right: PpValue) -> ArithmeticResult {
    let l = left.bits;
    let r = right.bits;
    let unsigned = left.is_unsigned || right.is_unsigned;
    let mut result = ArithmeticResult {
        value: PpValue {
            bits: 0,
            is_unsigned: unsigned,
        },
        valid: true,
    };
    let ordered_l = if unsigned { l } else { l ^ SIGN_BIT };
    let ordered_r = if unsigned { r } else { r ^ SIGN_BIT };
    match op {
        Operator::Multiply => {
            result.value.bits = l.wrapping_mul(r);
            let a = magnitude(l);
            let limit = if (l ^ r) & SIGN_BIT != 0 {
                SIGN_BIT
            } else {
                SIGN_BIT - 1
            };
            result.valid = unsigned || a == 0 || magnitude(r) <= limit / a;
        }
        Operator::Divide | Operator::Modulo => {
            result.valid = r != 0 && (unsigned || l != SIGN_BIT || r != u64::MAX);
            if result.valid {
                let a = if unsigned { l } else { magnitude(l) };
                let b = if unsigned { r } else { magnitude(r) };
                let (bits, sign) = if op == Operator::Divide {
                    (a / b, l ^ r)
                } else {
                    (a % b, l)
                };
                result.value.bits = if !unsigned && sign & SIGN_BIT != 0 {
                    bits.wrapping_neg()
                } else {
                    bits
                };
            }
        }
        Operator::Add => {
            result.value.bits = l.wrapping_add(r);
            result.valid =
                unsigned || (l ^ result.value.bits) & (r ^ result.value.bits) & SIGN_BIT == 0;
        }
        Operator::Subtract => {
            result.value.bits = l.wrapping_sub(r);
            result.valid = unsigned || (l ^ r) & (l ^ result.value.bits) & SIGN_BIT == 0;
        }
        Operator::ShiftLeft | Operator::ShiftRight => {
            result.value.is_unsigned = left.is_unsigned;
            result.valid = !right.negative() && r < 64;
            if result.valid {
                result.value.bits = if op == Operator::ShiftLeft {
                    l << r
                } else if left.negative() {
                    !(!l >> r)
                } else {
                    l >> r
                };
            }
        }
        Operator::Less => result.value = PpValue::boolean(ordered_l < ordered_r),
        Operator::Greater => result.value = PpValue::boolean(ordered_l > ordered_r),
        Operator::LessEqual => result.value = PpValue::boolean(ordered_l <= ordered_r),
        Operator::GreaterEqual => result.value = PpValue::boolean(ordered_l >= ordered_r),
        Operator::Equal => result.value = PpValue::boolean(l == r),
        Operator::NotEqual => result.value = PpValue::boolean(l != r),
        Operator::BitAnd => result.value.bits = l & r,
        Operator::BitXor => result.value.bits = l ^ r,
        Operator::BitOr => result.value.bits = l | r,
        Operator::LogicalAnd => result.value = PpValue::boolean(l != 0 && r != 0),
        Operator::LogicalOr => result.value = PpValue::boolean(l != 0 || r != 0),
        _ => result.valid = false,
    }
    result
}

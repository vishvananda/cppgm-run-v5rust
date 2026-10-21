//! Frozen optimized constant division: target-word multipliers and selection. See NOTICE.
use super::{
    division::Form,
    scalar_mir::{Operand, ScalarOp, flip_sign, signed},
    scalar_selection::Selection,
    x86::Register as R,
};
use crate::lowir::{
    Error,
    model::{TempId, Value},
    vocabulary::{Binary, Type, Unary},
};

struct UnsignedMagic {
    multiplier: u64,
    shift: u32,
    add: bool,
}
struct SignedMagic {
    multiplier: i64,
    shift: u32,
}

// Hacker's Delight magicu2, with the frozen 64-bit wrap at every step.
fn unsigned_magic(d: u64) -> UnsignedMagic {
    let two63 = 1_u64 << 63;
    let nc = u64::MAX - d.wrapping_neg() % d;
    let mut p = 63;
    let (mut q1, mut r1) = (two63 / nc, two63 % nc);
    let (mut q2, mut r2) = ((two63 - 1) / d, (two63 - 1) % d);
    let mut add = false;
    loop {
        p += 1;
        if r1 >= nc - r1 {
            q1 = q1.wrapping_mul(2).wrapping_add(1);
            r1 = r1.wrapping_mul(2).wrapping_sub(nc);
        } else {
            q1 = q1.wrapping_mul(2);
            r1 = r1.wrapping_mul(2);
        }
        if r2 + 1 >= d - r2 {
            add |= q2 >= two63 - 1;
            q2 = q2.wrapping_mul(2).wrapping_add(1);
            r2 = r2.wrapping_mul(2).wrapping_add(1).wrapping_sub(d);
        } else {
            add |= q2 >= two63;
            q2 = q2.wrapping_mul(2);
            r2 = r2.wrapping_mul(2).wrapping_add(1);
        }
        let delta = d - 1 - r2;
        if p >= 128 || !(q1 < delta || q1 == delta && r1 == 0) {
            break;
        }
    }
    UnsignedMagic {
        multiplier: q2.wrapping_add(1),
        shift: p - 64,
        add,
    }
}
fn signed_magic(d: i64) -> SignedMagic {
    let two63 = 1_u64 << 63;
    let ad = d.unsigned_abs();
    let t = two63 + (d.cast_unsigned() >> 63);
    let anc = t - 1 - t % ad;
    let mut p = 63;
    let (mut q1, mut r1) = (two63 / anc, two63 % anc);
    let (mut q2, mut r2) = (two63 / ad, two63 % ad);
    loop {
        p += 1;
        q1 = q1.wrapping_mul(2);
        r1 = r1.wrapping_mul(2);
        if r1 >= anc {
            q1 = q1.wrapping_add(1);
            r1 -= anc;
        }
        q2 = q2.wrapping_mul(2);
        r2 = r2.wrapping_mul(2);
        if r2 >= ad {
            q2 = q2.wrapping_add(1);
            r2 -= ad;
        }
        let delta = ad - r2;
        if !(q1 < delta || q1 == delta && r1 == 0) {
            break;
        }
    }
    let multiplier = q2.wrapping_add(1);
    SignedMagic {
        multiplier: if d < 0 {
            multiplier.wrapping_neg()
        } else {
            multiplier
        }
        .cast_signed(),
        shift: p - 64,
    }
}
fn immediate(v: i64) -> Operand {
    Operand::Immediate(i128::from(v).cast_unsigned())
}
fn register(r: R) -> Operand {
    Operand::Register(r)
}
impl Selection<'_> {
    fn division_binary(&mut self, op: Binary, r: R, source: Operand) {
        self.emit(ScalarOp::Binary(op, Type::I64, r, source));
    }
    fn division_shift(&mut self, op: Binary, r: R, count: u32) {
        self.emit(ScalarOp::Shift(op, r, immediate(i64::from(count))));
    }
    fn division_negate(&mut self, r: R) {
        self.emit(ScalarOp::Unary(Unary::Neg, Type::I64, r));
    }
    pub(super) fn constant_divide(
        &mut self,
        temp: TempId,
        op: Binary,
        ty: Type,
        left: Value,
        form: Form,
    ) -> Result<(), Error> {
        let unsigned = matches!(op, Binary::UDiv | Binary::UMod);
        let remainder = matches!(op, Binary::Mod | Binary::UMod);
        let convert = ty != Type::I64 && ty != Type::Ptr && unsigned == signed(ty);
        let a = self.operand(left, ty, R::R10)?;
        let x = if let Some(r) = a.register()
            && !convert
        {
            r
        } else {
            self.load(R::R10, a, ty);
            if convert {
                self.normalize(R::R10, flip_sign(ty));
            }
            R::R10
        };
        let d = match form {
            Form::SignedShift(divisor) | Form::Exact(divisor) => {
                let d = self.result(
                    temp,
                    [self.dying(a, false), None],
                    if x == R::R10 { 1 << R::R10.code() } else { 0 },
                );
                if matches!(form, Form::SignedShift(_)) {
                    self.signed_shift_divide(x, d, divisor, remainder);
                } else {
                    self.exact_divide(x, d, divisor);
                }
                d
            }
            Form::UnsignedMagic(divisor) | Form::SignedMagic(divisor) => {
                let from = if matches!(form, Form::UnsignedMagic(_)) {
                    self.unsigned_magic_divide(x, divisor, remainder)
                } else {
                    self.signed_magic_divide(x, divisor, remainder)
                };
                let d = self.result(temp, [Some(from), None], 0);
                self.load(d, register(from), Type::I64);
                d
            }
            _ => return Err(Error::Construction("nonconstant divide form".into())),
        };
        if convert {
            self.normalize(d, ty);
        }
        self.finish(temp, d)
    }
    fn signed_shift_divide(&mut self, x: R, d: R, divisor: i64, remainder: bool) {
        let magnitude = divisor.unsigned_abs();
        let k = magnitude.trailing_zeros();
        let t = if d != x {
            d
        } else if x == R::R11 {
            R::R10
        } else {
            R::R11
        };
        self.emit(ScalarOp::Move(t, register(x)));
        if k == 1 {
            self.division_shift(Binary::UShr, t, 63);
        } else {
            self.division_shift(Binary::Shr, t, 63);
            self.division_shift(Binary::UShr, t, 64 - k);
        }
        if !remainder {
            self.division_binary(Binary::Add, d, register(if t == d { x } else { t }));
            self.division_shift(Binary::Shr, d, k);
            if divisor < 0 {
                self.division_negate(d);
            }
            return;
        }
        self.division_binary(Binary::Add, t, register(x));
        if k <= 31 {
            self.division_binary(
                Binary::And,
                t,
                immediate(magnitude.wrapping_neg().cast_signed()),
            );
        } else {
            self.division_shift(Binary::Shr, t, k);
            self.division_shift(Binary::Shl, t, k);
        }
        if t == d {
            self.division_negate(d);
            self.division_binary(Binary::Add, d, register(x));
        } else {
            self.division_binary(Binary::Sub, d, register(t));
        }
    }
    fn exact_divide(&mut self, x: R, d: R, divisor: i64) {
        let magnitude = divisor.unsigned_abs();
        let k = magnitude.trailing_zeros();
        let odd = magnitude >> k;
        self.load(d, register(x), Type::I64);
        if k != 0 {
            self.division_shift(Binary::Shr, d, k);
        }
        if odd != 1 {
            let mut inverse = odd;
            for _ in 0..5 {
                inverse = inverse.wrapping_mul(2_u64.wrapping_sub(odd.wrapping_mul(inverse)));
            }
            let inverse = inverse.cast_signed();
            let source = if i32::try_from(inverse).is_ok() {
                immediate(inverse)
            } else {
                let t = if d == R::R11 { R::R10 } else { R::R11 };
                self.load(t, immediate(inverse), Type::I64);
                register(t)
            };
            self.division_binary(Binary::Mul, d, source);
        }
        if divisor < 0 {
            self.division_negate(d);
        }
    }
    fn magic_remainder(&mut self, x: R, q: R, divisor: i64) -> R {
        let source = if i32::try_from(divisor).is_ok() {
            immediate(divisor)
        } else {
            self.load(R::R11, immediate(divisor), Type::I64);
            register(R::R11)
        };
        self.division_binary(Binary::Mul, q, source);
        let r = if q == R::RDX { R::RAX } else { R::RDX };
        self.emit(ScalarOp::Move(r, register(x)));
        self.division_binary(Binary::Sub, r, register(q));
        r
    }
    fn unsigned_magic_divide(&mut self, x: R, divisor: i64, remainder: bool) -> R {
        let magic = unsigned_magic(divisor.cast_unsigned());
        self.emit(ScalarOp::Move(R::RAX, register(x)));
        self.load(R::R11, immediate(magic.multiplier.cast_signed()), Type::I64);
        self.emit(ScalarOp::MultiplyWide {
            unsigned: true,
            source: R::R11,
        });
        let mut q = R::RDX;
        if magic.add {
            self.emit(ScalarOp::Move(R::RAX, register(x)));
            self.division_binary(Binary::Sub, R::RAX, register(R::RDX));
            self.division_shift(Binary::UShr, R::RAX, 1);
            self.division_binary(Binary::Add, R::RAX, register(R::RDX));
            if magic.shift > 1 {
                self.division_shift(Binary::UShr, R::RAX, magic.shift - 1);
            }
            q = R::RAX;
        } else if magic.shift != 0 {
            self.division_shift(Binary::UShr, R::RDX, magic.shift);
        }
        if remainder {
            self.magic_remainder(x, q, divisor)
        } else {
            q
        }
    }
    fn signed_magic_divide(&mut self, x: R, divisor: i64, remainder: bool) -> R {
        let magic = signed_magic(divisor);
        self.emit(ScalarOp::Move(R::RAX, register(x)));
        self.load(R::R11, immediate(magic.multiplier), Type::I64);
        self.emit(ScalarOp::MultiplyWide {
            unsigned: false,
            source: R::R11,
        });
        if divisor > 0 && magic.multiplier < 0 {
            self.division_binary(Binary::Add, R::RDX, register(x));
        } else if divisor < 0 && magic.multiplier > 0 {
            self.division_binary(Binary::Sub, R::RDX, register(x));
        }
        if magic.shift != 0 {
            self.division_shift(Binary::Shr, R::RDX, magic.shift);
        }
        self.emit(ScalarOp::Move(R::RAX, register(R::RDX)));
        self.division_shift(Binary::UShr, R::RAX, 63);
        self.division_binary(Binary::Add, R::RDX, register(R::RAX));
        if remainder {
            self.magic_remainder(x, R::RDX, divisor)
        } else {
            R::RDX
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn magic_quotients_match_target_division_at_boundaries() {
        let divisors = [
            3_u64,
            5,
            7,
            31,
            255,
            65537,
            (1_u64 << 32) - 1,
            (1_u64 << 63) - 1,
            (1_u64 << 63) + 1,
            u64::MAX,
        ];
        let mut samples = vec![0, 1, 2, 7, u64::MAX, u64::MAX - 1, 1_u64 << 63];
        let mut state = 0x1234_5678_9abc_def0_u64;
        for _ in 0..1024 {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1);
            samples.push(state);
        }
        for divisor in divisors {
            let magic = unsigned_magic(divisor);
            for &x in &samples {
                let high =
                    u64::try_from((u128::from(x) * u128::from(magic.multiplier)) >> 64).unwrap();
                let quotient = if magic.add {
                    ((x - high) / 2 + high) >> (magic.shift - 1)
                } else {
                    high >> magic.shift
                };
                assert_eq!(quotient, x / divisor, "unsigned {x} / {divisor}");
            }
            if divisor >= 1_u64 << 63 {
                continue;
            }
            for divisor in [divisor.cast_signed(), -divisor.cast_signed()] {
                let magic = signed_magic(divisor);
                for &bits in &samples {
                    let x = bits.cast_signed();
                    let mut high =
                        i64::try_from((i128::from(x) * i128::from(magic.multiplier)) >> 64)
                            .unwrap();
                    if divisor > 0 && magic.multiplier < 0 {
                        high = high.wrapping_add(x);
                    } else if divisor < 0 && magic.multiplier > 0 {
                        high = high.wrapping_sub(x);
                    }
                    high >>= magic.shift;
                    let quotient = high.wrapping_add((high.cast_unsigned() >> 63).cast_signed());
                    assert_eq!(quotient, x / divisor, "signed {x} / {divisor}");
                }
            }
        }
    }
}

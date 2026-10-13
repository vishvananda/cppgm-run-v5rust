// Adapted from lex/interchange_float.cpp. See NOTICE.
//! Exact decimal-to-target rounding with bounded, reusable integer scratch.
use crate::types::FundamentalType;
use std::cmp::Ordering;

#[derive(Clone, Copy, Debug)]
pub enum FloatError {
    HostSize,
    NonFloatingType,
}

/// Initialized target bytes; x87's six padding bytes remain zero.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScalarBytes {
    bytes: [u8; 16],
    size: usize,
}

impl ScalarBytes {
    pub(crate) fn integer(ty: FundamentalType, value: u64) -> Self {
        let mut bytes = [0; 16];
        bytes[..8].copy_from_slice(&value.to_le_bytes());
        Self {
            bytes,
            size: ty.size(),
        }
    }

    #[must_use]
    pub fn as_slice(&self) -> &[u8] {
        &self.bytes[..self.size]
    }
}

#[derive(Default)]
struct Big {
    words: Vec<u32>,
}

impl Big {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "Extract each low base-2^32 limb before shifting the magnitude"
    )]
    fn set_binary(&mut self, mut value: u128) {
        self.words.clear();
        while value != 0 {
            self.words.push(value as u32);
            value >>= 32;
        }
    }

    #[expect(
        clippy::cast_possible_truncation,
        reason = "Base-2^32 division with remainder below the divisor leaves a quotient fitting one limb"
    )]
    fn decimal(&mut self, chunks: &mut Vec<u32>) -> Result<String, FloatError> {
        chunks.clear();
        while !self.words.is_empty() {
            let mut remainder = 0_u64;
            for word in self.words.iter_mut().rev() {
                let current = (remainder << 32) | u64::from(*word);
                *word = (current / 1_000_000_000) as u32;
                remainder = current % 1_000_000_000;
            }
            chunks.push(u32::try_from(remainder).map_err(|_| FloatError::HostSize)?);
            self.trim();
        }
        let mut out = chunks.pop().unwrap_or(0).to_string();
        while let Some(chunk) = chunks.pop() {
            use std::fmt::Write;
            write!(out, "{chunk:09}").map_err(|_| FloatError::HostSize)?;
        }
        Ok(out)
    }
    fn trim(&mut self) {
        while self.words.last() == Some(&0) {
            self.words.pop();
        }
    }

    fn copy_from(&mut self, other: &Self) {
        self.words.clone_from(&other.words);
    }

    #[expect(
        clippy::cast_possible_truncation,
        reason = "Split each base-2^32 product deliberately into low word and carry"
    )]
    fn multiply_add(&mut self, factor: u32, addend: u32) {
        let mut carry = u64::from(addend);
        for word in &mut self.words {
            let product = u64::from(*word) * u64::from(factor) + carry;
            *word = product as u32;
            carry = product >> 32;
        }
        if carry != 0 {
            self.words.push(carry as u32);
        }
    }

    fn power_of_ten(&mut self, mut count: i64) {
        while count >= 9 {
            self.multiply_add(1_000_000_000, 0);
            count -= 9;
        }
        let mut factor = 1;
        while count > 0 {
            factor *= 10;
            count -= 1;
        }
        self.multiply_add(factor, 0);
    }

    fn shift_left(&mut self, bits: i64) -> Result<(), FloatError> {
        if self.words.is_empty() || bits == 0 {
            return Ok(());
        }
        let bits = u32::try_from(bits).map_err(|_| FloatError::HostSize)?;
        let rest = bits % 32;
        if rest != 0 {
            let mut carry = 0;
            for word in &mut self.words {
                let next = *word >> (32 - rest);
                *word = (*word << rest) | carry;
                carry = next;
            }
            if carry != 0 {
                self.words.push(carry);
            }
        }
        let extra = usize::try_from(bits / 32).map_err(|_| FloatError::HostSize)?;
        let old_len = self.words.len();
        let new_len = old_len.checked_add(extra).ok_or(FloatError::HostSize)?;
        self.words.resize(new_len, 0);
        self.words.copy_within(..old_len, extra);
        self.words[..extra].fill(0);
        Ok(())
    }

    fn shift_right_one(&mut self) {
        for i in 0..self.words.len() {
            self.words[i] = (self.words[i] >> 1) | self.words.get(i + 1).map_or(0, |w| w << 31);
        }
        self.trim();
    }

    fn bit_length(&self) -> Result<i64, FloatError> {
        let Some(&top) = self.words.last() else {
            return Ok(0);
        };
        let lower = i64::try_from(self.words.len() - 1).map_err(|_| FloatError::HostSize)?;
        Ok(lower * 32 + i64::from(32 - top.leading_zeros()))
    }

    fn compare(&self, other: &Self) -> Ordering {
        self.words
            .len()
            .cmp(&other.words.len())
            .then_with(|| self.words.iter().rev().cmp(other.words.iter().rev()))
    }

    #[expect(
        clippy::cast_possible_truncation,
        reason = "Subtraction stores a base-2^32 difference and explicitly propagates the borrow"
    )]
    fn subtract(&mut self, other: &Self) {
        let mut borrow = 0u64;
        for (i, word) in self.words.iter_mut().enumerate() {
            let subtrahend = u64::from(other.words.get(i).copied().unwrap_or(0)) + borrow;
            borrow = u64::from(u64::from(*word) < subtrahend);
            *word = (u64::from(*word) + (borrow << 32) - subtrahend) as u32;
        }
        self.trim();
    }
}

#[derive(Clone, Copy)]
struct Format {
    precision: u32,
    max_exponent: i64,
    explicit_integer_bit: bool,
    size: usize,
}

impl Format {
    fn for_type(ty: FundamentalType) -> Result<Self, FloatError> {
        let (precision, max_exponent, explicit_integer_bit) = match ty {
            FundamentalType::Float => (24, 127, false),
            FundamentalType::Double => (53, 1023, false),
            FundamentalType::LongDouble => (64, 16_383, true),
            _ => return Err(FloatError::NonFloatingType),
        };
        Ok(Self {
            precision,
            max_exponent,
            explicit_integer_bit,
            size: ty.size(),
        })
    }
}

/// Scratch belongs to the tokenizer, retaining only capacity between literals.
#[derive(Default)]
pub struct FloatingScanner {
    num: Big,
    den: Big,
    scaled: Big,
    remainder: Big,
    divisor: Big,
    shifted_divisor: Big,
    decimal_chunks: Vec<u32>,
}

impl FloatingScanner {
    /// Round an exact binary rational to a target format, retaining capacity.
    pub(crate) fn binary(
        &mut self,
        significand: u128,
        exponent: i64,
        ty: FundamentalType,
    ) -> Result<Option<u128>, FloatError> {
        let format = Format::for_type(ty)?;
        if significand == 0 {
            return Ok(Some(0));
        }
        self.num.set_binary(significand);
        self.den.set_binary(1);
        if exponent >= 0 {
            self.num.shift_left(exponent)?;
        } else {
            self.den.shift_left(-exponent)?;
        }
        let result = self.round(format)?.map(|(s, e)| encode(format, s, e));
        self.clear();
        Ok(result)
    }

    /// Hexadecimal mantissa scanning for explicit `LowIR` input. The grammar
    /// owner checks syntax before this bounded rational conversion.
    pub(crate) fn hexadecimal(
        &mut self,
        digits: &[u8],
        exponent: i64,
        ty: FundamentalType,
    ) -> Result<Option<u128>, FloatError> {
        let _length = u32::try_from(digits.len()).map_err(|_| FloatError::HostSize)?;
        let format = Format::for_type(ty)?;
        self.num.words.clear();
        let mut point = false;
        let mut scale = exponent;
        let mut significant = 0;
        let mut sticky = false;
        for &c in digits {
            if c == b'.' {
                point = true;
                continue;
            }
            let digit = match c {
                b'0'..=b'9' => u32::from(c - b'0'),
                b'a'..=b'f' => u32::from(c - b'a' + 10),
                _ => u32::from(c - b'A' + 10),
            };
            if significant >= 32 {
                sticky |= digit != 0;
                if !point {
                    scale += 4;
                }
            } else {
                if point {
                    scale -= 4;
                }
                self.num.multiply_add(16, digit);
                if digit != 0 || significant != 0 {
                    significant += 1;
                }
            }
        }
        if sticky {
            self.num.multiply_add(16, 1);
            scale -= 4;
        }
        let top = self.num.bit_length()? + scale;
        let result = if self.num.words.is_empty()
            || top < 1 - format.max_exponent - i64::from(format.precision)
        {
            Some(0)
        } else if top > format.max_exponent + 1 {
            None
        } else {
            self.den.set_binary(1);
            if scale >= 0 {
                self.num.shift_left(scale)?;
            } else {
                self.den.shift_left(-scale)?;
            }
            self.round(format)?.map(|(s, e)| encode(format, s, e))
        };
        self.clear();
        Ok(result)
    }

    /// Exact terminating decimal digits and decimal scale of a binary value.
    pub(crate) fn decimal_binary(
        &mut self,
        significand: u128,
        exponent: i64,
    ) -> Result<(String, i64), FloatError> {
        self.num.set_binary(significand);
        let scale = if exponent >= 0 {
            self.num.shift_left(exponent)?;
            0
        } else {
            let mut count = -exponent;
            while count >= 13 {
                self.num.multiply_add(1_220_703_125, 0);
                count -= 13;
            }
            while count > 0 {
                self.num.multiply_add(5, 0);
                count -= 1;
            }
            exponent
        };
        let digits = self.num.decimal(&mut self.decimal_chunks)?;
        self.clear();
        Ok((digits, scale))
    }

    fn clear(&mut self) {
        self.num.words.clear();
        self.den.words.clear();
        self.scaled.words.clear();
        self.remainder.words.clear();
        self.divisor.words.clear();
        self.shifted_divisor.words.clear();
        self.decimal_chunks.clear();
    }
    /// Decode an already classified decimal spelling, without its suffix.
    ///
    /// # Errors
    /// Returns a typed error on host-size overflow or a nonfloating type.
    /// `None` is target overflow to infinity, which makes the literal invalid.
    pub fn scan(
        &mut self,
        text: &[u8],
        ty: FundamentalType,
    ) -> Result<Option<ScalarBytes>, FloatError> {
        let result = self.scan_value(text, ty);
        self.clear();
        result
    }

    fn scan_value(
        &mut self,
        text: &[u8],
        ty: FundamentalType,
    ) -> Result<Option<ScalarBytes>, FloatError> {
        let format = Format::for_type(ty)?;
        let (digit_count, exponent) = self.decimal_digits(text, format)?;
        let mut bytes = ScalarBytes {
            bytes: [0; 16],
            size: format.size,
        };
        if self.num.words.is_empty() {
            return Ok(Some(bytes));
        }
        let p = i64::from(format.precision);
        let emax = format.max_exponent;
        let emin = 1 - emax;
        let m = digit_count + exponent;
        // Frozen scanner's conservative log2(10) bounds limit all limb work.
        if (m - 1) * 33_219 >= (emax + 1) * 10_000 {
            return Ok(None);
        }
        if m * 33_220 < (emin - p) * 10_000 {
            return Ok(Some(bytes));
        }
        self.den.words.clear();
        self.den.words.push(1);
        if exponent >= 0 {
            self.num.power_of_ten(exponent);
        } else {
            self.den.power_of_ten(-exponent);
        }
        let Some((significand, biased)) = self.round(format)? else {
            return Ok(None);
        };
        let encoding = encode(format, significand, biased);
        bytes.bytes.copy_from_slice(&encoding.to_le_bytes());
        Ok(Some(bytes))
    }

    fn decimal_digits(&mut self, text: &[u8], format: Format) -> Result<(i64, i64), FloatError> {
        // Source lengths are compact, but check the standalone API too.
        let _length = u32::try_from(text.len()).map_err(|_| FloatError::HostSize)?;
        let p = i64::from(format.precision);
        let kept = (p * 30_103 + (format.max_exponent - 1 + p) * 69_897) / 100_000 + 3;
        self.num.words.clear();
        let mut exponent = 0;
        let mut significant = 0;
        let mut point = false;
        let mut sticky = false;
        let mut chunk = 0;
        let mut chunk_digits = 0;
        let mut chunk_scale = 1;
        let mut pos = 0;
        while pos < text.len() {
            let c = text[pos];
            if c == b'.' {
                point = true;
                pos += 1;
                continue;
            }
            if !c.is_ascii_digit() {
                break;
            }
            pos += 1;
            let value = u32::from(c - b'0');
            if significant >= kept {
                sticky |= value != 0;
                if !point {
                    exponent += 1;
                }
                continue;
            }
            if point {
                exponent -= 1;
            }
            if significant == 0 && value == 0 {
                continue;
            }
            chunk = chunk * 10 + value;
            chunk_scale *= 10;
            significant += 1;
            chunk_digits += 1;
            if chunk_digits == 9 {
                self.num.multiply_add(chunk_scale, chunk);
                chunk = 0;
                chunk_digits = 0;
                chunk_scale = 1;
            }
        }
        if chunk_digits != 0 {
            self.num.multiply_add(chunk_scale, chunk);
        }
        if sticky {
            self.num.multiply_add(10, 1);
            exponent -= 1;
            significant += 1;
        }
        if matches!(text.get(pos), Some(b'e' | b'E')) {
            pos += 1;
            let negative = text.get(pos) == Some(&b'-');
            if matches!(text.get(pos), Some(b'+' | b'-')) {
                pos += 1;
            }
            let mut written = 0;
            for &c in &text[pos..] {
                if !c.is_ascii_digit() {
                    break;
                }
                // Match the frozen exponent saturation exactly (it may cross
                // 10^9 on the last update, then stops accumulating).
                if written < 1_000_000_000 {
                    written = written * 10 + i64::from(c - b'0');
                }
            }
            exponent += if negative { -written } else { written };
        }
        Ok((significant, exponent))
    }

    fn round(&mut self, format: Format) -> Result<Option<(u128, u16)>, FloatError> {
        let p = i64::from(format.precision);
        let emax = format.max_exponent;
        let emin = 1 - emax;
        let t = self.num.bit_length()? - self.den.bit_length()?;
        let at_least = if t >= 0 {
            self.scaled.copy_from(&self.den);
            self.scaled.shift_left(t)?;
            self.num.compare(&self.scaled) != Ordering::Less
        } else {
            self.scaled.copy_from(&self.num);
            self.scaled.shift_left(-t)?;
            self.scaled.compare(&self.den) != Ordering::Less
        };
        let e = if at_least { t } else { t - 1 };
        let mut q = e.max(emin) - (p - 1);
        self.remainder.copy_from(&self.num);
        self.divisor.copy_from(&self.den);
        if q <= 0 {
            self.remainder.shift_left(-q)?;
        } else {
            self.divisor.shift_left(q)?;
        }
        self.shifted_divisor.copy_from(&self.divisor);
        self.shifted_divisor.shift_left(p)?;
        let mut significand = 0u128;
        for bit in (0..=format.precision).rev() {
            if self.remainder.compare(&self.shifted_divisor) != Ordering::Less {
                self.remainder.subtract(&self.shifted_divisor);
                significand |= 1u128 << bit;
            }
            self.shifted_divisor.shift_right_one();
        }
        self.remainder.shift_left(1)?;
        let half = self.remainder.compare(&self.divisor);
        if half == Ordering::Greater || (half == Ordering::Equal && significand & 1 != 0) {
            significand += 1;
        }
        if significand & (1u128 << format.precision) != 0 {
            significand >>= 1;
            q += 1;
        }
        let normal = significand & (1u128 << (format.precision - 1)) != 0;
        let biased = if normal { q + (p - 1) + emax } else { 0 };
        if biased > 2 * emax {
            return Ok(None);
        }
        Ok(Some((
            significand,
            u16::try_from(biased).map_err(|_| FloatError::HostSize)?,
        )))
    }
}

fn encode(format: Format, significand: u128, biased: u16) -> u128 {
    if format.explicit_integer_bit {
        significand | (u128::from(biased) << 64)
    } else {
        let fraction = format.precision - 1;
        (significand & ((1_u128 << fraction) - 1)) | (u128::from(biased) << fraction)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn direct_rounding_keeps_halfway_and_subnormal_boundaries() {
        let mut scanner = FloatingScanner::default();
        for (text, bits) in [
            ("1.000000059604644775390625", 0x3f80_0000u32),
            ("1.0000000596046447753906250000001", 0x3f80_0001),
            ("1.000000178813934326171875", 0x3f80_0002),
            (
                "7.00649232162408535461864791644958065640130970938257885878534141944895541342930300743319094181060791015625e-46",
                0,
            ),
            (
                "1.40129846432481707092372958328991613128026194187651577175706828388979108268586060148663818836212158203125e-45",
                1,
            ),
        ] {
            assert_eq!(
                scanner
                    .scan(text.as_bytes(), FundamentalType::Float)
                    .unwrap()
                    .unwrap()
                    .as_slice(),
                bits.to_le_bytes()
            );
        }
        assert!(
            scanner
                .scan(b"1e99999999999999", FundamentalType::Float)
                .unwrap()
                .is_none()
        );
        assert_eq!(
            scanner
                .scan(b"0e99999999999999", FundamentalType::Float)
                .unwrap()
                .unwrap()
                .as_slice(),
            [0; 4]
        );
    }

    #[test]
    fn x87_retains_all_64_significand_bits_and_zero_padding() {
        let mut scanner = FloatingScanner::default();
        let value = scanner
            .scan(
                b"1.000000000000000000108420217248550443400745280086994171142578125",
                FundamentalType::LongDouble,
            )
            .unwrap()
            .unwrap();
        assert_eq!(
            value.as_slice(),
            [1, 0, 0, 0, 0, 0, 0, 128, 255, 63, 0, 0, 0, 0, 0, 0]
        );
    }
}

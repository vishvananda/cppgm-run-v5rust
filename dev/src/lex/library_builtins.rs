// Translated from lex/gnu_builtins.cpp's fixed library and libm registries. See NOTICE.
//! Immutable signature vocabulary shared by hosted probes and semantic declaration.
use crate::types::FundamentalType as F;

#[derive(Clone, Copy)]
pub enum Type {
    Void,
    Scalar(F),
    Floating,
    Complex,
    VaList,
}
#[derive(Clone, Copy)]
pub enum Parameter {
    Value(Type),
    Pointer(Type),
    ConstPointer(Type),
}
#[derive(Clone, Copy)]
pub struct Signature {
    pub result: Parameter,
    pub parameters: &'static [Parameter],
}
#[derive(Clone, Copy)]
pub struct LibraryBuiltin {
    pub signature: Signature,
    pub floating: F,
}
use Parameter::{ConstPointer, Pointer, Value};
use Type::{Complex, Floating, Scalar, VaList, Void};

const FIXED: &[(&[u8], Signature)] = &[
    (
        b"abs",
        Signature {
            result: Value(Scalar(F::Int)),
            parameters: &[Value(Scalar(F::Int))],
        },
    ),
    (
        b"bzero",
        Signature {
            result: Value(Void),
            parameters: &[Pointer(Void), Value(Scalar(F::UnsignedLongInt))],
        },
    ),
    (
        b"free",
        Signature {
            result: Value(Void),
            parameters: &[Pointer(Void)],
        },
    ),
    (
        b"index",
        Signature {
            result: Pointer(Scalar(F::Char)),
            parameters: &[ConstPointer(Scalar(F::Char)), Value(Scalar(F::Int))],
        },
    ),
    (
        b"labs",
        Signature {
            result: Value(Scalar(F::LongInt)),
            parameters: &[Value(Scalar(F::LongInt))],
        },
    ),
    (
        b"llabs",
        Signature {
            result: Value(Scalar(F::LongLongInt)),
            parameters: &[Value(Scalar(F::LongLongInt))],
        },
    ),
    (
        b"malloc",
        Signature {
            result: Pointer(Void),
            parameters: &[Value(Scalar(F::UnsignedLongInt))],
        },
    ),
    (
        b"memchr",
        Signature {
            result: Pointer(Void),
            parameters: &[
                ConstPointer(Void),
                Value(Scalar(F::Int)),
                Value(Scalar(F::UnsignedLongInt)),
            ],
        },
    ),
    (
        b"memcmp",
        Signature {
            result: Value(Scalar(F::Int)),
            parameters: &[
                ConstPointer(Void),
                ConstPointer(Void),
                Value(Scalar(F::UnsignedLongInt)),
            ],
        },
    ),
    (
        b"memset",
        Signature {
            result: Pointer(Void),
            parameters: &[
                Pointer(Void),
                Value(Scalar(F::Int)),
                Value(Scalar(F::UnsignedLongInt)),
            ],
        },
    ),
    (
        b"rindex",
        Signature {
            result: Pointer(Scalar(F::Char)),
            parameters: &[ConstPointer(Scalar(F::Char)), Value(Scalar(F::Int))],
        },
    ),
    (
        b"strchr",
        Signature {
            result: Pointer(Scalar(F::Char)),
            parameters: &[ConstPointer(Scalar(F::Char)), Value(Scalar(F::Int))],
        },
    ),
    (
        b"strcmp",
        Signature {
            result: Value(Scalar(F::Int)),
            parameters: &[ConstPointer(Scalar(F::Char)), ConstPointer(Scalar(F::Char))],
        },
    ),
    (
        b"strcpy",
        Signature {
            result: Pointer(Scalar(F::Char)),
            parameters: &[Pointer(Scalar(F::Char)), ConstPointer(Scalar(F::Char))],
        },
    ),
    (
        b"strncmp",
        Signature {
            result: Value(Scalar(F::Int)),
            parameters: &[
                ConstPointer(Scalar(F::Char)),
                ConstPointer(Scalar(F::Char)),
                Value(Scalar(F::UnsignedLongInt)),
            ],
        },
    ),
    (
        b"strncpy",
        Signature {
            result: Pointer(Scalar(F::Char)),
            parameters: &[
                Pointer(Scalar(F::Char)),
                ConstPointer(Scalar(F::Char)),
                Value(Scalar(F::UnsignedLongInt)),
            ],
        },
    ),
    (
        b"strpbrk",
        Signature {
            result: Pointer(Scalar(F::Char)),
            parameters: &[ConstPointer(Scalar(F::Char)), ConstPointer(Scalar(F::Char))],
        },
    ),
    (
        b"strrchr",
        Signature {
            result: Pointer(Scalar(F::Char)),
            parameters: &[ConstPointer(Scalar(F::Char)), Value(Scalar(F::Int))],
        },
    ),
    (
        b"strstr",
        Signature {
            result: Pointer(Scalar(F::Char)),
            parameters: &[ConstPointer(Scalar(F::Char)), ConstPointer(Scalar(F::Char))],
        },
    ),
    (
        b"vsnprintf",
        Signature {
            result: Value(Scalar(F::Int)),
            parameters: &[
                Pointer(Scalar(F::Char)),
                Value(Scalar(F::UnsignedLongInt)),
                ConstPointer(Scalar(F::Char)),
                Value(VaList),
            ],
        },
    ),
];
const LIBM: &[(&[u8], Signature)] = &[
    (
        b"acos",
        Signature {
            result: Value(Floating),
            parameters: &[Value(Floating)],
        },
    ),
    (
        b"acosh",
        Signature {
            result: Value(Floating),
            parameters: &[Value(Floating)],
        },
    ),
    (
        b"asin",
        Signature {
            result: Value(Floating),
            parameters: &[Value(Floating)],
        },
    ),
    (
        b"asinh",
        Signature {
            result: Value(Floating),
            parameters: &[Value(Floating)],
        },
    ),
    (
        b"atan",
        Signature {
            result: Value(Floating),
            parameters: &[Value(Floating)],
        },
    ),
    (
        b"atan2",
        Signature {
            result: Value(Floating),
            parameters: &[Value(Floating), Value(Floating)],
        },
    ),
    (
        b"atanh",
        Signature {
            result: Value(Floating),
            parameters: &[Value(Floating)],
        },
    ),
    (
        b"cabs",
        Signature {
            result: Value(Floating),
            parameters: &[Value(Complex)],
        },
    ),
    (
        b"cacos",
        Signature {
            result: Value(Complex),
            parameters: &[Value(Complex)],
        },
    ),
    (
        b"cacosh",
        Signature {
            result: Value(Complex),
            parameters: &[Value(Complex)],
        },
    ),
    (
        b"carg",
        Signature {
            result: Value(Floating),
            parameters: &[Value(Complex)],
        },
    ),
    (
        b"casin",
        Signature {
            result: Value(Complex),
            parameters: &[Value(Complex)],
        },
    ),
    (
        b"casinh",
        Signature {
            result: Value(Complex),
            parameters: &[Value(Complex)],
        },
    ),
    (
        b"catan",
        Signature {
            result: Value(Complex),
            parameters: &[Value(Complex)],
        },
    ),
    (
        b"catanh",
        Signature {
            result: Value(Complex),
            parameters: &[Value(Complex)],
        },
    ),
    (
        b"cbrt",
        Signature {
            result: Value(Floating),
            parameters: &[Value(Floating)],
        },
    ),
    (
        b"ccos",
        Signature {
            result: Value(Complex),
            parameters: &[Value(Complex)],
        },
    ),
    (
        b"ccosh",
        Signature {
            result: Value(Complex),
            parameters: &[Value(Complex)],
        },
    ),
    (
        b"ceil",
        Signature {
            result: Value(Floating),
            parameters: &[Value(Floating)],
        },
    ),
    (
        b"cexp",
        Signature {
            result: Value(Complex),
            parameters: &[Value(Complex)],
        },
    ),
    (
        b"cimag",
        Signature {
            result: Value(Floating),
            parameters: &[Value(Complex)],
        },
    ),
    (
        b"clog",
        Signature {
            result: Value(Complex),
            parameters: &[Value(Complex)],
        },
    ),
    (
        b"conj",
        Signature {
            result: Value(Complex),
            parameters: &[Value(Complex)],
        },
    ),
    (
        b"copysign",
        Signature {
            result: Value(Floating),
            parameters: &[Value(Floating), Value(Floating)],
        },
    ),
    (
        b"cos",
        Signature {
            result: Value(Floating),
            parameters: &[Value(Floating)],
        },
    ),
    (
        b"cosh",
        Signature {
            result: Value(Floating),
            parameters: &[Value(Floating)],
        },
    ),
    (
        b"cpow",
        Signature {
            result: Value(Complex),
            parameters: &[Value(Complex), Value(Complex)],
        },
    ),
    (
        b"cproj",
        Signature {
            result: Value(Complex),
            parameters: &[Value(Complex)],
        },
    ),
    (
        b"creal",
        Signature {
            result: Value(Floating),
            parameters: &[Value(Complex)],
        },
    ),
    (
        b"csin",
        Signature {
            result: Value(Complex),
            parameters: &[Value(Complex)],
        },
    ),
    (
        b"csinh",
        Signature {
            result: Value(Complex),
            parameters: &[Value(Complex)],
        },
    ),
    (
        b"csqrt",
        Signature {
            result: Value(Complex),
            parameters: &[Value(Complex)],
        },
    ),
    (
        b"ctan",
        Signature {
            result: Value(Complex),
            parameters: &[Value(Complex)],
        },
    ),
    (
        b"ctanh",
        Signature {
            result: Value(Complex),
            parameters: &[Value(Complex)],
        },
    ),
    (
        b"erf",
        Signature {
            result: Value(Floating),
            parameters: &[Value(Floating)],
        },
    ),
    (
        b"erfc",
        Signature {
            result: Value(Floating),
            parameters: &[Value(Floating)],
        },
    ),
    (
        b"exp",
        Signature {
            result: Value(Floating),
            parameters: &[Value(Floating)],
        },
    ),
    (
        b"exp2",
        Signature {
            result: Value(Floating),
            parameters: &[Value(Floating)],
        },
    ),
    (
        b"expm1",
        Signature {
            result: Value(Floating),
            parameters: &[Value(Floating)],
        },
    ),
    (
        b"fabs",
        Signature {
            result: Value(Floating),
            parameters: &[Value(Floating)],
        },
    ),
    (
        b"fdim",
        Signature {
            result: Value(Floating),
            parameters: &[Value(Floating), Value(Floating)],
        },
    ),
    (
        b"floor",
        Signature {
            result: Value(Floating),
            parameters: &[Value(Floating)],
        },
    ),
    (
        b"fma",
        Signature {
            result: Value(Floating),
            parameters: &[Value(Floating), Value(Floating), Value(Floating)],
        },
    ),
    (
        b"fmax",
        Signature {
            result: Value(Floating),
            parameters: &[Value(Floating), Value(Floating)],
        },
    ),
    (
        b"fmin",
        Signature {
            result: Value(Floating),
            parameters: &[Value(Floating), Value(Floating)],
        },
    ),
    (
        b"fmod",
        Signature {
            result: Value(Floating),
            parameters: &[Value(Floating), Value(Floating)],
        },
    ),
    (
        b"frexp",
        Signature {
            result: Value(Floating),
            parameters: &[Value(Floating), Pointer(Scalar(F::Int))],
        },
    ),
    (
        b"hypot",
        Signature {
            result: Value(Floating),
            parameters: &[Value(Floating), Value(Floating)],
        },
    ),
    (
        b"ilogb",
        Signature {
            result: Value(Scalar(F::Int)),
            parameters: &[Value(Floating)],
        },
    ),
    (
        b"ldexp",
        Signature {
            result: Value(Floating),
            parameters: &[Value(Floating), Value(Scalar(F::Int))],
        },
    ),
    (
        b"lgamma",
        Signature {
            result: Value(Floating),
            parameters: &[Value(Floating)],
        },
    ),
    (
        b"llrint",
        Signature {
            result: Value(Scalar(F::LongLongInt)),
            parameters: &[Value(Floating)],
        },
    ),
    (
        b"llround",
        Signature {
            result: Value(Scalar(F::LongLongInt)),
            parameters: &[Value(Floating)],
        },
    ),
    (
        b"log",
        Signature {
            result: Value(Floating),
            parameters: &[Value(Floating)],
        },
    ),
    (
        b"log10",
        Signature {
            result: Value(Floating),
            parameters: &[Value(Floating)],
        },
    ),
    (
        b"log1p",
        Signature {
            result: Value(Floating),
            parameters: &[Value(Floating)],
        },
    ),
    (
        b"log2",
        Signature {
            result: Value(Floating),
            parameters: &[Value(Floating)],
        },
    ),
    (
        b"logb",
        Signature {
            result: Value(Floating),
            parameters: &[Value(Floating)],
        },
    ),
    (
        b"lrint",
        Signature {
            result: Value(Scalar(F::LongInt)),
            parameters: &[Value(Floating)],
        },
    ),
    (
        b"lround",
        Signature {
            result: Value(Scalar(F::LongInt)),
            parameters: &[Value(Floating)],
        },
    ),
    (
        b"modf",
        Signature {
            result: Value(Floating),
            parameters: &[Value(Floating), Pointer(Floating)],
        },
    ),
    (
        b"nearbyint",
        Signature {
            result: Value(Floating),
            parameters: &[Value(Floating)],
        },
    ),
    (
        b"nextafter",
        Signature {
            result: Value(Floating),
            parameters: &[Value(Floating), Value(Floating)],
        },
    ),
    (
        b"nexttoward",
        Signature {
            result: Value(Floating),
            parameters: &[Value(Floating), Value(Scalar(F::LongDouble))],
        },
    ),
    (
        b"pow",
        Signature {
            result: Value(Floating),
            parameters: &[Value(Floating), Value(Floating)],
        },
    ),
    (
        b"remainder",
        Signature {
            result: Value(Floating),
            parameters: &[Value(Floating), Value(Floating)],
        },
    ),
    (
        b"remquo",
        Signature {
            result: Value(Floating),
            parameters: &[Value(Floating), Value(Floating), Pointer(Scalar(F::Int))],
        },
    ),
    (
        b"rint",
        Signature {
            result: Value(Floating),
            parameters: &[Value(Floating)],
        },
    ),
    (
        b"round",
        Signature {
            result: Value(Floating),
            parameters: &[Value(Floating)],
        },
    ),
    (
        b"scalbln",
        Signature {
            result: Value(Floating),
            parameters: &[Value(Floating), Value(Scalar(F::LongInt))],
        },
    ),
    (
        b"scalbn",
        Signature {
            result: Value(Floating),
            parameters: &[Value(Floating), Value(Scalar(F::Int))],
        },
    ),
    (
        b"sin",
        Signature {
            result: Value(Floating),
            parameters: &[Value(Floating)],
        },
    ),
    (
        b"sinh",
        Signature {
            result: Value(Floating),
            parameters: &[Value(Floating)],
        },
    ),
    (
        b"sqrt",
        Signature {
            result: Value(Floating),
            parameters: &[Value(Floating)],
        },
    ),
    (
        b"tan",
        Signature {
            result: Value(Floating),
            parameters: &[Value(Floating)],
        },
    ),
    (
        b"tanh",
        Signature {
            result: Value(Floating),
            parameters: &[Value(Floating)],
        },
    ),
    (
        b"tgamma",
        Signature {
            result: Value(Floating),
            parameters: &[Value(Floating)],
        },
    ),
    (
        b"trunc",
        Signature {
            result: Value(Floating),
            parameters: &[Value(Floating)],
        },
    ),
];
fn find(table: &[(&[u8], Signature)], name: &[u8]) -> Option<Signature> {
    table
        .binary_search_by_key(&name, |&(name, _)| name)
        .ok()
        .map(|index| table[index].1)
}
#[must_use]
pub fn lookup(name: &[u8]) -> Option<LibraryBuiltin> {
    let name = name.strip_prefix(b"__builtin_")?;
    if let Some(signature) = find(FIXED, name).or_else(|| find(LIBM, name)) {
        return Some(LibraryBuiltin {
            signature,
            floating: F::Double,
        });
    }
    let (&suffix, base) = name.split_last()?;
    let floating = match suffix {
        b'f' => F::Float,
        b'l' => F::LongDouble,
        _ => return None,
    };
    Some(LibraryBuiltin {
        signature: find(LIBM, base)?,
        floating,
    })
}

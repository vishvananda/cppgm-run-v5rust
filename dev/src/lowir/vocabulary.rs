//! `LowIR` scalar and operation vocabulary, translated from `lowir_model.h`. See NOTICE.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Type {
    Void,
    I1,
    I8,
    U8,
    I16,
    U16,
    I32,
    U32,
    I64,
    I128,
    F32,
    F64,
    F80,
    Ptr,
    Object { bytes: u64, align: u8 },
}
impl Type {
    pub(crate) fn scalar(text: &str) -> Option<Self> {
        Some(match text {
            "void" => Self::Void,
            "i1" => Self::I1,
            "i8" => Self::I8,
            "u8" => Self::U8,
            "i16" => Self::I16,
            "u16" => Self::U16,
            "i32" => Self::I32,
            "u32" => Self::U32,
            "i64" => Self::I64,
            "i128" => Self::I128,
            "f32" => Self::F32,
            "f64" => Self::F64,
            "f80" => Self::F80,
            "ptr" => Self::Ptr,
            _ => return None,
        })
    }
    pub(crate) fn bits(self) -> u32 {
        match self {
            Self::I1 => 1,
            Self::I8 | Self::U8 => 8,
            Self::I16 | Self::U16 => 16,
            Self::I32 | Self::U32 => 32,
            Self::I64 => 64,
            Self::I128 => 128,
            _ => 0,
        }
    }
    pub(crate) fn float_rank(self) -> u8 {
        match self {
            Self::F32 => 1,
            Self::F64 => 2,
            Self::F80 => 3,
            _ => 0,
        }
    }
}
impl std::fmt::Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = match self {
            Self::Void => "void",
            Self::I1 => "i1",
            Self::I8 => "i8",
            Self::U8 => "u8",
            Self::I16 => "i16",
            Self::U16 => "u16",
            Self::I32 => "i32",
            Self::U32 => "u32",
            Self::I64 => "i64",
            Self::I128 => "i128",
            Self::F32 => "f32",
            Self::F64 => "f64",
            Self::F80 => "f80",
            Self::Ptr => "ptr",
            Self::Object { bytes, align } => return write!(f, "obj<{bytes}x{}>", 1_u64 << align),
        };
        f.write_str(text)
    }
}
// These paired tables are the shared reader/writer vocabulary, not implementation fragments.
macro_rules! vocabulary {
    ($name:ident { $($variant:ident => $word:literal),+ $(,)? }) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub(crate) enum $name { $($variant),+ }
        impl $name {
            pub(crate) fn parse(text: &str) -> Option<Self> {
                match text { $($word => Some(Self::$variant),)+ _ => None }
            }
            pub(crate) fn spelling(self) -> &'static str {
                match self { $(Self::$variant => $word,)+ }
            }
        }
    };
}
vocabulary!(Unary { Neg => "neg", Not => "not", BitNot => "bitnot", ByteSwap => "bswap", Ctz => "ctz", Clz => "clz" });
vocabulary!(Binary { Add => "add", Sub => "sub", Mul => "mul", Div => "div", Mod => "mod", UDiv => "udiv", UMod => "umod", And => "and", Or => "or", Xor => "xor", Shl => "shl", Shr => "shr", UShr => "ushr" });
vocabulary!(Compare { Eq => "eq", Ne => "ne", Lt => "lt", Le => "le", Gt => "gt", Ge => "ge", Ult => "ult", Ule => "ule", Ugt => "ugt", Uge => "uge" });
impl Compare {
    pub(crate) fn swapped(self) -> Self {
        match self {
            Self::Eq | Self::Ne => self,
            Self::Lt => Self::Gt,
            Self::Le => Self::Ge,
            Self::Gt => Self::Lt,
            Self::Ge => Self::Le,
            Self::Ult => Self::Ugt,
            Self::Ule => Self::Uge,
            Self::Ugt => Self::Ult,
            Self::Uge => Self::Ule,
        }
    }
    pub(crate) fn inverted(self) -> Self {
        match self {
            Self::Eq => Self::Ne,
            Self::Ne => Self::Eq,
            Self::Lt => Self::Ge,
            Self::Le => Self::Gt,
            Self::Gt => Self::Le,
            Self::Ge => Self::Lt,
            Self::Ult => Self::Uge,
            Self::Ule => Self::Ugt,
            Self::Ugt => Self::Ule,
            Self::Uge => Self::Ult,
        }
    }
}
vocabulary!(Convert { SignExtend => "sext", ZeroExtend => "zext", Truncate => "trunc", SignedToFloat => "sitofp", UnsignedToFloat => "uitofp", FloatToSigned => "fptosi", FloatToUnsigned => "fptoui", FloatExtend => "fpext", FloatTruncate => "fptrunc" });
vocabulary!(Role {
    Entry => "entry", Init => "init", Fini => "fini",
    EhAllocateException => "eh_allocate_exception", EhBeginCatch => "eh_begin_catch",
    EhEndCatch => "eh_end_catch", EhRethrow => "eh_rethrow", EhThrow => "eh_throw",
    EhPersonality => "eh_personality", EhResume => "eh_resume", AllocateMemory => "allocate_memory",
    FreeMemory => "free_memory", Terminate => "terminate", PureVirtual => "pure_virtual",
    DynamicCast => "dynamic_cast", BadCast => "bad_cast", BadTypeId => "bad_typeid",
    RttiClass => "rtti_class", RttiSi => "rtti_si", RttiVmi => "rtti_vmi", RttiData => "rtti_data",
    BadArrayNewLength => "bad_array_new_length", RttiPointer => "rtti_pointer",
});
impl Role {
    pub(crate) fn global(self) -> bool {
        matches!(
            self,
            Self::RttiClass | Self::RttiSi | Self::RttiVmi | Self::RttiData | Self::RttiPointer
        )
    }
}
pub(crate) fn named_literal(text: &str) -> bool {
    if text == "nullptr" {
        return true;
    }
    if matches!(text, "inf" | "INFINITY" | "nan" | "NAN" | "snan" | "SNAN") {
        return true;
    }
    let base = text.strip_suffix(['f', 'F', 'l', 'L']).unwrap_or(text);
    matches!(base, "inf" | "INFINITY" | "nan" | "NAN" | "snan" | "SNAN")
}
vocabulary!(Binding { Internal => "internal", Strong => "strong", Weak => "weak" });
vocabulary!(Storage { ReadOnly => "readonly", ThreadLocal => "thread_local" });
vocabulary!(Effects { ReadNone => "readnone", ReadOnly => "readonly" });
vocabulary!(Pass { IndirectResult => "indirect_result", ByAddress => "by_address" });
vocabulary!(Projection { ArrayElement => "array_element", Field => "field" });
vocabulary!(EightbyteClass { None => "none", Integer => "integer", Sse => "sse", SseUp => "sseup" });
vocabulary!(AtomicUpdate { AddFetch => "atomic_add_fetch", Exchange => "atomic_exchange" });
vocabulary!(Fence { Thread => "atomic_thread_fence", Signal => "atomic_signal_fence" });
/// GNU/Clang atomic ordering numbers, retained without strengthening C++'s rules.
#[derive(Clone, Copy)]
#[repr(u8)]
pub(crate) enum MemoryOrder {
    Relaxed,
    Consume,
    Acquire,
    Release,
    AcquireRelease,
    SequentiallyConsistent,
}
impl MemoryOrder {
    pub(crate) fn from_number(number: i64) -> Option<Self> {
        Some(match number {
            0 => Self::Relaxed,
            1 => Self::Consume,
            2 => Self::Acquire,
            3 => Self::Release,
            4 => Self::AcquireRelease,
            5 => Self::SequentiallyConsistent,
            _ => return None,
        })
    }
    pub(crate) fn number(self) -> u8 {
        self as u8
    }
}
#[derive(Clone, Copy)]
pub(crate) enum ObjectClasses {
    Memory,
    X87,
    Direct(EightbyteClass, Option<EightbyteClass>),
}
impl ObjectClasses {
    pub(crate) fn parse(text: &str, bytes: u64) -> Option<Self> {
        if text == "memory" {
            return Some(Self::Memory);
        }
        if bytes == 0 || bytes > 8 {
            let (first, second) = text.split_once('_')?;
            Some(Self::Direct(
                EightbyteClass::parse(first)?,
                Some(EightbyteClass::parse(second)?),
            ))
        } else {
            Some(Self::Direct(EightbyteClass::parse(text)?, None))
        }
    }
}
impl std::fmt::Display for ObjectClasses {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Memory => f.write_str("memory"),
            Self::X87 => f.write_str("x87"),
            Self::Direct(first, second) => {
                f.write_str(first.spelling())?;
                if let Some(second) = second {
                    write!(f, "_{}", second.spelling())?;
                }
                Ok(())
            }
        }
    }
}

/// Pre-intern the frozen reader vocabulary, including metadata keys and named
/// literal variants. These are real unit-owned names, even in an empty input.
pub(crate) fn intern_reader_words(names: &mut super::model::Names) -> Result<(), super::Error> {
    for word in [
        "declare",
        "global",
        "function",
        "alias",
        "object",
        "slot",
        "block",
        "zero",
        "as",
        "arity",
        "effects",
        "unwind",
        "return",
        "result_classes",
        "storage",
        "section",
        "role",
        "linkage",
        "binding",
        "tls_for",
        "keep_alias",
        "prefer_local",
        "object_root",
        "force_inline",
        "inline_hint",
        "no_inline",
        "replaceable",
        "pass",
        "object_bytes",
        "classes",
        "elision",
        "projection",
        "const",
        "copy",
        "phi",
        "addr",
        "load",
        "atomic_load",
        "index",
        "unary",
        "binary",
        "cmp",
        "select",
        "convert",
        "atomic_add_fetch",
        "atomic_exchange",
        "atomic_compare_exchange",
        "va_arg",
        "stack_alloc",
        "call",
        "exception",
        "exception_selector",
        "store",
        "atomic_store",
        "atomic_thread_fence",
        "atomic_signal_fence",
        "va_start",
        "copyobj",
        "zeroinit",
        "eh_try",
        "eh_cleanup",
        "eh_catch",
        "eh_filter",
        "eh_catch_all",
        "eh_end",
        "throw",
        "resume",
        "jump",
        "branch",
        "switch",
        "unreachable",
        "neg",
        "not",
        "bitnot",
        "bswap",
        "ctz",
        "clz",
        "add",
        "sub",
        "mul",
        "div",
        "mod",
        "udiv",
        "umod",
        "and",
        "or",
        "xor",
        "shl",
        "shr",
        "ushr",
        "eq",
        "ne",
        "lt",
        "le",
        "gt",
        "ge",
        "ult",
        "ule",
        "ugt",
        "uge",
        "sext",
        "zext",
        "trunc",
        "sitofp",
        "uitofp",
        "fptosi",
        "fptoui",
        "fpext",
        "fptrunc",
        "volatile",
        "typed",
        "nowrap",
        "exact",
        "readonly",
        "thread_local",
        "void",
        "i1",
        "i8",
        "u8",
        "i16",
        "u16",
        "i32",
        "u32",
        "i64",
        "i128",
        "f32",
        "f64",
        "f80",
        "ptr",
        "nullptr",
    ] {
        names.intern(word)?;
    }
    for literal in ["inf", "INFINITY", "nan", "NAN", "snan", "SNAN"] {
        for suffix in ["", "f", "F", "l", "L"] {
            names.intern(&format!("{literal}{suffix}"))?;
        }
    }
    Ok(())
}

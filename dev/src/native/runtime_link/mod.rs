//! Requested course runtime members and dependency-ordered construction. See NOTICE.
use super::object::Object;
mod allocation;
mod cast;
mod catch;
mod core;
mod division;
mod errors;
mod exceptions;
mod exits;
mod numeric_conversions;
mod program;
mod rtti;

use crate::lowir::Error;
#[derive(Clone, Copy)]
pub(super) enum Support {
    Allocation,
    ExitHandlers,
    Exceptions,
    DynamicCast,
    BadCast,
    WideDivision,
    FloatConversion,
}
impl Support {
    pub(super) const ALL: [Self; 7] = [
        Self::Allocation,
        Self::ExitHandlers,
        Self::Exceptions,
        Self::DynamicCast,
        Self::BadCast,
        Self::WideDivision,
        Self::FloatConversion,
    ];
    fn index(self) -> usize {
        match self {
            Self::Allocation => 0,
            Self::ExitHandlers => 1,
            Self::Exceptions => 2,
            Self::DynamicCast => 3,
            Self::BadCast => 4,
            Self::WideDivision => 5,
            Self::FloatConversion => 6,
        }
    }
}
#[derive(Clone, Copy)]
pub(super) enum Member {
    Entry,
    EhCore,
    System,
    Terminate,
    TypeVtables,
    Builtins,
    Fundamental,
    Support(Support),
}
impl Member {
    fn index(self) -> usize {
        match self {
            Self::Entry => 0,
            Self::EhCore => 1,
            Self::System => 2,
            Self::Terminate => 3,
            Self::TypeVtables => 4,
            Self::Builtins => 5,
            Self::Fundamental => 6,
            Self::Support(s) => 7 + s.index(),
        }
    }
    fn dependencies(self) -> &'static [Self] {
        match self {
            Self::Fundamental | Self::Support(Support::DynamicCast) => &[Self::TypeVtables],
            Self::Support(Support::Allocation) => &[Self::System, Self::Terminate],
            Self::Support(Support::ExitHandlers) => {
                &[Self::Support(Support::Allocation), Self::Entry]
            }
            Self::Support(Support::Exceptions) => &[
                Self::Support(Support::Allocation),
                Self::EhCore,
                Self::Terminate,
                Self::TypeVtables,
            ],
            Self::Support(Support::BadCast) => {
                &[Self::Support(Support::Exceptions), Self::TypeVtables]
            }
            _ => &[],
        }
    }
}
pub(super) struct Members(u16);
impl Members {
    pub(super) fn contains(&self, member: Member) -> bool {
        self.0 & (1 << member.index()) != 0
    }
}
pub(super) const VTABLES: [&[u8]; 9] = [
    b"_ZTVN10__cxxabiv117__class_type_infoE",
    b"_ZTVN10__cxxabiv120__si_class_type_infoE",
    b"_ZTVN10__cxxabiv121__vmi_class_type_infoE",
    b"_ZTVN10__cxxabiv123__fundamental_type_infoE",
    b"_ZTVN10__cxxabiv119__pointer_type_infoE",
    b"_ZTVN10__cxxabiv129__pointer_to_member_type_infoE",
    b"_ZTVN10__cxxabiv120__function_type_infoE",
    b"_ZTVN10__cxxabiv117__array_type_infoE",
    b"_ZTVN10__cxxabiv116__enum_type_infoE",
];
fn fundamental(code: &[u8]) -> bool {
    matches!(code,[byte] if b"vwbcahstijlmxynofde".contains(byte))
        || matches!(code, b"Dn" | b"Ds" | b"Di")
}
pub(super) fn runtime_type(code: &[u8]) -> Option<(bool, u8, &[u8])> {
    if fundamental(code) {
        return Some((false, 0, code));
    }
    let mut rest = code.strip_prefix(b"P")?;
    let mut flags = 0;
    if let Some(next) = rest.strip_prefix(b"V") {
        flags |= 2;
        rest = next;
    }
    if let Some(next) = rest.strip_prefix(b"K") {
        flags |= 1;
        rest = next;
    }
    fundamental(rest).then_some((true, flags, rest))
}
pub(super) fn type_name(name: &[u8]) -> Option<&[u8]> {
    let code = name
        .strip_prefix(b"_ZTI")
        .or_else(|| name.strip_prefix(b"_ZTS"))?;
    runtime_type(code).map(|_| code)
}
pub(super) fn member_of(name: &[u8]) -> Option<Member> {
    Some(match name {
        b"_start"
        | b"__cppgm_exit"
        | b"__cppgm_atexit_head"
        | b"__cppgm_thread_atexit_head"
        | b"__dso_handle" => Member::Entry,
        b"__cppgm_eh_top"
        | b"__cppgm_eh_payload"
        | b"__cppgm_eh_selector"
        | b"__cppgm_eh_dispatch"
        | b"__cppgm_eh_uncaught" => Member::EhCore,
        b"__cppgm_mmap" | b"__cppgm_munmap" | b"__cppgm_mremap" => Member::System,
        b"_ZSt9terminatev" | b"__cxa_pure_virtual" | b"__cxa_deleted_virtual" | b"abort" => {
            Member::Terminate
        }
        b"cppgm_builtin_strlen"
        | b"cppgm_builtin_memcpy"
        | b"cppgm_builtin_memmove"
        | b"cppgm_builtin_memset"
        | b"cppgm_builtin_memcmp"
        | b"strlen"
        | b"memcpy"
        | b"memmove"
        | b"memset"
        | b"memcmp" => Member::Builtins,
        b"__cppgm_allocate"
        | b"__cppgm_release"
        | b"cppgm_builtin_operator_new"
        | b"cppgm_builtin_operator_new_array"
        | b"cppgm_builtin_operator_delete"
        | b"cppgm_builtin_operator_delete_array"
        | b"_Znwm"
        | b"_Znam"
        | b"_ZdlPv"
        | b"_ZdaPv"
        | b"_ZdlPvm"
        | b"_ZdaPvm"
        | b"__cppgm_operator_new"
        | b"malloc"
        | b"free"
        | b"calloc"
        | b"realloc" => Member::Support(Support::Allocation),
        b"atexit"
        | b"__cxa_atexit"
        | b"__cxa_thread_atexit"
        | b"__cxa_guard_acquire"
        | b"__cxa_guard_release"
        | b"__cxa_guard_abort" => Member::Support(Support::ExitHandlers),
        b"__cxa_allocate_exception"
        | b"__cxa_free_exception"
        | b"__cxa_throw"
        | b"__cxa_begin_catch"
        | b"__cxa_end_catch"
        | b"__cxa_rethrow"
        | b"__cxa_get_exception_ptr"
        | b"__cppgm_eh_catch" => Member::Support(Support::Exceptions),
        b"__dynamic_cast" => Member::Support(Support::DynamicCast),
        b"__cxa_bad_cast" | b"__cxa_bad_typeid" | b"__cxa_throw_bad_array_new_length" => {
            Member::Support(Support::BadCast)
        }
        b"__divti3" | b"__udivti3" | b"__modti3" | b"__umodti3" => {
            Member::Support(Support::WideDivision)
        }
        b"__floattisf" | b"__floattidf" | b"__floattixf" | b"__floatuntisf" | b"__floatuntidf"
        | b"__floatuntixf" | b"__fixsfti" | b"__fixdfti" | b"__fixxfti" | b"__fixunssfti"
        | b"__fixunsdfti" | b"__fixunsxfti" => Member::Support(Support::FloatConversion),
        _ if VTABLES.contains(&name) => Member::TypeVtables,
        _ if type_name(name).is_some() => Member::Fundamental,
        _ => return None,
    })
}
pub(super) fn provides(name: &[u8]) -> bool {
    member_of(name).is_some()
}
pub(super) fn build(names: &[&[u8]]) -> Result<Vec<Object>, Error> {
    let mut members = Members(0);
    let mut work = vec![Member::Entry];
    work.extend(names.iter().filter_map(|name| member_of(name)));
    while let Some(member) = work.pop() {
        if members.contains(member) {
            continue;
        }
        members.0 |= 1 << member.index();
        work.extend_from_slice(member.dependencies());
    }
    let core = core::build(&members, names)?;
    let mut objects = vec![core];
    if Support::ALL
        .iter()
        .copied()
        .any(|s| members.contains(Member::Support(s)))
    {
        let mut program = crate::lowir::model::Program::default();
        let mut runtime = program::RuntimeProgram {
            program: &mut program,
        };
        for member in Support::ALL {
            if !members.contains(Member::Support(member)) {
                continue;
            }
            match member {
                Support::Allocation => allocation::build(&mut runtime)?,
                Support::ExitHandlers => exits::build(&mut runtime)?,
                Support::Exceptions => exceptions::build(&mut runtime)?,
                Support::WideDivision => division::build(&mut runtime)?,
                Support::FloatConversion => numeric_conversions::build(&mut runtime)?,
                Support::DynamicCast => cast::build(&mut runtime)?,
                Support::BadCast => errors::build(&mut runtime)?,
            }
        }
        let mut object =
            super::object_compile::compile(&mut program, b"<runtime library>".to_vec(), 0)?;
        for symbol in object.symbols_mut() {
            if symbol.binding == super::object::Binding::Global
                && symbol.definition != super::object::Definition::Undefined
            {
                symbol.binding = super::object::Binding::Weak;
            }
        }
        objects.push(object);
    }
    Ok(objects)
}

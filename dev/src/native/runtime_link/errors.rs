//! Type information and throwers for runtime standard exceptions. See NOTICE.
use super::{
    VTABLES,
    program::{RuntimeProgram, Word},
};
use crate::lowir::{Error, vocabulary::Type as T};
fn thrower(
    runtime: &mut RuntimeProgram<'_>,
    routine: &[u8],
    ty: &[u8],
    name: &[u8],
    table: &[u8],
    base: Option<(&[u8], &[u8])>,
) -> Result<(), Error> {
    let exception_name = runtime.string(b"_ZTSSt9exception", b"St9exception")?;
    // Obtain the symbol before constructing its definition, as in symbol_for.
    let mut base_symbol = runtime.reference(b"_ZTISt9exception")?;
    if !runtime.defined(b"_ZTISt9exception")? {
        let vtable = runtime.global(VTABLES[0], 0)?;
        runtime.words(
            b"_ZTISt9exception",
            &[
                Word {
                    symbol: Some(vtable),
                    value: 16,
                },
                Word {
                    symbol: Some(exception_name),
                    value: 0,
                },
            ],
        )?;
    }
    if let Some((base_type, base_name)) = base {
        let exception = base_symbol;
        base_symbol = runtime.reference(base_type)?;
        if !runtime.defined(base_type)? {
            let vtable = runtime.global(VTABLES[1], 0)?;
            let name = [b"_ZTS", base_name].concat();
            let name = runtime.string(&name, base_name)?;
            runtime.words(
                base_type,
                &[
                    Word {
                        symbol: Some(vtable),
                        value: 16,
                    },
                    Word {
                        symbol: Some(name),
                        value: 0,
                    },
                    Word {
                        symbol: Some(exception),
                        value: 0,
                    },
                ],
            )?;
        }
    }
    let type_name = [b"_ZTS", name].concat();
    let type_name = runtime.string(&type_name, name)?;
    let vtable = runtime.global(VTABLES[1], 0)?;
    let information = runtime.words(
        ty,
        &[
            Word {
                symbol: Some(vtable),
                value: 16,
            },
            Word {
                symbol: Some(type_name),
                value: 0,
            },
            Word {
                symbol: Some(base_symbol),
                value: 0,
            },
        ],
    )?;
    let table = runtime.words(
        table,
        &[
            Word {
                symbol: None,
                value: 0,
            },
            Word {
                symbol: Some(information),
                value: 0,
            },
            Word {
                symbol: None,
                value: 0,
            },
            Word {
                symbol: None,
                value: 0,
            },
        ],
    )?;
    let symbol = runtime.function(routine, T::Void, &[], true)?;
    let mut r = runtime.routine(symbol)?;
    let eight = r.integer(8)?;
    let allocate = r
        .runtime()
        .function(b"__cxa_allocate_exception", T::Ptr, &[T::I64], false)?;
    let object = r.call(T::Ptr, allocate, &[eight])?;
    let address = r.address(table)?;
    let value = r.at_offset(address, 16)?;
    r.store(T::Ptr, value, object)?;
    let address = r.address(information)?;
    let zero = r.integer(0)?;
    let throw = r
        .runtime()
        .function(b"__cxa_throw", T::Void, &[T::Ptr, T::Ptr, T::Ptr], false)?;
    r.call_void(throw, &[object, address, zero])?;
    r.unreachable()?;
    r.finish()
}
pub(super) fn build(runtime: &mut RuntimeProgram<'_>) -> Result<(), Error> {
    thrower(
        runtime,
        b"__cxa_bad_cast",
        b"_ZTISt8bad_cast",
        b"St8bad_cast",
        b"_ZTVSt8bad_cast",
        None,
    )?;
    thrower(
        runtime,
        b"__cxa_bad_typeid",
        b"_ZTISt10bad_typeid",
        b"St10bad_typeid",
        b"_ZTVSt10bad_typeid",
        None,
    )?;
    thrower(
        runtime,
        b"__cxa_throw_bad_array_new_length",
        b"_ZTISt20bad_array_new_length",
        b"St20bad_array_new_length",
        b"_ZTVSt20bad_array_new_length",
        Some((b"_ZTISt9bad_alloc", b"St9bad_alloc")),
    )
}

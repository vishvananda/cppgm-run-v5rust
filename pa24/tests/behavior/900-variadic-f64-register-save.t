function @read_f64(%tag : i64) -> i64 [arity=variadic] {
  slot $arguments : obj<24x8>

  block ^entry:
    %list = addr $arguments
    va_start %list
    %fp_offset_address = index i8 %list, 4
    %fp_offset = load i32 %fp_offset_address
    %offset = convert zext i64 i32 %fp_offset
    %save_address = index i8 %list, 16
    %save = load ptr %save_address
    %argument = index i8 %save, %offset
    %value = load f64 %argument
    %integer = convert fptosi i64 f64 %value
    return i64 %integer
}

function @main() -> i64 [role=entry] {
  block ^entry:
    %value = call i64 @read_f64(0, 42.0)
    %ok = cmp eq i64 %value, 42
    %failed = cmp eq i64 %ok, 0
    return i64 %failed
}

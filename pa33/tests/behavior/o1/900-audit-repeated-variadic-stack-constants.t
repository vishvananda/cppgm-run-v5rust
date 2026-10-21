function @read(%tag : i64) -> i64 [arity=variadic] {
  slot $arguments : obj<24x8>
  block ^entry:
    %list = addr $arguments
    va_start %list
    %a = va_arg i64 %list
    %b = va_arg i64 %list
    %c = va_arg i64 %list
    %d = va_arg i64 %list
    %e = va_arg i64 %list
    %f = va_arg i64 %list
    %g = va_arg i64 %list
    %h = va_arg i64 %list
    return i64 %h
}

function @main() -> i64 [role=entry] {
  block ^entry:
    %value = call i64 @read(0, 1234567890123, 1234567890123, 1234567890123, 1234567890123, 1234567890123, 1234567890123, 1234567890123, 1234567890123)
    %failed = cmp ne i64 %value, 1234567890123
    return i64 %failed
}

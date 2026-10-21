function @read(%tag : i64) -> i64 [arity=variadic] {
  slot $arguments : obj<24x8>
  block ^entry:
    %list = addr $arguments
    va_start %list
    %i0 = va_arg i64 %list
    %i1 = va_arg i64 %list
    %i2 = va_arg i64 %list
    %i3 = va_arg i64 %list
    %i4 = va_arg i64 %list
    %i5 = va_arg i64 %list
    %i6 = va_arg i64 %list
    %f0 = va_arg f64 %list
    %c0 = convert fptosi i64 f64 %f0
    %f1 = va_arg f64 %list
    %c1 = convert fptosi i64 f64 %f1
    %f2 = va_arg f64 %list
    %c2 = convert fptosi i64 f64 %f2
    %f3 = va_arg f64 %list
    %c3 = convert fptosi i64 f64 %f3
    %f4 = va_arg f64 %list
    %c4 = convert fptosi i64 f64 %f4
    %f5 = va_arg f64 %list
    %c5 = convert fptosi i64 f64 %f5
    %f6 = va_arg f64 %list
    %c6 = convert fptosi i64 f64 %f6
    %f7 = va_arg f64 %list
    %c7 = convert fptosi i64 f64 %f7
    %f8 = va_arg f64 %list
    %c8 = convert fptosi i64 f64 %f8
    %f9 = va_arg f64 %list
    %c9 = convert fptosi i64 f64 %f9
    %sum0 = binary add i64 %i0, %i1
    %sum1 = binary add i64 %sum0, %i2
    %sum2 = binary add i64 %sum1, %i3
    %sum3 = binary add i64 %sum2, %i4
    %sum4 = binary add i64 %sum3, %i5
    %sum5 = binary add i64 %sum4, %i6
    %sum6 = binary add i64 %sum5, %c0
    %sum7 = binary add i64 %sum6, %c1
    %sum8 = binary add i64 %sum7, %c2
    %sum9 = binary add i64 %sum8, %c3
    %sum10 = binary add i64 %sum9, %c4
    %sum11 = binary add i64 %sum10, %c5
    %sum12 = binary add i64 %sum11, %c6
    %sum13 = binary add i64 %sum12, %c7
    %sum14 = binary add i64 %sum13, %c8
    %sum15 = binary add i64 %sum14, %c9
    return i64 %sum15
}

function @main() -> i64 [role=entry] {
  block ^entry:
    %value = call i64 @read(0, 1, 2, 3, 4, 5, 6, 7, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0, 18.0, 19.0, 20.0)
    %failed = cmp ne i64 %value, 183
    return i64 %failed
}

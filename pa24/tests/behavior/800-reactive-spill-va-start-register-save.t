function @sum_variadic(%tag : i64) -> i64 [arity=variadic, binding=strong] {
  slot $ap : obj<24x8>

  block ^entry:
    %t1 = binary add i64 %tag, 1
    %t2 = binary add i64 %tag, 2
    %t3 = binary add i64 %tag, 3
    %t4 = binary add i64 %tag, 4
    %t5 = binary add i64 %tag, 5
    %t6 = binary add i64 %tag, 6
    %t7 = binary add i64 %tag, 7
    %t8 = binary add i64 %tag, 8
    %t9 = binary add i64 %tag, 9
    %t10 = binary add i64 %tag, 10
    %t11 = binary add i64 %tag, 11
    %t12 = binary add i64 %tag, 12
    %list = addr $ap
    va_start %list
    %gp = load i32 %list
    %gp64 = convert zext i64 i32 %gp
    %rsa_ptr_ptr = index i8 %list, 16
    %rsa = load ptr %rsa_ptr_ptr
    %arg_ptr = index i8 %rsa, %gp64
    %v = load i64 %arg_ptr
    %s1 = binary add i64 %t1, %t2
    %s2 = binary add i64 %s1, %t3
    %s3 = binary add i64 %s2, %t4
    %s4 = binary add i64 %s3, %t5
    %s5 = binary add i64 %s4, %t6
    %s6 = binary add i64 %s5, %t7
    %s7 = binary add i64 %s6, %t8
    %s8 = binary add i64 %s7, %t9
    %s9 = binary add i64 %s8, %t10
    %s10 = binary add i64 %s9, %t11
    %s11 = binary add i64 %s10, %t12
    %out = binary add i64 %s11, %v
    return i64 %out
}

function @main() -> i32 [role=entry, binding=strong, keep_alias=yes] {
  block ^entry:
    %r = call i64 @sum_variadic(0, 100)
    %ok = cmp eq i64 %r, 178
    %bad = cmp eq i64 %ok, 0
    %exit = convert trunc i32 i64 %bad
    return i32 %exit
}

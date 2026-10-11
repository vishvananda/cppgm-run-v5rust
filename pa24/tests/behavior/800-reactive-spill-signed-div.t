function @clobber(%a : i64, %b : i64, %c : i64, %d : i64, %e : i64, %f : i64) -> i64 {
  block ^entry:
    return i64 0
}

function @f(%a : i64, %b : i64, %c : i64, %d : i64, %e : i64, %f : i64, %g : i64, %h : i64, %i : i64, %j : i64) -> i64 [binding=strong] {
  block ^entry:
    %t1 = binary add i64 %a, 1
    %t2 = binary add i64 %b, 2
    %t3 = binary add i64 %c, 3
    %t4 = binary add i64 %d, 4
    %t5 = binary add i64 %e, 5
    %t6 = binary add i64 %f, 6
    %t7 = binary add i64 %g, 7
    %t8 = binary add i64 %h, 8
    %t9 = binary add i64 %i, 9
    %t10 = binary add i64 %j, 10
    %ignored = call i64 @clobber(1, 2, 3, 4, 5, 6)
    %q = binary div i64 %j, -3
    %s1 = binary add i64 %t1, %t2
    %s2 = binary add i64 %s1, %t3
    %s3 = binary add i64 %s2, %t4
    %s4 = binary add i64 %s3, %t5
    %s5 = binary add i64 %s4, %t6
    %s6 = binary add i64 %s5, %t7
    %s7 = binary add i64 %s6, %t8
    %s8 = binary add i64 %s7, %t9
    %s9 = binary add i64 %s8, %t10
    %s10 = binary add i64 %s9, %q
    %s11 = binary add i64 %s10, %ignored
    return i64 %s11
}

function @main() -> i32 [role=entry, binding=strong, keep_alias=yes] {
  block ^entry:
    %r = call i64 @f(1, 2, 3, 4, 5, 6, 7, 8, 9, 18)
    %ok = cmp eq i64 %r, 112
    %bad = cmp eq i64 %ok, 0
    %exit = convert trunc i32 i64 %bad
    return i32 %exit
}

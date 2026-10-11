function @sum8(%a : i64, %b : i64, %c : i64, %d : i64, %e : i64, %f : i64, %g : i64, %h : i64) -> i64 {
  block ^entry:
    %t1 = binary add i64 %a, %b
    %t2 = binary add i64 %t1, %c
    %t3 = binary add i64 %t2, %d
    %t4 = binary add i64 %t3, %e
    %t5 = binary add i64 %t4, %f
    %t6 = binary add i64 %t5, %g
    %t7 = binary add i64 %t6, %h
    return i64 %t7
}

function @main() -> i64 [role=entry] {
  block ^entry:
    %a = const i64 1
    %b = const i64 2
    %c = const i64 3
    %d = const i64 4
    %e = const i64 5
    %f = const i64 6
    %g = const i64 7
    %h = const i64 8
    %r = call i64 @sum8(%a, %b, %c, %d, %e, %f, %g, %h)
    %x = binary sub i64 %r, 36
    return i64 %x
}

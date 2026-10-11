global @table = {
  ptr addr @sum6
}

function @sum6(%a : i64, %b : i64, %c : i64, %d : i64, %e : i64, %f : i64) -> i64 {
  block ^entry:
    %t1 = binary add i64 %a, %b
    %t2 = binary add i64 %t1, %c
    %t3 = binary add i64 %t2, %d
    %t4 = binary add i64 %t3, %e
    %t5 = binary add i64 %t4, %f
    return i64 %t5
}

function @main() -> i64 [role=entry] {
  block ^entry:
    %fp = load ptr @table
    %a = const i64 1
    %b = const i64 2
    %c = const i64 3
    %d = const i64 4
    %e = const i64 5
    %f = const i64 6
    %r = call i64 %fp(%a, %b, %c, %d, %e, %f) as (%arg0 : i64, %arg1 : i64, %arg2 : i64, %arg3 : i64, %arg4 : i64, %arg5 : i64) -> i64
    %x = binary sub i64 %r, 21
    return i64 %x
}

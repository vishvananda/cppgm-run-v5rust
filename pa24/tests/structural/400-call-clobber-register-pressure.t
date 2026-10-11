function @add1(%x: i64) -> i64 {
  block ^entry:
    %y = binary add i64 %x, 1
    return i64 %y
}

function @main() -> i64 [role=entry] {
  block ^entry:
    %a = const i64 7
    %b = const i64 8
    %c = binary add i64 %a, %b
    %d = call i64 @add1(20)
    %e = binary add i64 %c, %d
    return i64 %e
}

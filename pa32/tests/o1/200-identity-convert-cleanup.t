function @main(%x : i64, %b : i1) -> i64 {
  block ^entry:
    %a = convert sext i64 i64 %x
    %c = convert zext i1 i1 %b
    %d = binary add i64 %a, 1
    %e = convert trunc i64 i64 %d
    return i64 %e
}

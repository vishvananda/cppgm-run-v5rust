function @main() -> i64 [role=entry] {
  block ^entry:
    %a = cmp ult u32 1, 2
    %b = binary add i64 %a, %a
    return i64 %b
}

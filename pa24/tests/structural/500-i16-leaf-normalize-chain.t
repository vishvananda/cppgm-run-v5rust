function @main() -> i64 [role=entry] {
  block ^entry:
    %a = binary add i16 300, 40
    %b = copy i64 %a
    return i64 %b
}

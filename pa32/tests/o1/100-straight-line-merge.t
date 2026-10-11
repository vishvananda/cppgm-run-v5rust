function @main() -> i64 {
  block ^entry:
    %x = const i64 4
    jump ^body

  block ^body:
    %y = copy i64 %x
    %z = binary add i64 %y, 3
    return i64 %z
}

function @f(%a : i16) -> i16 {
  block ^entry:
    %v = unary ctz i16 %a
    return i16 %v
}

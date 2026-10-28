function @logical(%x : i64) -> i64 {
 block ^entry:
  %a = binary add i64 %x, 7
  %b = unary not i64 %a
  return i64 %b
}

function @high_zero(%x : i64) -> i64 {
 block ^entry:
  %a = binary add i64 %x, 7
  %b = cmp eq i64 %a, 0x10000000000000000
  return i64 %b
}

function @main() -> i64 [role=entry] {
 block ^entry:
  %a = call i64 @logical(-7)
  %b = call i64 @high_zero(-7)
  %bad_a = cmp ne i64 %a, 1
  %bad_b = cmp ne i64 %b, 1
  %bad = binary or i64 %bad_a, %bad_b
  return i64 %bad
}

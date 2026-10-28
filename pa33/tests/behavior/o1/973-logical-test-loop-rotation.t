function @f(%x0 : i64, %y : i64) -> i64 {
 block ^entry:
  jump ^test
 block ^test:
  %x = phi i64 [^entry: %x0, ^body: %next]
  %n = unary not i64 %x
  %z = binary xor i64 %n, %y
  branch %z, ^body, ^done
 block ^body:
  %next = binary sub i64 %x, 1
  jump ^test
 block ^done:
  return i64 %x
}
function @main() -> i64 [role=entry] {
 block ^entry:
  %r = call i64 @f(0, 0)
  %bad = cmp ne i64 %r, -1
  return i64 %bad
}

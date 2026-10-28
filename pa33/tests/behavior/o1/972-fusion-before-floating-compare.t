function @probe(%x : i64, %a : f64, %b : f64) -> i64 {
 block ^entry:
  %y = binary add i64 %x, 5
  %c = cmp lt f64 %a, %b
  branch %c, ^yes, ^no
 block ^yes:
  return i64 %y
 block ^no:
  return i64 %x
}
function @main() -> i64 [role=entry] {
 block ^entry:
  %r = call i64 @probe(2, 1.0, 2.0)
  %bad = cmp ne i64 %r, 7
  return i64 %bad
}

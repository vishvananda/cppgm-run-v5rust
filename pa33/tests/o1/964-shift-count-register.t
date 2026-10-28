function @shift(%x : i64, %y : i64) -> i64 {
 block ^entry:
  %count = binary add i64 %y, 1
  %shifted = binary shl i64 %x, %count
  %result = binary add i64 %shifted, %y
  return i64 %result
}
function @main() -> i64 [role=entry] {
 block ^entry:
  %value = call i64 @shift(3, 1)
  %result = binary sub i64 %value, 13
  return i64 %result
}

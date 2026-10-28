function @main() -> i64 [role=entry] {
 slot $area : obj<12x4>
 block ^entry:
  zeroinit 12x4 $area
  %value = load i64 $area
  return i64 %value
}

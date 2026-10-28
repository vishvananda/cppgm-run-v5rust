function @mask(%value : u8) -> u32 {
 block ^entry:
  eh_try ^handler
  eh_end
  %flags = select u32 %value, 24, 0
  %a = binary and u32 %flags, 8
  branch %a, ^next, ^no
 block ^next:
  %b = binary and u32 %flags, 16
  branch %b, ^yes, ^no
 block ^yes:
  return u32 1
 block ^no:
  return u32 0
 block ^handler:
  return u32 99
}
function @main() -> i64 [role=entry] {
 block ^entry:
  %both = call u32 @mask(1)
  %neither = call u32 @mask(0)
  %bad_both = cmp ne u32 %both, 1
  %bad_neither = cmp ne u32 %neither, 0
  %bad = binary or i64 %bad_both, %bad_neither
  return i64 %bad
}

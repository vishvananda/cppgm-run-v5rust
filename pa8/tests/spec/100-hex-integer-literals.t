# lowir.md: integer literals are read by value; 16, 0x10 and 020 are one
# literal, so 0xff and 0x1e are the integers 255 and 30
function @f() -> i64 {
  block ^entry:
    %a = const i64 0xff
    %b = const i64 0x1e
    return i64 %a
}

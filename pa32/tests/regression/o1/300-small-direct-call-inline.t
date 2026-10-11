function @inc(%x : i64) -> i64 {
  block ^entry:
    %a = binary add i64 %x, 1
    return i64 %a
}
function @main(%x : i64) -> i64 {
  block ^entry:
    %r = call i64 @inc(%x)
    %s = binary mul i64 %r, 2
    return i64 %s
}

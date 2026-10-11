function @add1(%x : i32) -> i32 [force_inline=yes] {
  block ^entry:
    %y = binary add i32 %x, 1
    return i32 %y
}

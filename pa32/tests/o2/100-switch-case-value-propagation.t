function @select(%x : i32) -> i32 {
  block ^entry:
    %negative = unary neg i32 1
    switch %x, ^fallback, %negative:^hit

  block ^fallback:
    return i32 0

  block ^hit:
    return i32 1
}

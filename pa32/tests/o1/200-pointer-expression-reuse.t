function @main(%base : ptr, %flag : i64) -> ptr {
  block ^entry:
    %p = index i8 %base, 8
    branch %flag, ^left, ^right

  block ^left:
    jump ^join

  block ^right:
    jump ^join

  block ^join:
    %q = index i8 %base, 8
    return ptr %q
}

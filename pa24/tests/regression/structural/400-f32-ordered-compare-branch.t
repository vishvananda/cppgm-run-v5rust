function @main() -> i64 [role=entry] {
  block ^entry:
    %0 = cmp gt f32 1.0f, 2.0f
    branch %0, ^wrong, ^right

  block ^wrong:
    return i64 1

  block ^right:
    return i64 0
}

function @main() -> i64 [role=entry] {
  block ^entry:
    %p = stack_alloc 2147483648
    store volatile i64 0, %p
    return i64 0
}

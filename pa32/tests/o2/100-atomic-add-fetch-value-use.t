function @atomic_sub_one(%p : ptr) -> i64 {
  block ^entry:
    %delta = unary neg i32 1
    %new = atomic_add_fetch i64 %p, %delta, 4
    return i64 %new
}

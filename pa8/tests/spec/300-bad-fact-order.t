function @f(%p : ptr) -> i32 {
  block ^entry:
    %v = load typed volatile i32 %p
    return i32 %v
}

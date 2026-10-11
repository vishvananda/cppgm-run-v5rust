function @f(%p : ptr) -> i64 {
  block ^entry:
    %b = load typed u8 %p
    %w = convert zext i64 u8 %b
    return i64 %w
}

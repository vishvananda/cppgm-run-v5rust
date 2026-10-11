function @f(%a : i32) -> i32 {
  block ^entry:
    %q = binary div nowrap i32 %a, 3
    return i32 %q
}

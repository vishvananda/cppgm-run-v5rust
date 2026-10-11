function @main() -> i32 [role=entry, binding=strong, keep_alias=yes] {
  slot $value : f32

  block ^entry:
    store f32 0, $value
    %stored = load f32 $value
    %ok = cmp eq f32 %stored, 0.0
    %failed = cmp eq i64 %ok, 0
    %exit = convert trunc i32 i64 %failed
    return i32 %exit
}

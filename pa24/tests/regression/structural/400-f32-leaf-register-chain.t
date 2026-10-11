function @main() -> i64 [role=entry] {
  block ^entry:
    %a = const f32 1.5f
    %b = const f32 2.0f
    %c = binary add f32 %a, %b
    %d = binary mul f32 %c, 4.0f
    %ok = cmp eq f32 %d, 14.0f
    branch %ok, ^ok, ^bad

  block ^bad:
    return i64 1

  block ^ok:
    return i64 0
}

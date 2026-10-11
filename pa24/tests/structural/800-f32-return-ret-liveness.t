function @makef() -> f32 {
  block ^entry:
    return f32 1.5f
}

function @main() -> i64 [role=entry] {
  block ^entry:
    %v = call f32 @makef() as () -> f32
    %ok = cmp eq f32 %v, 1.5f
    branch %ok, ^right, ^wrong

  block ^wrong:
    return i64 1

  block ^right:
    return i64 0
}

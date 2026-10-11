function @check(%a0 : f32, %a1 : f32, %a2 : f32, %a3 : f32, %a4 : f32, %a5 : f32, %a6 : f32, %a7 : f32) -> i64 {
  block ^entry:
    %ok = cmp eq f32 %a1, 2.0f
    branch %ok, ^pass, ^fail

  block ^fail:
    return i64 1

  block ^pass:
    return i64 0
}

function @main() -> i64 [role=entry] {
  block ^entry:
    %v0 = const f32 1.0f
    %v1 = const f32 2.0f
    %v2 = const f32 3.0f
    %v3 = const f32 4.0f
    %v4 = const f32 5.0f
    %v5 = const f32 6.0f
    %v6 = const f32 7.0f
    %v7 = const f32 8.0f
    %result = call i64 @check(%v2, %v1, %v4, %v0, %v6, %v3, %v5, %v7)
    return i64 %result
}

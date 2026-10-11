global @g : f32 = 1.5f

function @main() -> i64 {
  block ^entry:
    %x = load f32 @g
    %y = const f32 1.5f
    %ok = cmp eq f32 %x, %y
    return i64 %ok
}

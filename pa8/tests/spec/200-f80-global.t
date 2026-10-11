global @g : f80 = 1.25L

function @main() -> i64 {
  block ^entry:
    %0 = load f80 @g
    %1 = const f80 1.25L
    %2 = cmp eq f80 %0, %1
    return i64 %2
}

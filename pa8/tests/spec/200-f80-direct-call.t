function @id(%x : f80) -> f80 {
  block ^entry:
    return f80 %x
}

function @main() -> i64 {
  block ^entry:
    %0 = const f80 1.75L
    %1 = call f80 @id(%0)
    %2 = cmp eq f80 %0, %1
    return i64 %2
}

function @addl(%a : f80, %b : f80) -> f80 {
  block ^entry:
    %sum = binary add f80 %a, %b
    return f80 %sum
}

function @main() -> i64 [role=entry] {
  block ^entry:
    %a = const f80 1.5L
    %b = const f80 2.25L
    %r = call f80 @addl(%a, %b)
    %e = const f80 3.75L
    %ok = cmp eq f80 %r, %e
    return i64 %ok
}

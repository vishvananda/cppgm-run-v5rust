global @value : i64 = 7

function @sum() -> f80 {
  block ^entry:
    %sum = binary add f80 @value, 1.0
    return f80 %sum
}

function @main() -> i64 [role=entry] {
  block ^entry:
    return i64 0
}

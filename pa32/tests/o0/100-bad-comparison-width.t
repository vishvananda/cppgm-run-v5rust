function @bad(%value : i128) -> i64 {
  block ^entry:
    %truth = cmp ne i64 %value, 0
    return i64 %truth
}

function @main() -> i64 [role=entry] {
  block ^entry:
    return i64 0
}

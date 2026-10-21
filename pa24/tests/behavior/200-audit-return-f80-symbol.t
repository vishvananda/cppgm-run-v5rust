global @value : i64 = 7

function @address() -> f80 {
  block ^entry:
    return f80 @value
}

function @main() -> i64 [role=entry] {
  block ^entry:
    return i64 0
}

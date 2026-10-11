function @roundtrip(%x : f64) -> f64 {
  slot $y : f80

  block ^entry:
    store f80 %x, $y
    %t = load f80 $y
    return f64 %t
}

function @main() -> i64 [role=entry] {
  block ^entry:
    %r = call f64 @roundtrip(2.0)
    %ok = cmp eq f64 %r, 2.0
    %fail = cmp eq i64 %ok, 0
    return i64 %fail
}

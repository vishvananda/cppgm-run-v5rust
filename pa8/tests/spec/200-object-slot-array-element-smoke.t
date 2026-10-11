function @main() -> i64 {
  slot $buf : obj<16x8>
  block ^entry:
    %base = addr $buf
    zeroinit 16x8 %base
    %elt = index i64 [projection=array_element] %base, 1
    store i64 42, %elt
    %v = load i64 %elt
    return i64 %v
}

function @main() -> i64 [role=entry] {
  slot $object : obj<8x8>
  block ^entry:
    store volatile obj<8x8> 1, $object
    return i64 0
}

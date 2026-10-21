function @main() -> i64 [role=entry] {
  slot $object : obj<8x8>
  block ^entry:
    %x = copy obj<8x8> 1
    store volatile obj<8x8> %x, $object
    return i64 0
}

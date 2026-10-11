function @main() -> i64 {
  block ^entry:
    jump ^mid1

  block ^mid1:
    jump ^mid2

  block ^mid2:
    return i64 5
}

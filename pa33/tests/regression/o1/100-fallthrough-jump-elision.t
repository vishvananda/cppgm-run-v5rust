function @main() -> i64 [role=entry] {
  block ^entry:
    jump ^next

  block ^next:
    return i64 7
}

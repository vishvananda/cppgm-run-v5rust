function @main() -> i64 {
  block ^entry:
    branch 1, ^keep, ^drop

  block ^keep:
    return i64 7

  block ^drop:
    return i64 9
}

function @main() -> i64 {
  block ^entry:
    jump ^exit
  block ^entry:
    return i64 0
  block ^exit:
    return i64 0
}

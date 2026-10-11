function @main() -> i64 [role=entry] {
  block ^entry:
    jump ^done

  block ^cold:
    return i64 1

  block ^done:
    return i64 0
}

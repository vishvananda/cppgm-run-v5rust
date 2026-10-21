function @main() -> i64 [role=entry] {
  block ^entry:
    switch 1.0, ^done, 1:^done
  block ^done:
    return i64 0
}

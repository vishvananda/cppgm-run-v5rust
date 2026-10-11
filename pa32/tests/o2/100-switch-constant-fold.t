function @main() -> i64 {
  block ^entry:
    switch 4, ^fallback, 1:^one, 4:^four

  block ^fallback:
    return i64 9

  block ^one:
    return i64 1

  block ^four:
    jump ^done

  block ^done:
    return i64 4
}

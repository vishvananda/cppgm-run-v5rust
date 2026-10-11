function @main() -> i64 {
  block ^entry:
    %which = const i64 1
    switch %which, %which, 0:^hit

  block ^hit:
    return i64 0
}

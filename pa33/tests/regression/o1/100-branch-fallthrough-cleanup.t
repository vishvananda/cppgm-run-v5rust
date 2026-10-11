function @main() -> i64 [role=entry] {
  block ^entry:
    %ok = const i64 1
    branch %ok, ^true, ^false

  block ^false:
    return i64 1

  block ^true:
    return i64 0
}

function @main() -> i64 {
  block ^entry:
    eh_try ^handler
    %0 = const i64 3
    eh_end
    return i64 %0

  block ^handler:
    %1 = exception i64
    return i64 %1
}

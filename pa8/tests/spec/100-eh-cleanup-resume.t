global @g : i64 = 0

function @main() -> i64 {
  block ^entry:
    eh_try ^handler
    eh_cleanup ^cleanup
    %0 = const i64 10
    throw i64 %0

  block ^cleanup:
    %1 = const i64 4
    store i64 %1, @g
    resume

  block ^handler:
    %2 = load i64 @g
    %3 = exception i64
    %4 = binary add i64 %2, %3
    return i64 %4
}

global @g : i64 = 0

function @main() -> i64 [role=entry] {
  block ^entry:
    eh_try ^handler
    eh_cleanup ^cleanup
    throw i64 10

  block ^cleanup:
    store i64 4, @g
    resume

  block ^handler:
    %g = load i64 @g
    %exception = exception i64
    %result = binary add i64 %g, %exception
    return i64 %result
}

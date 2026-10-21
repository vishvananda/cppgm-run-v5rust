function @test(%value : f64) -> i64 {
  block ^entry:
    branch %value, ^yes, ^no
  block ^yes:
    return i64 1
  block ^no:
    return i64 0
}

function @main() -> i64 [role=entry] {
  block ^entry:
    %positive_zero = call i64 @test(0.0)
    %negative_zero = call i64 @test(-0.0)
    %one = call i64 @test(1.0)
    %sum = binary add i64 %positive_zero, %negative_zero
    %total = binary add i64 %sum, %one
    %failed = cmp ne i64 %total, 2
    return i64 %failed
}

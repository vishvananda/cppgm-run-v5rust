function @main() -> i64 [role=entry] {
  block ^entry:
    %zero = call i64 @sum_to(0)
    %one = call i64 @sum_to(1)
    %six = call i64 @sum_to(6)
    %many = call i64 @sum_to(100)
    %bad0 = cmp ne i64 %zero, 0
    %bad1 = cmp ne i64 %one, 1
    %bad6 = cmp ne i64 %six, 21
    %bad100 = cmp ne i64 %many, 5050
    %bad01 = binary or i1 %bad0, %bad1
    %bad6100 = binary or i1 %bad6, %bad100
    %bad = binary or i1 %bad01, %bad6100
    %status = convert zext i64 i1 %bad
    return i64 %status
}

global @calls : i64 = 0

function @step(%x : i64) -> i64 {
  block ^entry:
    %count = load i64 @calls
    %next = binary add i64 %count, 1
    store i64 %next, @calls
    %triple = binary mul i64 %x, 3
    %value = binary add i64 %triple, 1
    return i64 %value
}

function @main() -> i64 [role=entry] {
  block ^entry:
    %step = addr @step
    %positive = call i64 @call_twice(%step, 2)
    %negative = call i64 @call_twice(%step, -1)
    %count = load i64 @calls
    %bad_positive = cmp ne i64 %positive, 22
    %bad_negative = cmp ne i64 %negative, -5
    %bad_count = cmp ne i64 %count, 4
    %bad_values = binary or i1 %bad_positive, %bad_negative
    %bad = binary or i1 %bad_values, %bad_count
    %status = convert zext i64 i1 %bad
    return i64 %status
}

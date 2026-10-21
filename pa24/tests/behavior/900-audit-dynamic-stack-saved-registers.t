function @identity(%value : i64) -> i64 {
  block ^entry:
    return i64 %value
}

function @allocate(%size : i64) -> i64 {
  block ^entry:
    %first = stack_alloc %size
    store i64 17, %first
    %second = stack_alloc 17
    store i64 25, %second
    %call = call i64 @identity(3)
    %a = load i64 %first
    %b = load i64 %second
    %sum = binary add i64 %a, %b
    %total = binary add i64 %sum, %call
    return i64 %total
}

function @main() -> i64 [role=entry] {
  block ^entry:
    %a = call i64 @allocate(31)
    %b = call i64 @allocate(64)
    %sum = binary add i64 %a, %b
    %failed = cmp ne i64 %sum, 90
    return i64 %failed
}

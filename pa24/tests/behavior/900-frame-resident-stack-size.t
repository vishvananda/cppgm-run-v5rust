function @allocate(%a : i64, %b : i64, %c : i64,
                   %d : i64, %e : i64, %f : i64, %bytes : i64) -> i64 {
  slot $size : i64
  block ^entry:
    %size_address = addr $size
    store volatile i64 %bytes, %size_address
    %previous = stack_alloc 16
    %memory = stack_alloc $size
    %distance = binary sub i64 %previous, %memory
    %bad_size = cmp ne i64 %distance, %bytes
    store volatile i64 73, %memory
    %value = load i64 %memory
    %bad_value = cmp ne i64 %value, 73
    %bad = binary or i64 %bad_size, %bad_value
    return i64 %bad
}
function @main() -> i64 [role=entry] {
  block ^entry:
    %actual = call i64 @allocate(1, 2, 3, 4, 5, 6, 32)
    %bad = cmp ne i64 %actual, 0
    return i64 %bad
}

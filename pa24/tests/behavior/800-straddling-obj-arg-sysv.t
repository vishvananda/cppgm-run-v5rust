function @read_pair(%a : i64, %b : i64, %c : i64, %d : i64, %e : i64, %pair : obj<16x8>) -> i64 {
  slot $pair : obj<16x8>

  block ^entry:
    %p = addr $pair
    copyobj 16x8 %pair, %p
    %lo = load i64 %p
    %hi_ptr = index i8 %p, 8
    %hi = load i64 %hi_ptr
    %sum = binary add i64 %lo, %hi
    return i64 %sum
}

function @main() -> i32 [role=entry] {
  slot $pair : obj<16x8>

  block ^entry:
    %p = addr $pair
    store i64 100, %p
    %hi_ptr = index i8 %p, 8
    store i64 23, %hi_ptr
    %r = call i64 @read_pair(1, 2, 3, 4, 5, $pair)
    %ok = cmp eq i64 %r, 123
    %bad = cmp eq i64 %ok, 0
    %exit = convert trunc i32 i64 %bad
    return i32 %exit
}

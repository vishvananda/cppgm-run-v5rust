global @g : i64 = 0

function @pressure(%seed : i64) -> i64 {
  block ^entry:
    %v1 = binary add i64 %seed, 1
    %v2 = binary add i64 %seed, 2
    %v3 = binary add i64 %seed, 3
    %v4 = binary add i64 %seed, 4
    %v5 = binary add i64 %seed, 5
    jump ^exchange

  block ^exchange:
    %source = binary add i64 %seed, 6
    %address = addr @g
    %old = atomic_exchange i64 %address, %source, 5
    branch %old, ^exchange, ^done

  block ^done:
    %s1 = binary add i64 %v1, %v2
    %s2 = binary add i64 %v3, %v4
    %s4 = binary add i64 %s1, %s2
    %result = binary add i64 %s4, %v5
    return i64 %result
}

function @main() -> i32 [role=entry] {
  block ^entry:
    %actual = call i64 @pressure(10)
    %failed = cmp ne i64 %actual, 65
    %exit = convert trunc i32 i64 %failed
    return i32 %exit
}

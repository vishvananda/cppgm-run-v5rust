global @g : i8 = 7

function @main() -> i64 [role=entry] {
  block ^entry:
    %pg = addr @g
    %v = atomic_load i8 %pg, 0
    %one = const i8 1
    %sum = binary add i8 %v, %one
    atomic_store i8 %sum, %pg, 0
    %check = atomic_load i8 %pg, 0
    %ok = cmp eq i8 %check, 8
    return i64 %ok
}

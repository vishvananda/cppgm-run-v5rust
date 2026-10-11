global @g : i64 = 7

function @main() -> i64 {
  block ^entry:
    %pg = addr @g
    %v = atomic_load i64 %pg, 0
    %one = const i64 1
    %sum = binary add i64 %v, %one
    atomic_store i64 %sum, %pg, 0
    %check = atomic_load i64 %pg, 0
    %eight = const i64 8
    %ok = cmp eq i64 %check, %eight
    return i64 %ok
}

global @g : i64 = 7

function @main() -> i64 [role=entry] {
  block ^entry:
    %pg = addr @g
    %nine = const i64 9
    %old = atomic_exchange i64 %pg, %nine, 5
    %now = atomic_load i64 %pg, 5
    %seven = const i64 7
    %ok_old = cmp eq i64 %old, %seven
    %ok_now = cmp eq i64 %now, %nine
    %ok = binary and i64 %ok_old, %ok_now
    return i64 %ok
}

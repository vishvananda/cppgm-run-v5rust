global @g : i32 = 7

function @main() -> i64 [role=entry] {
  block ^entry:
    %pg = addr @g
    %eleven = const i32 11
    %old = atomic_exchange i32 %pg, %eleven, 5
    %now = atomic_load i32 %pg, 5
    %ok_old = cmp eq i32 %old, 7
    %ok_now = cmp eq i32 %now, 11
    %ok = binary and i64 %ok_old, %ok_now
    return i64 %ok
}

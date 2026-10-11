global @g : i32 = 7

function @main() -> i64 [role=entry] {
  block ^entry:
    %pg = addr @g
    %eleven = const i32 11
    atomic_store i32 %eleven, %pg, 5
    %now = atomic_load i32 %pg, 5
    %ok = cmp eq i32 %now, 11
    return i64 %ok
}

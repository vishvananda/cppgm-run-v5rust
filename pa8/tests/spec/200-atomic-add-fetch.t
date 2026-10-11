global @g : i64 = 7

function @main() -> i64 {
  block ^entry:
    %pg = addr @g
    %delta = const i64 5
    %new = atomic_add_fetch i64 %pg, %delta, 5
    %twelve = const i64 12
    %ok = cmp eq i64 %new, %twelve
    return i64 %ok
}

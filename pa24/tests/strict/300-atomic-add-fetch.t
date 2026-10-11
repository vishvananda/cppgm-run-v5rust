global @g : i64 = 7

function @main() -> i64 [role=entry] {
  block ^entry:
    %pg = addr @g
    %delta = const i64 5
    %new = atomic_add_fetch i64 %pg, %delta, 5
    %twelve = const i64 12
    %ok = cmp eq i64 %new, %twelve
    %stored = atomic_load i64 %pg, 5
    %stored_ok = cmp eq i64 %stored, %twelve
    %both_ok = binary and i1 %ok, %stored_ok
    return i64 %both_ok
}

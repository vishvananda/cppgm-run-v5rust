global @g : i128 = 7

function @main() -> i64 [role=entry] {
  slot $expected : i128

  block ^entry:
    %pg = addr @g
    %loaded = atomic_load i128 %pg, 5
    store i128 %loaded, $expected
    %expectedp = addr $expected
    %nine = const i128 9
    %ok = atomic_compare_exchange i128 %pg, %expectedp, %nine, 5, 2
    %now = atomic_load i128 %pg, 5
    %wanted = const i128 9
    %is_nine = cmp eq i128 %now, %wanted
    %all = binary and i64 %ok, %is_nine
    return i64 %all
}

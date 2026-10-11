global @g : i64 = 7

function @main() -> i64 {
  slot $expected : i64

  block ^entry:
    %pg = addr @g
    %seven = const i64 7
    store i64 %seven, $expected
    %expectedp = addr $expected
    %nine = const i64 9
    %ok = atomic_compare_exchange i64 %pg, %expectedp, %nine, 5, 2
    %now = atomic_load i64 %pg, 5
    %seen = load i64 $expected
    %one = const i64 1
    %ok1 = cmp eq i64 %ok, %one
    %ok2 = cmp eq i64 %now, %nine
    %ok3 = cmp eq i64 %seen, %seven
    %tmp = binary and i64 %ok1, %ok2
    %all = binary and i64 %tmp, %ok3
    return i64 %all
}

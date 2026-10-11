global @bytes = {
  i8 11
  i8 22
  i8 33
  i8 44
}

function @touch(%p : ptr, %a : i64, %b : i64, %c : i64, %d : i64, %e : i64) -> void {
  block ^entry:
    return void
}

function @main() -> i64 [role=entry] {
  block ^entry:
    %k1 = const i64 1
    %k2 = const i64 2
    %k3 = const i64 3
    %k4 = const i64 4
    %k5 = const i64 5
    %base = addr @bytes
    %p = index i8 %base, 2
    call void @touch(%p, %k1, %k2, %k3, %k4, %k5)
    %base2 = addr @bytes
    %expected = index i8 %base2, 2
    %same = cmp eq ptr %p, %expected
    branch %same, ^good, ^bad

  block ^good:
    %s12 = binary add i64 %k1, %k2
    %s123 = binary add i64 %s12, %k3
    %s1234 = binary add i64 %s123, %k4
    %s12345 = binary add i64 %s1234, %k5
    %sum_ok = cmp eq i64 %s12345, 15
    %ret = cmp eq i64 %sum_ok, 0
    return i64 %ret

  block ^bad:
    return i64 1
}

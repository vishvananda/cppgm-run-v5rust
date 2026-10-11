function @main() -> i64 [role=entry] {
  slot $i : i64
  slot $sum : i64

  block ^entry:
    store i64 1, $i
    store i64 0, $sum
    jump ^cond

  block ^cond:
    %old = load i64 $i
    %next = binary sub i64 %old, 1
    store i64 %next, $i
    %more = cmp gt i64 %old, 0
    branch %more, ^body, ^end

  block ^body:
    %sum0 = load i64 $sum
    %sum1 = binary add i64 %sum0, 17
    store i64 %sum1, $sum
    jump ^cond

  block ^end:
    %result = load i64 $sum
    %ok = cmp eq i64 %result, 17
    branch %ok, ^pass, ^fail

  block ^pass:
    return i64 0

  block ^fail:
    return i64 1
}

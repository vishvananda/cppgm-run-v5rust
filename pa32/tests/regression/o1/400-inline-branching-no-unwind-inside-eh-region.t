function @choose(%x : i64) -> i64 [unwind=no, prefer_local=yes] {
  block ^entry:
    %t1 = cmp eq i64 %x, 0
    branch %t1, ^zero, ^nonzero

  block ^zero:
    return i64 10

  block ^nonzero:
    return i64 20
}

function @caller(%x : i64) -> i64 {
  block ^entry:
    eh_try ^dispatch
    %t1 = call i64 @choose(%x)
    eh_end
    %t2 = binary add i64 %t1, 1
    return i64 %t2

  block ^dispatch:
    resume
}

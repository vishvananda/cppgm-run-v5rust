function @leaf(%x : i64) -> i64 [unwind=no] {
  block ^entry:
    %t1 = binary add i64 %x, 1
    return i64 %t1
}

function @wrapper(%x : i64) -> i64 [unwind=no] {
  block ^entry:
    eh_try ^dispatch
    %t1 = call i64 @leaf(%x)
    jump ^finish

  block ^dispatch:
    resume

  block ^finish:
    eh_end
    return i64 %t1
}

function @main(%x : i64) -> i64 {
  block ^entry:
    %t1 = call i64 @wrapper(%x)
    %t2 = binary mul i64 %t1, 2
    return i64 %t2
}

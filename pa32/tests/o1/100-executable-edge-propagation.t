function @main(%x : i64, %y : i64) -> i64 {
  block ^entry:
    %a = binary add i64 %x, %y
    jump ^head

  block ^head:
    %b = binary add i64 %x, %y
    %c = cmp eq i64 %b, %a
    branch %c, ^exit, ^body

  block ^body:
    %d = binary add i64 %x, %y
    %e = cmp eq i64 %d, %a
    branch %e, ^head, ^exit

  block ^exit:
    return i64 1
}

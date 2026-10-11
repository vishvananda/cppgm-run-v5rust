function @id(%x : i64) -> i64 {
  slot $xslot : i64

  block ^entry:
    store i64 %x, $xslot
    jump ^body

  block ^body:
    %y = load i64 $xslot
    %z = binary add i64 %y, 1
    return i64 %z
}

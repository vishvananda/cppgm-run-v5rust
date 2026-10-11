function @touch(%p : ptr) -> void {
  block ^entry:
    return void
}
function @main(%x : i64) -> i64 {
  slot $xslot : i64

  block ^entry:
    store i64 %x, $xslot
    %p = addr $xslot
    call void @touch(%p)
    %y = load i64 $xslot
    return i64 %y
}

declare function @observe(%value : i64) -> void [unwind=no]

function @below_zero(%x : i64) -> i64 [binding=strong] {
  block ^entry:
    %r = cmp ult i64 %x, 0
    return i64 %r
}

function @zero_above(%x : i64) -> i64 [binding=strong] {
  block ^entry:
    %r = cmp ugt i64 0, %x
    return i64 %r
}

function @at_least_zero(%x : i64) -> i64 [binding=strong] {
  block ^entry:
    %r = cmp uge i64 %x, 0
    return i64 %r
}

function @zero_at_most(%x : i64) -> i64 [binding=strong] {
  block ^entry:
    %r = cmp ule i64 0, %x
    return i64 %r
}

function @signed_stays(%x : i64) -> i64 [binding=strong] {
  block ^entry:
    %r = cmp lt i64 %x, 0
    return i64 %r
}

function @guard_folds(%pos : i64, %size : i64) -> void [binding=strong] {
  block ^entry:
    %outside = cmp ugt i64 0, %size
    branch %outside, ^reject, ^accept

  block ^reject:
    call void @observe(-1)
    return void

  block ^accept:
    call void @observe(%size)
    return void
}

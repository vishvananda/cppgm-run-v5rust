declare function @observe(%value : i64) -> void [unwind=no]
declare function @risky() -> void

global @bang_type : i8 = 0

function @fold_through_jump_and_unrelated_branch(%flag : i64, %other : i64) -> i64 {
  block ^entry:
    branch %flag, ^taken, ^skipped

  block ^taken:
    call void @observe(1)
    jump ^hop

  block ^hop:
    branch %other, ^again, ^retest

  block ^again:
    call void @observe(2)
    return i64 5

  block ^retest:
    branch %flag, ^known, ^impossible

  block ^known:
    return i64 7

  block ^impossible:
    return i64 99

  block ^skipped:
    return i64 3
}

function @keep_when_chain_merges(%flag : i64, %other : i64) -> i64 {
  block ^entry:
    branch %flag, ^taken, ^skipped

  block ^taken:
    call void @observe(1)
    jump ^merge

  block ^skipped:
    call void @observe(2)
    jump ^merge

  block ^merge:
    call void @observe(3)
    jump ^retest

  block ^retest:
    branch %flag, ^left, ^right

  block ^left:
    return i64 7

  block ^right:
    return i64 3
}

function @keep_across_landing_pad(%flag : i64) -> i64 {
  block ^entry:
    branch %flag, ^taken, ^skipped

  block ^taken:
    eh_try ^catch_dispatch
    call void @risky()
    eh_end
    return i64 1

  block ^catch_dispatch:
    eh_catch @bang_type, 1
    %selector = exception_selector i32
    %matched = cmp eq i32 %selector, 1
    branch %matched, ^catch_body, ^catch_next

  block ^catch_body:
    jump ^retest

  block ^retest:
    branch %flag, ^left, ^right

  block ^left:
    return i64 7

  block ^right:
    return i64 3

  block ^catch_next:
    resume

  block ^skipped:
    return i64 3
}

function @keep_entry_back_edge(%p : i64) -> i64 {
  block ^entry:
    branch %p, ^exit, ^body

  block ^body:
    call void @observe(1)
    branch %p, ^entry, ^other

  block ^other:
    return i64 3

  block ^exit:
    return i64 7
}

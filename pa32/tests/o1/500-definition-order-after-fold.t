declare function @observe(%value : i64) -> void [unwind=no]

function @late_block_becomes_dominator(%x : ptr, %n : i64) -> i64 {
  block ^entry:
    %one = copy i64 1
    branch %one, ^late, ^early

  block ^early:
    %q = index i8 %x, 8
    %v = load i64 %q
    jump ^loop

  block ^loop:
    %i = phi i64 [^early: 0, ^loop: %next]
    %next = binary add i64 %i, %v
    %done = cmp eq i64 %next, %n
    branch %done, ^exit, ^loop

  block ^exit:
    return i64 %next

  block ^late:
    %p = index i8 %x, 8
    %w = load i64 %p
    call void @observe(%w)
    jump ^early
}

declare function @observe(%value : i64) -> void [unwind=no]

function @constant_branch_drops_phi_input(%n : i64, %x : i64) -> i64 {
  block ^entry:
    jump ^head

  block ^head:
    %i = phi i64 [^entry: 0, ^latch: %next]
    %acc = phi i64 [^entry: 0, ^latch: %acc2]
    %done = cmp eq i64 %i, %n
    branch %done, ^exit, ^body

  block ^body:
    branch 1, ^keep, ^drop

  block ^keep:
    call void @observe(%i)
    jump ^latch

  block ^drop:
    call void @observe(99)
    jump ^latch

  block ^latch:
    %inc = phi i64 [^keep: 1, ^drop: 2]
    %next = binary add i64 %i, %inc
    %acc2 = binary add i64 %acc, %x
    jump ^head

  block ^exit:
    return i64 %acc
}

function @constant_switch_erases_region(%x : i64) -> i64 {
  block ^entry:
    %two = copy i64 2
    switch %two, ^other, 1:^one, 2:^two

  block ^one:
    call void @observe(1)
    jump ^dead_join

  block ^other:
    call void @observe(3)
    jump ^dead_join

  block ^dead_join:
    %d = phi i64 [^one: 1, ^other: 3]
    %e = binary add i64 %d, %x
    jump ^join

  block ^two:
    call void @observe(2)
    jump ^join

  block ^join:
    %r = phi i64 [^dead_join: %e, ^two: %x]
    return i64 %r
}

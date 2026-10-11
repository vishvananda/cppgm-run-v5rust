declare function @observe(%value : i64) -> void [unwind=no]

function @empty_diamond_in_loop(%n : i64, %flag : u8) -> i64 {
  block ^entry:
    %c = cmp ne u8 %flag, 0
    jump ^head

  block ^head:
    %i = phi i64 [^entry: 0, ^latch: %next]
    %sum = phi i64 [^entry: 0, ^latch: %sum2]
    %done = cmp eq i64 %i, %n
    branch %done, ^exit, ^body

  block ^body:
    branch %c, ^left, ^right

  block ^left:
    jump ^join

  block ^right:
    jump ^join

  block ^join:
    %sum2 = binary add i64 %sum, %i
    %next = binary add i64 %i, 1
    jump ^latch

  block ^latch:
    jump ^head

  block ^exit:
    return i64 %sum
}

function @distinct_arm_values_stay(%flag : u8, %x : i64, %y : i64) -> i64 {
  block ^entry:
    %c = cmp ne u8 %flag, 0
    branch %c, ^left, ^right

  block ^left:
    jump ^join

  block ^right:
    jump ^join

  block ^join:
    %r = phi i64 [^left: %x, ^right: %y]
    return i64 %r
}

function @one_arm_same_value(%flag : u8, %x : i64) -> i64 {
  block ^entry:
    %c = cmp ne u8 %flag, 0
    branch %c, ^left, ^join

  block ^left:
    jump ^join

  block ^join:
    %r = phi i64 [^left: %x, ^entry: %x]
    return i64 %r
}

function @jump_chain_into_phi(%flag : u8, %x : i64, %y : i64) -> i64 {
  block ^entry:
    %c = cmp ne u8 %flag, 0
    branch %c, ^left, ^right

  block ^left:
    call void @observe(%x)
    jump ^left_hop

  block ^left_hop:
    jump ^join

  block ^right:
    call void @observe(%y)
    jump ^right_hop

  block ^right_hop:
    jump ^join

  block ^join:
    %r = phi i64 [^left_hop: %x, ^right_hop: %y]
    return i64 %r
}

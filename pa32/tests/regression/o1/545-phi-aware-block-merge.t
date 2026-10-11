declare function @observe(%value : i64) -> void [unwind=no]
declare function @risky() -> void

global @bang_type : i8 = 0

function @merge_chain_renames_phi_input(%n : i64, %x : i64) -> i64 {
  block ^entry:
    jump ^head

  block ^head:
    %i = phi i64 [^entry: 0, ^step2: %next]
    %done = cmp eq i64 %i, %n
    branch %done, ^exit, ^step1

  block ^step1:
    call void @observe(%i)
    jump ^step2

  block ^step2:
    %next = binary add i64 %i, 1
    jump ^head

  block ^exit:
    return i64 %i
}

function @single_input_phi_becomes_copy(%x : i64) -> i64 {
  block ^entry:
    %y = binary add i64 %x, 1
    jump ^cont

  block ^cont:
    %z = phi i64 [^entry: %y]
    %w = binary add i64 %z, %x
    return i64 %w
}

function @keep_two_predecessors(%flag : i64, %x : i64) -> i64 {
  block ^entry:
    branch %flag, ^left, ^right

  block ^left:
    call void @observe(1)
    jump ^join

  block ^right:
    call void @observe(2)
    jump ^join

  block ^join:
    %r = phi i64 [^left: %x, ^right: 0]
    return i64 %r
}

function @keep_landing_pad(%x : i64) -> i64 {
  block ^entry:
    eh_try ^pad
    call void @risky()
    eh_end
    jump ^after

  block ^after:
    return i64 %x

  block ^pad:
    eh_catch @bang_type, 1
    %selector = exception_selector i32
    %matched = cmp eq i32 %selector, 1
    branch %matched, ^handled, ^next

  block ^handled:
    return i64 0

  block ^next:
    resume
}

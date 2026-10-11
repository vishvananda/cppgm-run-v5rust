function @construct_at_end(%buffer : ptr, %n : i64, %value : ptr) -> void {
  block ^entry:
    %end_slot = index i8 %buffer, 8
    %end = load ptr %end_slot
    %new_end = index i8 %end, %n
    jump ^cond

  block ^cond:
    %p = phi ptr [^entry: %end, ^body: %next]
    %more = cmp ne ptr %p, %new_end
    branch %more, ^body, ^done

  block ^body:
    %v = load u8 %value
    %dst = copy ptr %p
    store u8 %v, %dst
    %next = index i8 %p, 1
    jump ^cond

  block ^done:
    store ptr %new_end, %end_slot
    return void
}

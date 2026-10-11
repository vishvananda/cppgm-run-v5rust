function @destruct_at_end(%buffer : ptr, %new_last : ptr) -> void {
  block ^entry:
    %end_slot = index i8 %buffer, 8
    %end = load ptr %end_slot
    jump ^cond

  block ^cond:
    %p = phi ptr [^entry: %end, ^body: %prev]
    %more = cmp ne ptr %new_last, %p
    branch %more, ^body, ^done

  block ^body:
    %prev = index i8 %p, -1
    jump ^cond

  block ^done:
    store ptr %new_last, %end_slot
    return void
}

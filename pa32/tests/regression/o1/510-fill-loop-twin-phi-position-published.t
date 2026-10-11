function @construct_at_end(%vector : ptr, %n : i64, %value : ptr) -> void {
  block ^entry:
    %end_field = index i8 [projection=field] %vector, 8
    %end = load ptr %end_field
    %new_end = index i8 %end, %n
    jump ^cond

  block ^cond:
    %position = phi ptr [^entry: %end, ^body: %next]
    %p = phi ptr [^entry: %end, ^body: %next]
    %more = cmp ne ptr %p, %new_end
    branch %more, ^body, ^done

  block ^body:
    %dst = copy ptr %p
    %v = load u8 %value
    store u8 %v, %dst
    %next = index i8 %p, 1
    jump ^cond

  block ^done:
    store ptr %position, %end_field
    return void
}

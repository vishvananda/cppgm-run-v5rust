function @fill_range(%first : ptr, %last : ptr) -> void {
  block ^entry:
    jump ^cond

  block ^cond:
    %p = phi ptr [^entry: %first, ^body: %next]
    %more = cmp ne ptr %p, %last
    branch %more, ^body, ^done

  block ^body:
    store u8 0, %p
    %next = index i8 %p, 1
    jump ^cond

  block ^done:
    return void
}

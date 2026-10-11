function @pattern_words(%first : ptr, %n : i64) -> void {
  block ^entry:
    %bytes = binary shl i64 %n, 2
    %last = index i8 %first, %bytes
    jump ^cond

  block ^cond:
    %p = phi ptr [^entry: %first, ^body: %next]
    %more = cmp ne ptr %p, %last
    branch %more, ^body, ^done

  block ^body:
    store u32 258, %p
    %next = index i8 %p, 4
    jump ^cond

  block ^done:
    return void
}

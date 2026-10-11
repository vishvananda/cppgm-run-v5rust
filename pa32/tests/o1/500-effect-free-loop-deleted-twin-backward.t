function @destroy_backward(%begin : ptr, %end : ptr) -> ptr {
  block ^entry:
    jump ^cond

  block ^cond:
    %walk = phi ptr [^entry: %end, ^body: %prev]
    %twin = phi ptr [^entry: %end, ^body: %prev]
    %more = cmp ne ptr %begin, %twin
    branch %more, ^body, ^done

  block ^body:
    %prev = index i8 %walk, -8
    jump ^cond

  block ^done:
    return ptr %begin
}

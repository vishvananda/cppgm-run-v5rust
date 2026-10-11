function @id(%p : ptr) -> ptr {
  slot $pslot : ptr

  block ^entry:
    store ptr %p, $pslot
    jump ^body

  block ^body:
    %q = load ptr $pslot
    return ptr %q
}

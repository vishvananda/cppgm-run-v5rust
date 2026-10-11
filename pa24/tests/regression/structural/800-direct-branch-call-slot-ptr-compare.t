global @cells = {
  i64 11
  i64 13
}

function @clobber(%a : i64, %b : i64, %c : i64, %d : i64, %e : i64, %f : i64) -> void {
  block ^entry:
    return void
}

function @main() -> i64 [role=entry] {
  slot $first : ptr
  slot $last : ptr

  block ^entry:
    %base = addr @cells
    store ptr %base, $first
    store ptr %base, $last
    call void @clobber(10, 20, 30, 40, 50, 60)
    %cur = load ptr $first
    %end = load ptr $last
    %done = cmp eq ptr %cur, %end
    branch %done, ^done, ^body

  block ^body:
    return i64 1

  block ^done:
    return i64 0
}

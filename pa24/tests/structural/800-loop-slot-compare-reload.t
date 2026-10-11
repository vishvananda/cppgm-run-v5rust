global @cells = {
  i64 11
  i64 13
  i64 17
}

function @main() -> i64 [role=entry] {
  slot $first : ptr
  slot $last : ptr

  block ^entry:
    %base = addr @cells
    store ptr %base, $first
    %lastp = index i64 %base, 2
    store ptr %lastp, $last
    jump ^loop

  block ^loop:
    %cur = load ptr $first
    %done = cmp eq ptr %cur, $last
    branch %done, ^done, ^body

  block ^body:
    %next = index i64 %cur, 1
    store ptr %next, $first
    jump ^loop

  block ^done:
    return i64 0
}

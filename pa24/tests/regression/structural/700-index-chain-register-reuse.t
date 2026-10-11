global @cells = {
  i64 11
  i64 13
  i64 17
}

function @main() -> i64 [role=entry] {
  block ^entry:
    %base = addr @cells
    %next = index i64 %base, 1
    %next2 = index i64 %next, 1
    %value = load i64 %next2
    %ok = cmp eq i64 %value, 17
    return i64 %ok
}

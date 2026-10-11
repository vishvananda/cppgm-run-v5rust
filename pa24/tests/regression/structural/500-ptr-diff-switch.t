global @cells = {
  i64 11
  i64 13
  i64 17
  i64 19
}

function @main() -> i64 [role=entry] {
  block ^entry:
    %base = addr @cells
    %last = index i64 %base, 3
    %delta = binary sub ptr %last, %base
    switch %delta, ^default, 24:^case_three

  block ^case_three:
    return i64 0

  block ^default:
    return i64 1
}

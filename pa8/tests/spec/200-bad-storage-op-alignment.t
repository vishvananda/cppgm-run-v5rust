global @src = {
  i64 5
}

global @dst = {
  i64 0
}

function @main() -> i64 {
  block ^entry:
    %0 = addr @dst
    %1 = addr @src
    copyobj 8x3 %1, %0
    return i64 0
}

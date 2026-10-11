function @pick() -> i32 {
  block ^entry:
    return i32 -7
}

function @spoil() -> i32 {
  block ^entry:
    return i32 9
}

function @classify() -> i32 {
  block ^entry:
    %selector = call i32 @pick()
    %ignored = call i32 @spoil()
    switch %selector, ^default, -7:^hit, 9:^wrong

  block ^hit:
    return i32 0

  block ^wrong:
    return i32 2

  block ^default:
    return i32 1
}

function @main() -> i32 [role=entry, binding=strong] {
  block ^entry:
    %result = call i32 @classify()
    return i32 %result
}

function @mark() -> void {
  block ^entry:
    return void
}

function @sink(%a : ptr, %b : ptr) -> void {
  block ^entry:
    store i64 1, %a
    store i64 2, %b
    return void
}

function @f() -> i64 {
  slot $x : i64
  slot $y : i64

  block ^entry:
    %t1 = addr $x
    %t2 = addr $y
    call void @mark()
    call void @sink(%t1, %t2)
    %u = load i64 $x
    %v = load i64 $y
    %w = binary add i64 %u, %v
    return i64 %w
}

function @main() -> i64 [role=entry] {
  block ^entry:
    %r = call i64 @f()
    return i64 %r
}

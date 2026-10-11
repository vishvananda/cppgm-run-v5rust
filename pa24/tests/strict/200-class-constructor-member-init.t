function @YP__YP(%this : ptr) -> void {
  slot $this : ptr

  block ^entry:
    store ptr %this, $this
    %t1 = load ptr $this
    store i32 3, %t1
    return void
}
function @YP___YP(%this : ptr) -> void {
  slot $this : ptr

  block ^entry:
    store ptr %this, $this
    return void
}
function @main() -> i64 [role=entry] {
  slot $p__0 : i64

  block ^entry:
    %t1 = addr $p__0
    call void @YP__YP(%t1)
    %t2 = addr $p__0
    %t3 = load i32 %t2
    %t4 = addr $p__0
    call void @YP___YP(%t4)
    return i64 %t3
}
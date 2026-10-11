function @main() -> i64 [role=entry] {
  slot $b__0 : i64

  block ^entry:
    %t1 = addr $b__0
    call void @Box_3___Box(%t1)
    %t2 = addr $b__0
    %t3 = call i64 @Box_3___get(%t2)
    %t4 = addr $b__0
    call void @Box_3____Box(%t4)
    return i64 %t3
}
function @Box_3___get(%this : ptr) -> i64 {
  slot $this : ptr

  block ^entry:
    store ptr %this, $this
    return i64 5
}
function @Box_3___Box(%this : ptr) -> void {
  slot $this : ptr

  block ^entry:
    store ptr %this, $this
    return void
}
function @Box_3____Box(%this : ptr) -> void {
  slot $this : ptr

  block ^entry:
    store ptr %this, $this
    return void
}
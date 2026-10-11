function @main() -> i64 [role=entry] {
  slot $box__0 : i64

  block ^entry:
    %t1 = addr $box__0
    call void @Box_int___Box(%t1)
    %t2 = addr $box__0
    store i32 7, %t2
    %t3 = addr $box__0
    %t4 = load i32 %t3
    %t5 = binary sub i64 %t4, 7
    %t6 = addr $box__0
    call void @Box_int____Box(%t6)
    return i64 %t5
}
function @Box_int___Box(%this : ptr) -> void {
  slot $this : ptr

  block ^entry:
    store ptr %this, $this
    return void
}
function @Box_int____Box(%this : ptr) -> void {
  slot $this : ptr

  block ^entry:
    store ptr %this, $this
    return void
}

function @Y__Y(%this : ptr) -> void {
  slot $this : ptr

  block ^entry:
    store ptr %this, $this
    return void
}
function @Y___Y(%this : ptr) -> void {
  slot $this : ptr

  block ^entry:
    store ptr %this, $this
    return void
}
function @id(%y : ptr) -> i64 {
  slot $y__0 : i64

  block ^entry:
    %t1 = addr $y__0
    copyobj 4 %y, %t1
    %t2 = addr $y__0
    %t3 = load i32 %t2
    return i64 %t3
}
function @main() -> i64 [role=entry] {
  slot $a__0 : i64

  block ^entry:
    %t1 = addr $a__0
    call void @Y__Y(%t1)
    %t2 = addr $a__0
    store i32 8, %t2
    %t3 = addr $a__0
    %t4 = call i64 @id(%t3)
    %t5 = addr $a__0
    call void @Y___Y(%t5)
    return i64 %t4
}

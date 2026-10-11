global @first = {
  i64 0
}
global @second = {
  i64 0
}
global @third = {
  i64 0
}
global @fourth = {
  i64 0
}

function @target(%ret : ptr [pass=indirect_result], %this : ptr, %class : ptr, %index : i32, %base : u8, %address : ptr, %vector : ptr) -> void {
  block ^entry:
    %first = addr @first
    %second = addr @second
    %third = addr @third
    %fourth = addr @fourth
    %wrong1 = cmp ne ptr %this, %first
    %wrong2 = cmp ne ptr %class, %second
    %wrong3 = cmp ne i32 %index, 7
    %wrong4 = cmp ne u8 %base, 1
    %wrong5 = cmp ne ptr %address, %third
    %wrong6 = cmp ne ptr %vector, %fourth
    %sum1 = binary add i64 %wrong1, %wrong2
    %sum2 = binary add i64 %sum1, %wrong3
    %sum3 = binary add i64 %sum2, %wrong4
    %sum4 = binary add i64 %sum3, %wrong5
    %sum5 = binary add i64 %sum4, %wrong6
    store i64 %sum5, %ret
    return void
}

function @forward(%ret : ptr [pass=indirect_result], %arg1 : ptr, %arg2 : ptr, %arg3 : i32, %arg4 : u8, %arg5 : ptr, %arg6 : ptr) -> void {
  block ^entry:
    %this = index i8 %arg1, -136
    call void @target(%ret, %this, %arg2, %arg3, %arg4, %arg5, %arg6)
    return void
}

function @main() -> i64 [role=entry] {
  slot $result : i64

  block ^entry:
    %result = addr $result
    %first = addr @first
    %adjusted = index i8 %first, 136
    %second = addr @second
    %third = addr @third
    %fourth = addr @fourth
    call void @forward(%result, %adjusted, %second, 7, 1, %third, %fourth)
    %value = load i64 $result
    return i64 %value
}

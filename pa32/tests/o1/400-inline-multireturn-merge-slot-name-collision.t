function @callee(%x : i64, %take_x : u8) -> i64 {
  slot $__o1inl0__retmerge__1 : i64

  block ^entry:
    store i64 %x, $__o1inl0__retmerge__1
    branch %take_x, ^take_x, ^increment

  block ^take_x:
    %t1 = load i64 $__o1inl0__retmerge__1
    return i64 %t1

  block ^increment:
    %t2 = binary add i64 %x, 1
    return i64 %t2
}
function @caller(%x : i64, %take_x : u8) -> i64 {
  block ^entry:
    %t1 = call i64 @callee(%x, %take_x)
    return i64 %t1
}
function @object_callee(%p : ptr, %take_p : u8) -> obj<8x8> {
  slot $__o1inl0__retmergeobj__1 : obj<8x8>

  block ^entry:
    branch %take_p, ^take_p, ^take_null

  block ^take_p:
    %t1 = addr $__o1inl0__retmergeobj__1
    store ptr %p, %t1
    return obj<8x8> $__o1inl0__retmergeobj__1

  block ^take_null:
    %t2 = addr $__o1inl0__retmergeobj__1
    store ptr 0, %t2
    return obj<8x8> $__o1inl0__retmergeobj__1
}
function @object_caller(%p : ptr, %take_p : u8, %out : ptr) -> void {
  block ^entry:
    %t1 = call obj<8x8> @object_callee(%p, %take_p)
    copyobj 8x8 %t1, %out
    return void
}

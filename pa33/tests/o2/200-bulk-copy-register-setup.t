global @src = {
  i64 0
  i64 9
}

global @dst = {
  i64 1
  i64 1
}

global @src_ptr = {
  ptr addr @src
}

global @dst_ptr = {
  ptr addr @dst
}

function @main() -> i64 [role=entry] {
  block ^entry:
    %destination = load ptr @dst_ptr
    %source = load ptr @src_ptr
    copyobj 16x8 %source, %destination
    %result = load i64 @dst
    return i64 %result
}

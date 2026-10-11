global @g readonly : i64 [binding=strong, object=_Z1g] = 7

function @main() -> i32 [role=entry, binding=strong, keep_alias=yes] {
  block ^entry:
    %t1 = load i64 @g
    %t2 = binary and i64 %t1, 4294967295
    %t3 = copy i32 %t2
    return i32 %t3
}

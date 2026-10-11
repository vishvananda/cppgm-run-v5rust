global @g : i32 [binding=strong, object=_Z1g] = 7

function @main() -> i32 [role=entry, binding=strong, keep_alias=yes] {
  block ^entry:
    %t1 = load i32 @g
    return i32 %t1
}

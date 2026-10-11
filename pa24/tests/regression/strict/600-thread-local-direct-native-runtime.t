declare function @g__tls_wrapper() -> ptr [binding=strong, object=_ZTW1g, tls_for=@g]
global @g : i64 [storage=thread_local, binding=strong, object=_Z1g] = 0

function @main() -> i32 [role=entry, binding=strong, keep_alias=yes] {
  block ^entry:
    %t1 = copy i64 7
    store i64 %t1, @g
    %t2 = load i64 @g
    %t3 = binary and i64 %t2, 4294967295
    %t4 = copy i32 %t3
    return i32 %t4
}

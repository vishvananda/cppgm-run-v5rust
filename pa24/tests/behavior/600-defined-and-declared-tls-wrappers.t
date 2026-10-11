global @g : i64 [storage=thread_local] = 0
global @h : i64 [storage=thread_local] = 9
declare function @h_wrapper() -> ptr [tls_for=@h]

function @g_wrapper() -> ptr [tls_for=@g] {
  block ^entry:
    store i64 5, @g
    %address = addr @g
    return ptr %address
}

function @main() -> i64 [role=entry] {
  block ^entry:
    %pg = call ptr @g_wrapper()
    %ph = call ptr @h_wrapper()
    %x = load i64 %pg
    %y = load i64 %ph
    %sum = binary add i64 %x, %y
    return i64 %sum
}

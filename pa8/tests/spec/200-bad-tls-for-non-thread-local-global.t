declare function @g__tls_wrapper() -> ptr [tls_for=@g]
global @g : i64 = 7

function @main() -> i64 [role=entry] {
  block ^entry:
    %0 = load i64 @g
    return i64 %0
}

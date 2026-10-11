declare global @tls_ext : i32 [storage=thread_local]
global @tls_value : i64 [storage=thread_local] = 7

function @main() -> i64 [role=entry] {
  block ^entry:
    %0 = load i64 @tls_value
    return i64 %0
}

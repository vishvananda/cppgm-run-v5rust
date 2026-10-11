global @tls : i64 [storage=thread_local] = 0

declare function @first() -> ptr [tls_for=@tls]
declare function @second() -> ptr [tls_for=@tls]

function @main() -> i64 {
  block ^entry:
    return i64 0
}

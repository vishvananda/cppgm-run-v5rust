declare global @ext : ptr [linkage=c]
declare function @helper(%arg0 : ptr) -> void [linkage=c]

function @main() -> i64 [role=entry, linkage=c] {
  block ^entry:
    %0 = const i64 0
    return i64 %0
}

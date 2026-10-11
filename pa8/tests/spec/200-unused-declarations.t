declare global @ext
declare global @typed_ext : ptr
declare function @helper(%arg0 : ptr) -> void

function @main() -> i64 {
  block ^entry:
    %0 = const i64 0
    return i64 %0
}

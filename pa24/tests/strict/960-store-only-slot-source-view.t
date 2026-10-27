# Compile the frozen raw slot-value view without reading indeterminate data.
function @uncalled() -> ptr {
  slot $source : i64
  slot $destination : ptr
  block ^entry:
    store ptr $source, $destination
    %result = load ptr $destination
    return ptr %result
}
function @main() -> i64 [role=entry] {
  block ^entry:
    return i64 0
}

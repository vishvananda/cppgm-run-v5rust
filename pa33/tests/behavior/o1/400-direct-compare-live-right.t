global @edge = {
  u32 1
}
global @sentinel = {
  u32 4294967295
}
function @sink() -> void {
  block ^entry:
    return void
}
function @probe(%p0 : i64, %p1 : i64, %p2 : i64, %p3 : i64, %p4 : i64) -> i64 {
  block ^entry:
    call void @sink()
    %a = load u32 @edge
    %b = load u32 @sentinel
    %different = cmp ne u32 %a, %b
    branch %different, ^good, ^bad
  block ^good:
    call void @sink()
    %correct = cmp eq u32 %b, 4294967295
    branch %correct, ^done, ^bad
  block ^done:
    %s0 = binary add i64 %p0, %p1
    %s1 = binary add i64 %s0, %p2
    %s2 = binary add i64 %s1, %p3
    %s3 = binary add i64 %s2, %p4
    %status = cmp ne i64 %s3, 15
    return i64 %status
  block ^bad:
    return i64 1
}
function @main() -> i64 [role=entry] {
  block ^entry:
    %status = call i64 @probe(1, 2, 3, 4, 5)
    return i64 %status
}

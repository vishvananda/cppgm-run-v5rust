function @choose(%condition : i64) -> i64 {
  slot $data : obj<16x8>
  block ^entry:
    branch %condition, ^left, ^right
  block ^left:
    %offset = addr $data
    jump ^merge
  block ^right:
    jump ^merge
  block ^merge:
    %address = phi ptr [^left: %offset, ^right: 0]
    branch %address, ^yes, ^no
  block ^yes:
    return i64 1
  block ^no:
    return i64 0
}
function @main() -> i64 [role=entry] {
  block ^entry:
    %present = call i64 @choose(1)
    %absent = call i64 @choose(0)
    %bad_present = cmp ne i64 %present, 1
    %bad_absent = cmp ne i64 %absent, 0
    %bad = binary or i64 %bad_present, %bad_absent
    return i64 %bad
}

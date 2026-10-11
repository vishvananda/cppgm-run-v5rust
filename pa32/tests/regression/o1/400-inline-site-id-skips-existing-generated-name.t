function @callee(%x : i64) -> i64 {
  block ^entry:
    %t1 = binary add i64 %x, 1
    return i64 %t1
}
function @caller(%x : i64) -> i64 {
  block ^entry:
    %__o1inl0__t1 = binary mul i64 %x, 2
    %t2 = call i64 @callee(%x)
    %t3 = binary add i64 %__o1inl0__t1, %t2
    return i64 %t3
}

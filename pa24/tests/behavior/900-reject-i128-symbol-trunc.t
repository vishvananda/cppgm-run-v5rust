global @g : i64 = 7
function @f() -> i64 {block ^entry:
%v = convert trunc i64 i128 @g
return i64 %v}
function @main() -> i64 [role=entry] {block ^entry:
return i64 0}

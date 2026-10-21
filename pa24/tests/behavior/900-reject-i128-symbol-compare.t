global @g : i64 = 7
function @f() -> i64 {block ^entry:
%v = cmp eq i128 @g, 1
return i64 %v}
function @main() -> i64 [role=entry] {block ^entry:
return i64 0}

global @g : i64 = 7
function @f() -> i128 {block ^entry:
%v = binary div i128 @g, 1
return i128 %v}
function @main() -> i64 [role=entry] {block ^entry:
return i64 0}

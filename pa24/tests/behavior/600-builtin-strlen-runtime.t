declare function @length(%s : ptr) -> i64 [object=cppgm_builtin_strlen, effects=readonly, unwind=no]
global @text = {
  i64 8680820740569200760
  i64 8680820740569200760
  i64 8680820740569200760
  i8 0
}

function @main() -> i64 [role=entry] {
  block ^entry:
    %text = addr @text
    %long = call i64 @length(%text)
    %indirect = addr @length
    %end = index i8 %text, 24
    %empty = call i64 %indirect(%end) as (%s : ptr) -> i64
    store i8 0, %text
    %changed = call i64 @length(%text)
    %a = binary sub i64 %long, 24
    %b = binary or i64 %a, %empty
    %result = binary or i64 %b, %changed
    return i64 %result
}

function @calc(%x : i64, %y : i64, %scale : i64) -> i64 {
  block ^entry:
    %q = binary div i64 %x, %y
    %r = binary mul i64 %scale, %q
    return i64 %r
}

function @main() -> i32 [role=entry, binding=strong, keep_alias=yes] {
  block ^entry:
    %got = call i64 @calc(20, 4, 7)
    %ok = cmp eq i64 %got, 35
    %bad = cmp eq i64 %ok, 0
    %exit = convert trunc i32 i64 %bad
    return i32 %exit
}

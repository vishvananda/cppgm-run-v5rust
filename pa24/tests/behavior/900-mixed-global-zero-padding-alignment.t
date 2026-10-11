global @prefix : u8 = 0
global @data = {
  i8 1
  zero 15
}

function @main() -> i32 [role=entry, binding=strong, keep_alias=yes] {
  block ^entry:
    %prefix = addr @prefix
    %data = addr @data
    %prefix_raw = copy i64 %prefix
    %data_raw = copy i64 %data
    %distance = binary sub i64 %data_raw, %prefix_raw
    %ok = cmp eq i64 %distance, 1
    %failed = cmp eq i64 %ok, 0
    %exit = convert trunc i32 i64 %failed
    return i32 %exit
}

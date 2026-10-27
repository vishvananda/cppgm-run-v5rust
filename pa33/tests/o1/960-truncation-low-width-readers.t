global @input : i32 = -7

function @low_reads(%x : i64) -> i32 {
  block ^entry:
    %a = binary add i64 %x, 4
    %n = convert trunc i32 i64 %a
    %b = binary add i64 %x, 1
    %m = convert trunc i32 i64 %b
    %r = binary add i32 %n, %m
    return i32 %r
}

function @full_read(%x : i64) -> i64 {
  block ^entry:
    %n = convert trunc i32 i64 %x
    %full = convert sext i64 i32 %n
    return i64 %full
}

function @main() -> i64 [role=entry] {
  block ^entry:
    %input = load i32 @input
    %wide = convert sext i64 i32 %input
    %low = call i32 @low_reads(%wide)
    %bad_low = cmp ne i32 %low, -9
    %full = call i64 @full_read(4294967295)
    %bad_full = cmp ne i64 %full, -1
    %bad = binary or i64 %bad_low, %bad_full
    return i64 %bad
}

global @source = {
  i64 11
  i64 22
  i64 33
  i64 44
}

global @dest = {
  i64 0
  i64 0
  i64 0
  i64 0
}

global @good : i64 = 0

function @mark(%target : ptr) -> void {
  block ^entry:
    store i64 99, %target
    return void
}

function @copy_then_mark(%target : ptr) -> void {
  block ^entry:
    %src = addr @source
    %dst = addr @dest
    copyobj 24x8 %src, %dst
    call void @mark(%target)
    return void
}

function @main() -> i32 [role=entry, binding=strong, keep_alias=yes] {
  block ^entry:
    %target = addr @good
    call void @copy_then_mark(%target)
    %good = load i64 @good
    %d0 = load i64 @dest
    %dest_base = addr @dest
    %d1p = index i8 %dest_base, 8
    %d1 = load i64 %d1p
    %d2p = index i8 %dest_base, 16
    %d2 = load i64 %d2p
    %tailp = index i8 %dest_base, 24
    %tail = load i64 %tailp
    %good_ok = cmp eq i64 %good, 99
    %d0_ok = cmp eq i64 %d0, 11
    %d1_ok = cmp eq i64 %d1, 22
    %d2_ok = cmp eq i64 %d2, 33
    %tail_ok = cmp eq i64 %tail, 0
    %first = binary and i64 %good_ok, %d0_ok
    %second = binary and i64 %d1_ok, %d2_ok
    %third = binary and i64 %first, %second
    %ok = binary and i64 %third, %tail_ok
    %failed = cmp eq i64 %ok, 0
    %exit = convert trunc i32 i64 %failed
    return i32 %exit
}

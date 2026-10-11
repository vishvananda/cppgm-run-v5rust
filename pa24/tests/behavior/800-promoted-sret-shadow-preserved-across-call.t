global @saved_ret : ptr = 0

function @clobber(%a : i64, %b : i64, %c : i64, %d : i64, %e : i64, %f : i64) -> void {
  block ^entry:
    return void
}

function @make_pair(%ret : ptr [pass=indirect_result]) -> void {
  slot $ret_shadow : ptr

  block ^entry:
    store ptr %ret, $ret_shadow
    %before = load ptr $ret_shadow
    store ptr %before, @saved_ret
    call void @clobber(1, 2, 3, 4, 5, 6)
    %first = index i8 [projection=field] %ret, 0
    store i64 11, %first
    %second = index i8 [projection=field] %ret, 8
    store i64 22, %second
    return void
}

function @main() -> i32 [role=entry, binding=strong, keep_alias=yes] {
  slot $out : obj<16x8>

  block ^entry:
    %out = addr $out
    call void @make_pair(%out)
    %saved = load ptr @saved_ret
    %saved_ok = cmp eq ptr %saved, %out
    %v0 = load i64 %out
    %field1 = index i8 [projection=field] %out, 8
    %v1 = load i64 %field1
    %ok0 = cmp eq i64 %v0, 11
    %ok1 = cmp eq i64 %v1, 22
    %pair_ok = binary and i64 %ok0, %ok1
    %ok = binary and i64 %saved_ok, %pair_ok
    %failed = cmp eq i64 %ok, 0
    %exit = convert trunc i32 i64 %failed
    return i32 %exit
}

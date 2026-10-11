global @copied = {
  i64 0
}

function @make_tag_spilled() -> obj<8x8> {
  slot $tmp : obj<8x8>
  slot $force_spill : obj<40x8>

  block ^entry:
    %p = addr $tmp
    store i64 77, %p
    return obj<8x8> $tmp
}

function @main() -> i32 [role=entry, binding=strong, keep_alias=yes] {
  block ^entry:
    %tag = call obj<8x8> @make_tag_spilled()
    %dst = addr @copied
    copyobj 8x8 %tag, %dst
    %v = load i64 @copied
    %ok = cmp eq i64 %v, 77
    %fail = cmp eq i64 %ok, 0
    %exit = convert trunc i32 i64 %fail
    return i32 %exit
}

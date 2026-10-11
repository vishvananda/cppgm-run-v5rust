function @narrow_false() -> u8 {
  block ^entry:
    %wide = const i64 256
    return u8 %wide
}

function @main() -> i32 [role=entry, binding=strong, keep_alias=yes] {
  block ^entry:
    %value = call u8 @narrow_false()
    %ok = cmp eq i64 %value, 0
    branch %ok, ^success, ^failure

  block ^success:
    return i32 0

  block ^failure:
    return i32 1
}

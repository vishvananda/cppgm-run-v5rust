function @dispatch(%a : i64, %b : i64, %c : i64, %d : i64, %e : i64, %f : i64) -> i64 {
  slot $selector_word : i64
  slot $poison_word : i64

  block ^entry:
    store i64 4294967335, $selector_word
    store i64 1, $poison_word
    %poison = load u32 $poison_word
    %selector = load u32 $selector_word
    jump ^dispatch

  block ^dispatch:
    %ab = binary add i64 %a, %b
    %cd = binary add i64 %c, %d
    %ef = binary add i64 %e, %f
    %abcd = binary add i64 %ab, %cd
    %all = binary add i64 %abcd, %ef
    switch %selector, ^retry, 39:^success, %poison:^failure

  block ^retry:
    jump ^dispatch

  block ^success:
    return i64 %all

  block ^failure:
    return i64 -1
}

function @main() -> i32 [role=entry, binding=strong, keep_alias=yes] {
  block ^entry:
    %actual = call i64 @dispatch(1, 2, 3, 4, 5, 6)
    %failed = cmp ne i64 %actual, 21
    %exit = convert trunc i32 i64 %failed
    return i32 %exit
}

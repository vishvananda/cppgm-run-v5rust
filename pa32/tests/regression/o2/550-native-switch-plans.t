function @runs(%selector : i8) -> i32 [no_inline=yes, unwind=no] {
  block ^entry:
    switch %selector, ^fallback, -128:^target0, -127:^target0, -126:^target0, -125:^target0, -124:^target0, -123:^target0, -122:^target0, -121:^target0, 120:^target1, 121:^target1, 122:^target1, 123:^target1, 124:^target1, 125:^target1, 126:^target1, 127:^target1
  block ^target0:
    return i32 10
  block ^target1:
    return i32 11
  block ^fallback:
    return i32 9
}

function @masks(%selector : i64) -> i32 [no_inline=yes, unwind=no] {
  block ^entry:
    switch %selector, ^fallback, 0:^target0, 2:^target1, 4:^target0, 6:^target1, 8:^target0, 10:^target1, 12:^target0, 14:^target1
  block ^target0:
    return i32 10
  block ^target1:
    return i32 11
  block ^fallback:
    return i32 9
}

function @window_first(%selector : i64) -> i32 [no_inline=yes, unwind=no] {
  block ^entry:
    switch %selector, ^fallback, 0:^target0, 2:^target1, 4:^target0, 6:^target1, 8:^target0, 10:^target1, 12:^target0, 14:^target1, 1000:^target2
  block ^target0:
    return i32 10
  block ^target1:
    return i32 11
  block ^target2:
    return i32 12
  block ^fallback:
    return i32 9
}

function @tree(%selector : i64) -> i32 [no_inline=yes, unwind=no] {
  block ^entry:
    switch %selector, ^fallback, 0:^target0, 100:^target1, 200:^target2, 300:^target0, 400:^target1, 500:^target2, 600:^target0, 700:^target1, 800:^target2, 900:^target0, 1000:^target1, 1100:^target2
  block ^target0:
    return i32 10
  block ^target1:
    return i32 11
  block ^target2:
    return i32 12
  block ^fallback:
    return i32 9
}

function @table(%selector : i64) -> i32 [no_inline=yes, unwind=no] {
  block ^entry:
    switch %selector, ^fallback, -2:^target0, -1:^target1, 1:^target2, 2:^target3, 2:^target4, 3:^target4, 4:^target5, 5:^target6
  block ^target0:
    return i32 10
  block ^target1:
    return i32 11
  block ^target2:
    return i32 12
  block ^target3:
    return i32 13
  block ^target4:
    return i32 14
  block ^target5:
    return i32 15
  block ^target6:
    return i32 16
  block ^fallback:
    return i32 9
}

function @wide_cases(%selector : i64) -> i32 [no_inline=yes, unwind=no] {
  block ^entry:
    switch %selector, ^fallback, 1099511627776:^target0, 1099511627777:^target1, -1:^target2
  block ^target0:
    return i32 10
  block ^target1:
    return i32 11
  block ^target2:
    return i32 12
  block ^fallback:
    return i32 9
}

function @dynamic(%selector : i64, %case : i64) -> i32 [no_inline=yes, unwind=no] {
 block ^entry:
 switch %selector, ^no, %case:^yes
 block ^yes:
 return i32 10
 block ^no:
 return i32 9
}

function @table_phi(%selector : i32) -> i32 [no_inline=yes, unwind=no] {
 block ^entry:
 switch %selector, ^fallback, 0:^target0, 1:^target1, 2:^target2, 3:^target3, 4:^target4, 5:^target5, 6:^target6, 7:^target7
 block ^target0:
 %v0 = phi i32 [^entry: 20]
 return i32 %v0
 block ^target1:
 %v1 = phi i32 [^entry: 21]
 return i32 %v1
 block ^target2:
 %v2 = phi i32 [^entry: 22]
 return i32 %v2
 block ^target3:
 %v3 = phi i32 [^entry: 23]
 return i32 %v3
 block ^target4:
 %v4 = phi i32 [^entry: 24]
 return i32 %v4
 block ^target5:
 %v5 = phi i32 [^entry: 25]
 return i32 %v5
 block ^target6:
 %v6 = phi i32 [^entry: 26]
 return i32 %v6
 block ^target7:
 %v7 = phi i32 [^entry: 27]
 return i32 %v7
 block ^fallback:
 return i32 9
}
function @scratch(%address : ptr) -> i32 [no_inline=yes, unwind=no] {
 block ^entry:
 %selector = load i64 %address
 switch %selector, ^fallback, 0:^target0, 2:^target1, 4:^target0, 6:^target1, 8:^target0, 10:^target1, 12:^target0, 14:^target1
 block ^target0:
 return i32 10
 block ^target1:
 return i32 11
 block ^fallback:
 return i32 9
}

function @wide_selector(%selector : i128) -> i32 [no_inline=yes, unwind=no] {
 block ^entry:
 switch %selector, ^fallback, 18446744073709551616:^high, -1:^negative
 block ^high:
 return i32 31
 block ^negative:
 return i32 32
 block ^fallback:
 return i32 9
}

function @main() -> i32 {
 slot $cell : i64
 block ^entry:
 %value0 = call i32 @runs(-128)
 %value1 = call i32 @runs(127)
 %sum1 = binary add i32 %value0, %value1
 %value2 = call i32 @runs(0)
 %sum2 = binary add i32 %sum1, %value2
 %value3 = call i32 @masks(0)
 %sum3 = binary add i32 %sum2, %value3
 %value4 = call i32 @masks(2)
 %sum4 = binary add i32 %sum3, %value4
 %value5 = call i32 @masks(1)
 %sum5 = binary add i32 %sum4, %value5
 %value6 = call i32 @window_first(1000)
 %sum6 = binary add i32 %sum5, %value6
 %value7 = call i32 @window_first(2)
 %sum7 = binary add i32 %sum6, %value7
 %value8 = call i32 @window_first(14)
 %sum8 = binary add i32 %sum7, %value8
 %value9 = call i32 @window_first(1)
 %sum9 = binary add i32 %sum8, %value9
 %value10 = call i32 @tree(1100)
 %sum10 = binary add i32 %sum9, %value10
 %value11 = call i32 @tree(100)
 %sum11 = binary add i32 %sum10, %value11
 %value12 = call i32 @tree(550)
 %sum12 = binary add i32 %sum11, %value12
 %value13 = call i32 @table(2)
 %sum13 = binary add i32 %sum12, %value13
 %value14 = call i32 @table(0)
 %sum14 = binary add i32 %sum13, %value14
 %value15 = call i32 @table(5)
 %sum15 = binary add i32 %sum14, %value15
 %value16 = call i32 @table(-2)
 %sum16 = binary add i32 %sum15, %value16
 %value17 = call i32 @wide_cases(1099511627776)
 %sum17 = binary add i32 %sum16, %value17
 %value18 = call i32 @wide_cases(-1)
 %sum18 = binary add i32 %sum17, %value18
 %value19 = call i32 @dynamic(7, 7)
 %sum19 = binary add i32 %sum18, %value19
 store i64 6, $cell
 %pointer = addr $cell
 %scratch_value = call i32 @scratch(%pointer)
 %phi_value = call i32 @table_phi(6)
 %wide_value = call i32 @wide_selector(18446744073709551616)
 %with_scratch = binary add i32 %sum19, %scratch_value
 %with_phi = binary add i32 %with_scratch, %phi_value
 %with_wide = binary add i32 %with_phi, %wide_value
 %result = binary sub i32 %with_wide, 283
 return i32 %result
}

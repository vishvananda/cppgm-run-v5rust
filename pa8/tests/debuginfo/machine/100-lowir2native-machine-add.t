function @main() -> i32 [binding=strong] {
  block ^entry:
    %t1 = binary add i32 40, 2 !dbg(tests/debuginfo/machine/100-lowir2native-machine-add.t, 3, 10)
    return i32 %t1
}

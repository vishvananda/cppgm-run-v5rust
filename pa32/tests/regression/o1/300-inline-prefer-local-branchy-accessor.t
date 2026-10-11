function @accessor(%p : ptr) -> i64 [prefer_local=yes] {
  slot $p : ptr
  slot $cond__1 : i64

  block ^entry:
    store ptr %p, $p
    %t1 = load ptr $p
    %t2 = index i8 [projection=field] %t1, 0
    %t3 = index i8 [projection=field] %t2, 0
    %t4 = index i8 [projection=field] %t3, 0
    %t5 = load i8 %t4
    %t6 = binary and i8 %t5, 1
    branch %t6, ^long, ^short

  block ^long:
    %t7 = load ptr $p
    %t8 = index i8 [projection=field] %t7, 0
    %t9 = index i8 [projection=field] %t8, 0
    %t10 = index i8 [projection=field] %t9, 8
    %t11 = load i64 %t10
    store i64 %t11, $cond__1
    jump ^end

  block ^short:
    %t12 = load ptr $p
    %t13 = index i8 [projection=field] %t12, 0
    %t14 = index i8 [projection=field] %t13, 0
    %t15 = index i8 [projection=field] %t14, 0
    %t16 = load i8 %t15
    %t17 = binary shr i8 %t16, 1
    %t18 = binary and i8 %t17, 127
    store i64 %t18, $cond__1
    jump ^end

  block ^end:
    %t19 = load i64 $cond__1
    return i64 %t19
}
function @caller(%p : ptr) -> i64 {
  block ^entry:
    %t1 = call i64 @accessor(%p)
    %t2 = binary add i64 %t1, 1
    return i64 %t2
}

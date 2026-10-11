function @hash_like(%h : i64, %bc : i64) -> i64 [prefer_local=yes] {
  slot $h : i64
  slot $bc : i64
  slot $cond__1 : i64
  slot $cond__2 : i64

  block ^entry:
    store i64 %h, $h
    store i64 %bc, $bc
    %t1 = load i64 $bc
    %t2 = load i64 $bc
    %t4 = binary sub i64 %t2, 1
    %t5 = binary and i64 %t1, %t4
    %t6 = cmp eq i64 %t5, 0
    branch %t6, ^pow2, ^generic

  block ^pow2:
    %t7 = load i64 $h
    %t8 = load i64 $bc
    %t10 = binary sub i64 %t8, 1
    %t11 = binary and i64 %t7, %t10
    store i64 %t11, $cond__1
    jump ^end

  block ^generic:
    %t12 = load i64 $h
    %t13 = load i64 $bc
    %t14 = cmp ult i64 %t12, %t13
    branch %t14, ^small, ^mod

  block ^small:
    %t15 = load i64 $h
    store i64 %t15, $cond__2
    jump ^merge

  block ^mod:
    %t16 = load i64 $h
    %t17 = load i64 $bc
    %t18 = binary umod i64 %t16, %t17
    store i64 %t18, $cond__2
    jump ^merge

  block ^merge:
    %t19 = load i64 $cond__2
    store i64 %t19, $cond__1
    jump ^end

  block ^end:
    %t20 = load i64 $cond__1
    return i64 %t20
}

function @caller(%h : i64, %bc : i64) -> i64 {
  block ^entry:
    %t1 = call i64 @hash_like(%h, %bc)
    %t2 = binary add i64 %t1, 1
    return i64 %t2
}

global @table = {
  ptr addr @mix
}

function @mix(%a : i64, %b : f64, %c : i64, %d : f64) -> i64 {
  block ^entry:
    %bi = convert fptosi i64 f64 %b
    %di = convert fptosi i64 f64 %d
    %sum1 = binary add i64 %a, %c
    %sum2 = binary add i64 %bi, %di
    %sum = binary add i64 %sum1, %sum2
    return i64 %sum
}

function @main() -> i64 [role=entry] {
  block ^entry:
    %fp = load ptr @table
    %result = call i64 %fp(10, 1.5, 20, 2.5) as (%arg0 : i64, %arg1 : f64, %arg2 : i64, %arg3 : f64) -> i64
    %ok = cmp eq i64 %result, 33
    return i64 %ok
}

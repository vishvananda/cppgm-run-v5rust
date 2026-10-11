global @g : i64 = 4

function @add() -> i64 {
  block ^entry:
    %0 = load i64 @g
    %1 = const i64 3
    %2 = binary add i64 %0, %1
    return i64 %2
}

function @main() -> i64 {
  block ^entry:
    %0 = call i64 @add()
    return i64 %0
}

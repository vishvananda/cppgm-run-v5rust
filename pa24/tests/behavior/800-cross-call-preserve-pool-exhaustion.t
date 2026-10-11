global @g1 : i64 = 10
global @g2 : i64 = 20
global @g3 : i64 = 30
global @g4 : i64 = 40
global @g5 : i64 = 50
global @g6 : i64 = 60

function @clobber(%a : i64, %b : i64, %c : i64, %d : i64, %e : i64, %f : i64) -> void {
  block ^entry:
    return void
}

function @run() -> i64 {
  block ^entry:
    %v1 = load i64 @g1
    %v2 = load i64 @g2
    %v3 = load i64 @g3
    %v4 = load i64 @g4
    %v5 = load i64 @g5
    %v6 = load i64 @g6
    call void @clobber(1, 2, 3, 4, 5, 6)
    %s1 = binary add i64 %v1, %v2
    %s2 = binary add i64 %s1, %v3
    %s3 = binary add i64 %s2, %v4
    %s4 = binary add i64 %s3, %v5
    %s5 = binary add i64 %s4, %v6
    return i64 %s5
}

function @main() -> i32 [role=entry, binding=strong, keep_alias=yes] {
  block ^entry:
    %sum = call i64 @run()
    %ok = cmp eq i64 %sum, 210
    %failed = cmp eq i64 %ok, 0
    %exit = convert trunc i32 i64 %failed
    return i32 %exit
}

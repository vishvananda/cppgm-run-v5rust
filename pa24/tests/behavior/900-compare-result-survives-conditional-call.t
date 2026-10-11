function @clobber(%a : i64, %b : i64, %c : i64, %d : i64, %e : i64) -> i64 {
  block ^entry:
    %result = binary add i64 %a, %e
    return i64 %result
}

function @classify(%kind : i32) -> i64 {
  slot $saved_kind : i32

  block ^entry:
    store i32 %kind, $saved_kind
    %loaded_kind = load i32 $saved_kind
    %unnamed = cmp ne i32 %loaded_kind, 1
    %named = cmp eq i64 %unnamed, 0
    branch %named, ^named_path, ^merge

  block ^named_path:
    %discard = call i64 @clobber(1, 2, 3, 4, 5)
    jump ^merge

  block ^merge:
    return i64 %unnamed
}

function @main() -> i32 [role=entry] {
  block ^entry:
    %warm = call i64 @classify(1)
    %actual = call i64 @classify(0)
    %failed = cmp ne i64 %actual, 1
    %exit = convert trunc i32 i64 %failed
    return i32 %exit
}

global @value : i64 = 77
global @value_ptr : ptr = addr @value

function @main() -> i64 [role=entry] {
  block ^entry:
    %address = load ptr @value_ptr
    %copy = copy ptr %address
    %same = cmp eq ptr %copy, %address
    branch %same, ^check, ^bad

  block ^bad:
    return i64 1

  block ^check:
    %loaded = load i64 %copy
    %wrong = cmp ne i64 %loaded, 77
    return i64 %wrong
}

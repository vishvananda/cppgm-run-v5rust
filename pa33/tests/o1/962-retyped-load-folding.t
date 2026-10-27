# Pointer/i64 aliases preserve their eight-byte memory read in O1 folding.
global @pointer : ptr = 0
global @word : i64 = 0
function @read_pointer() -> i64 {
block ^entry:
  %loaded = load ptr @pointer
  %view = copy i64 %loaded
  %sum = binary add i64 %view, 1
  return i64 %sum
}
function @read_word() -> ptr {
block ^entry:
  %loaded = load i64 @word
  %view = copy ptr %loaded
  %sum = binary add ptr %view, 2
  return ptr %sum
}
function @main() -> i64 [role=entry] {
block ^entry:
  %pointer = call i64 @read_pointer()
  %word = call ptr @read_word()
  %a = cmp ne i64 %pointer, 1
  %b = cmp ne ptr %word, 2
  %bad = binary or i1 %a, %b
  return i64 %bad
}

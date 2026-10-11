# pa8.gram: a return operand is a value; `return i32 ,` is malformed and must
# be rejected
function @f() -> i32 {
  block ^entry:
    return i32 ,
}

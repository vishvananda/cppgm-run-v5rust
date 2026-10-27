function @integer_to_float(%value : i64) -> i64 {
  block ^entry:
    %converted = convert sitofp i64 i64 %value
    return i64 %converted
}
function @float_to_integer(%value : i64) -> i64 {
  block ^entry:
    %converted = convert fptosi i64 i64 %value
    return i64 %converted
}
function @floating_integer_extension(%value : f64) -> f64 {
  block ^entry:
    %converted = convert sext f64 f64 %value
    return f64 %converted
}
function @floating_literal_extension() -> f64 {
  block ^entry:
    %converted = convert sext f64 f64 1.25
    return f64 %converted
}
function @floating_literal_to_float() -> f64 {
  block ^entry:
    %converted = convert sitofp f64 f64 1.25
    return f64 %converted
}
function @wide_to_float(%value : i128) -> i128 {
  block ^entry:
    %converted = convert sitofp i128 i128 %value
    return i128 %converted
}
function @wide_from_float(%value : i128) -> i128 {
  block ^entry:
    %converted = convert fptosi i128 i128 %value
    return i128 %converted
}
function @pointer_across_call(%pointer : ptr, %condition : i1) -> ptr {
  block ^entry:
    %converted = convert trunc ptr ptr %pointer
    call void @clobber()
    %chosen = select ptr %condition, %converted, %pointer
    return ptr %chosen
}
function @clobber() -> void {
  block ^entry:
    return void
}
function @main() -> i64 [role=entry] {
  block ^entry:
    return i64 0
}

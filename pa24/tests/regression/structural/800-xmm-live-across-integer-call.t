global @observed : f64 = 0.0

function @clobber_xmm_from_integer(%value : i32) -> i32 {
  block ^entry:
    %converted = convert sitofp f64 i32 %value
    %adjusted = binary add f64 %converted, 0.5
    %product = binary mul f64 %adjusted, 3.0
    store f64 %product, @observed
    %result = binary sub i32 %value, 5
    return i32 %result
}

function @preserve_across_integer_call(%value : f64) -> f64 {
  block ^entry:
    %live = binary add f64 %value, 1.25
    %integer = call i32 @clobber_xmm_from_integer(7)
    %converted = convert sitofp f64 i32 %integer
    %result = binary add f64 %live, %converted
    return f64 %result
}

function @main() -> i32 [role=entry, binding=strong, keep_alias=yes] {
  block ^entry:
    %result = call f64 @preserve_across_integer_call(4.0)
    %result_ok = cmp eq f64 %result, 7.25
    %observed = load f64 @observed
    %observed_ok = cmp eq f64 %observed, 22.5
    %ok = binary and i64 %result_ok, %observed_ok
    %failed = cmp eq i64 %ok, 0
    %exit = convert trunc i32 i64 %failed
    return i32 %exit
}

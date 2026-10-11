global @good : i64 = 42
global @bad : i64 = 13

function @sink(%p : ptr) -> i64 {
  block ^entry:
    %v = load i64 %p
    return i64 %v
}

function @target(%a : ptr, %b : ptr, %c : ptr, %d : ptr, %z : ptr, %f : ptr, %g : ptr) -> i64 {
  slot $z : ptr
  slot $force_spill : obj<40x8>

  block ^entry:
    store ptr %z, $z
    %scratch = addr $force_spill
    store ptr %b, %scratch
    jump ^after

  block ^after:
    %zp = load ptr $z
    %result = call i64 @sink(%zp)
    return i64 %result
}

function @main() -> i32 [role=entry, binding=strong, keep_alias=yes] {
  block ^entry:
    %good = addr @good
    %bad = addr @bad
    %r = call i64 @target(%bad, %bad, 0, 0, %good, 0, 0)
    %ok = cmp eq i64 %r, 42
    %fail = cmp eq i64 %ok, 0
    %exit = convert trunc i32 i64 %fail
    return i32 %exit
}

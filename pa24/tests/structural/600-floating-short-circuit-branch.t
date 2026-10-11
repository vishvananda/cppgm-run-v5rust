function @check(%p : ptr) -> void {
  block ^entry:
    %first_ptr = index i8 %p, 0
    %first = load f64 %first_ptr
    %first_truth = cmp ne f64 %first, 0.0
    branch %first_truth, ^rhs, ^end

  block ^rhs:
    %second_ptr = index i8 %p, 8
    %second = load i32 %second_ptr
    %second_truth = cmp ne i32 %second, 0
    branch %second_truth, ^then, ^end

  block ^then:
    store i32 1, @seen
    jump ^end

  block ^end:
    return void
}

global @seen : i32 = 0
global @pair = {
  f64 1.0
  i32 2
}

function @main() -> i64 [role=entry] {
  block ^entry:
    %p = addr @pair
    call void @check(%p)
    %seen = load i32 @seen
    %ok = cmp eq i32 %seen, 1
    branch %ok, ^pass, ^fail

  block ^pass:
    return i64 0

  block ^fail:
    return i64 1
}

function @min_of(%a : i64, %b : i64) -> i64 {
  slot $a : i64
  slot $b : i64

  block ^entry:
    store i64 %a, $a
    store i64 %b, $b
    %pa = addr $a
    %pb = addr $b
    %ca = copy ptr %pb
    %vb = load i64 %ca
    %cb = copy ptr %pa
    %va = load i64 %cb
    %less = cmp ult i64 %vb, %va
    branch %less, ^then, ^else

  block ^then:
    jump ^end

  block ^else:
    jump ^end

  block ^end:
    %p = phi ptr [^then: %pb, ^else: %pa]
    %cp = copy ptr %p
    %v = load i64 %cp
    return i64 %v
}

function @min_kept_after_store(%a : i64, %b : i64, %out : ptr) -> i64 {
  slot $a : i64
  slot $b : i64

  block ^entry:
    store i64 %a, $a
    store i64 %b, $b
    %pa = addr $a
    %pb = addr $b
    %vb = load i64 %pb
    %va = load i64 %pa
    %less = cmp ult i64 %vb, %va
    branch %less, ^then, ^else

  block ^then:
    store i64 7, %out
    jump ^end

  block ^else:
    jump ^end

  block ^end:
    %p = phi ptr [^then: %pb, ^else: %pa]
    %v = load i64 %p
    return i64 %v
}

function @min_of_fields(%q : ptr) -> i64 {
  block ^entry:
    %x = index i8 %q, 0
    %y = index i8 %q, 8
    %vx = load i64 %x
    %vy = load i64 %y
    %less = cmp ult i64 %vy, %vx
    branch %less, ^then, ^else

  block ^then:
    jump ^end

  block ^else:
    jump ^end

  block ^end:
    %p = phi ptr [^then: %y, ^else: %x]
    %v = load i64 %p
    return i64 %v
}

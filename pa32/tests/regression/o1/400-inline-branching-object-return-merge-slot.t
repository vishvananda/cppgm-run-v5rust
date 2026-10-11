function @leaf(%cond : i1, %p : ptr) -> obj<8x8> {
  slot $retobj__1 : obj<8x8>

  block ^entry:
    branch %cond, ^then, ^else

  block ^then:
    %t1 = addr $retobj__1
    store ptr %p, %t1
    return obj<8x8> $retobj__1

  block ^else:
    %t2 = addr $retobj__1
    store ptr 0, %t2
    return obj<8x8> $retobj__1
}

function @caller(%cond : i1, %p : ptr, %out : ptr) -> void {
  block ^entry:
    %t1 = call obj<8x8> @leaf(%cond, %p)
    copyobj 8x8 %t1, %out
    return void
}

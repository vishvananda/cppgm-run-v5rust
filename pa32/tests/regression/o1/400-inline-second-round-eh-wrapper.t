function @caller(%p : ptr) -> ptr {
  block ^entry:
    eh_try ^dispatch
    %t1 = call ptr @mid(%p)
    eh_end
    return ptr %t1

  block ^dispatch:
    resume
}

function @mid(%p : ptr) -> ptr {
  block ^entry:
    %t1 = call ptr @leaf(%p)
    return ptr %t1
}

function @leaf(%p : ptr) -> ptr {
  slot $p : ptr

  block ^entry:
    store ptr %p, $p
    %t1 = load ptr $p
    return ptr %t1
}

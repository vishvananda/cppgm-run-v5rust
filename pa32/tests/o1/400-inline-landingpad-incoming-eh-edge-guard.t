function @leaf() -> void [unwind=no, prefer_local=yes] {
  block ^entry:
    return void
}

declare function @may_throw() -> void

function @main() -> void {
  block ^entry:
    eh_try ^landing
    call void @may_throw()
    eh_end
    return void

  block ^landing:
    call void @leaf()
    eh_end
    resume
}

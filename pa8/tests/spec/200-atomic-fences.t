function @main() -> i64 {
  block ^entry:
    atomic_thread_fence 5
    atomic_signal_fence 5
    %one = const i64 1
    return i64 %one
}

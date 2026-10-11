struct pair_value {
  int first;
  int second;
};

template<class T>
struct atomic_box {
  _Atomic(T) value;

  explicit atomic_box(T initial) : value(initial) {}

  T load() const {
    return __c11_atomic_load(&value, __ATOMIC_ACQUIRE);
  }

  void store(T replacement) {
    __c11_atomic_store(&value, replacement, __ATOMIC_RELEASE);
  }

  bool compare_exchange(T& expected, T desired) {
    return __c11_atomic_compare_exchange_strong(
        &value,
        &expected,
        desired,
        __ATOMIC_SEQ_CST,
        __ATOMIC_ACQUIRE);
  }
};

int main() {
  atomic_box<pair_value> value(pair_value{17, 25});
  pair_value expected{17, 25};
  value.compare_exchange(expected, pair_value{19, 23});
  value.store(pair_value{21, 21});
  pair_value loaded = value.load();
  return loaded.first + loaded.second == 42 ? 0 : 1;
}

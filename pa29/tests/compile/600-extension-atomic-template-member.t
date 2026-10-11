template <class T>
struct atomic_box {
  atomic_box() noexcept = default;
  explicit atomic_box(T v) noexcept : value(v) {}
  __extension__ _Atomic(T) value;
};

atomic_box<int> g(1);

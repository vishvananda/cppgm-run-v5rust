struct direct_box {
  explicit direct_box(int v) noexcept : value(v) {}
  _Atomic(int) value;
};

struct assign_box {
  explicit assign_box(int v) noexcept { value = v; }
  _Atomic(int) value;
};

template <class T>
struct templ_direct_box {
  explicit templ_direct_box(T v) noexcept : value(v) {}
  _Atomic(T) value;
};

template <class T>
struct templ_assign_box {
  explicit templ_assign_box(T v) noexcept { value = v; }
  _Atomic(T) value;
};

direct_box g1(1);
assign_box g2(2);
templ_direct_box<int> g3(3);
templ_assign_box<int> g4(4);

// expect reject
struct B { int m; };
struct D : B {
  static constexpr int D::*q = static_cast<int D::*>(&B::m) == nullptr
    ? nullptr : (1/0 ? nullptr : nullptr);
  static_assert(false, "later assertion");
};

namespace n {
template<class A> struct c {
  struct b { template<class> c f() const { return {}; } };
  struct d : b {
    using b::f;
    c f();
  };
};
}

using C = n::c<n::c<int>>;

int main() {
  C::d const v{};
  v.f<int>();
}

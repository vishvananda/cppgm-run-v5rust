// Default/value queries retain the inherited template's specialization;
// supplied arguments select a forwarding constructor. Expect exit 0.
struct Base {
  int n;
  template<class T = int> Base(T x = T(8)) : n(x) {}
};
struct Derived : Base { using Base::Base; Derived(double) : Base(9) {} };
struct Middle : Base { using Base::Base; };
struct End : Middle { using Middle::Middle; };
struct Own : Base { using Base::Base; Own() : Base(3) {} };
struct Packed {
  int n;
  template<class... T> Packed(T...) : n(sizeof...(T)) {}
};
struct EmptyPack : Packed { using Packed::Packed; };
int take(Derived value = {}) { return value.n; }
int main() {
  Derived a;
  Derived b{};
  Derived c = {};
  End d;
  End e(4);
  Own f;
  EmptyPack g;
  EmptyPack h(1, 2);
  return a.n + b.n + c.n + d.n + e.n + f.n + g.n + h.n + take() - 49;
}

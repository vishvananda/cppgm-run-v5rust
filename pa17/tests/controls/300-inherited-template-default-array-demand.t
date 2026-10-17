// Array storage demands the selected base template definition; an unused
// inline function's array does not demand its invalid body. Expect exit 0.
struct Base {
  int n;
  template<class T = int> Base(T x = T(8)) : n(x) {}
};
struct Derived : Base { using Base::Base; };
struct Bad { template<class T = int> Bad(T x = T()) { T::missing(); } };
struct Unused : Bad { using Bad::Bad; };
inline void unused() { Unused values[2]; }
int main() {
  Derived values[2];
  Derived braced[2]{};
  return values[0].n + values[1].n + braced[0].n + braced[1].n - 32;
}

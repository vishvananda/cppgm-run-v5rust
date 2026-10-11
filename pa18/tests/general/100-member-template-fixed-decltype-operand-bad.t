// N3485 14.7.1p1: instantiating A<X> declares its member template f with T =
// X; declval<X&>().run() then depends on none of f's parameters, so its error
// is the instance's, not a deduction failure of f (14.8.2p8).
template<class T> T&& declval();
struct X {};
template<class T> struct A {
  template<class U> static int f(U u, decltype((void)declval<T&>().run(), (void)u, 0)* = 0) { return 1; }
  static int f(...) { return 0; }
};
int main() { return A<X>::f(0); }

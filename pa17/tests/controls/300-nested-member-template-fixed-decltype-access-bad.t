// expect reject
// The instantiated nested class declares its member template without demanding
// that member's body. Its now-fixed member access is already invalid.
template<class T> T&& declval();
struct X {};
template<class T> struct A {
  struct B {
    template<class U> auto f() -> decltype((declval<T&>().missing, declval<U>()));
  };
};
int main() { A<X>::B b; return 0; }

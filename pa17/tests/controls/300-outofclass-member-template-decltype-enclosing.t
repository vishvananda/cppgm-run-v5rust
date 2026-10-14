// N3485 14.5.2p1, 14.5.6.1p5-6: out-of-class definition of a member template
// whose decltype return type names only the enclosing parameter C is
// equivalent to its declaration; expect exit 0
template <class T> T&& declval();
template <class C> struct A {
  template <class T> static auto f(T t) -> decltype(declval<C>() + 1);
};
template <class C> template <class T> auto A<C>::f(T t) -> decltype(declval<C>() + 1) { return 1; }
int main() { return A<int>::f(1) == 1 ? 0 : 1; }

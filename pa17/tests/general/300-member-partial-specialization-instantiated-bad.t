// N3485 14.5.5.3p2, 14.7.1p1: instantiating A<X> instantiates the declaration
// of its member partial specialization, whose argument declval<X&>().run() is
// invalid outside any deduction; must be rejected
template<class T> T&& declval();
struct X {};
template<class T> struct A {
  template<class U, class = void> struct has { static const int v = 0; };
  template<class U> struct has<U, decltype(void(declval<T&>().run()), void(declval<U>()))> { static const int v = 1; };
};
int main() { return A<X>::has<int>::v; }

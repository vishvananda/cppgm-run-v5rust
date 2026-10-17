// expect exit 0
// A member name stays with its dependent call; fixed unevaluated calls do not
// demand their bodies. Completing the owner must preserve both boundaries.
template<class T> T helper() { return T::bad; }
struct X {};
template<class T> struct A {
  template<class U> auto deferred() -> decltype(T().missing(U()), void());
  template<class U> auto fixed() -> decltype(helper<T>(), void(U())) {
    return U::unused_body;
  }
};
int main() { A<X> a; return 0; }

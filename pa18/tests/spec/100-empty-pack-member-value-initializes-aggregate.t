struct X { int n; };
template<class T> union U {
  T value;
  template<class... A> U(A&&... a): value(static_cast<A&&>(a)...) {}
};
int main() { U<X> u; return u.value.n; }

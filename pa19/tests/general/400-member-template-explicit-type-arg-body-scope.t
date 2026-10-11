template<class T> T&& forward(T&);
template<class T>
struct X {
  template<class U1>
  static U1&& f(U1& u1) { return forward<U1>(u1); }
};
unsigned long g(unsigned long x) { return X<int>::f<unsigned long>(x); }

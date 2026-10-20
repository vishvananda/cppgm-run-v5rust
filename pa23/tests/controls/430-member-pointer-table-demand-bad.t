// expect reject
template<class T> struct R {
  virtual int extra() { return T::missing(); }
};
struct D : virtual R<int> {
  virtual int f(int n) { return n; }
};
struct X { int h(int); };
typedef int (X::*Fn)(int);
struct C { int g(); };
struct E : virtual R<int> { Fn get(); };
int use(D& p, C& q, E& r, X& x) { return p.f((x.*r.get())(q.g())); }

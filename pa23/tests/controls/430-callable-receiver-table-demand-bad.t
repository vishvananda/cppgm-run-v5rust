// expect reject
template<class T> struct R {
  virtual int extra() { return T::missing(); }
};
struct D : virtual R<int> {
  virtual int f(int n) { return n; }
};
struct C { int operator()(int); };
struct E : virtual R<int> { C get(); };
int use(D& p, E& r) { return p.f(r.get()(1)); }

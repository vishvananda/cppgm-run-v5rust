// expect reject
template<class T> struct R {
  virtual int extra() { return T::missing(); }
};
struct D : virtual R<int> {
  virtual int f(int n) { return n; }
};
struct C {};
int use(D& p, C& q) { return p.f((q.~C(), 1)); }

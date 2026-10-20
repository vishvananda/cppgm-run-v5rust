// expect reject
template<class T> struct R {
  virtual int extra() { return T::missing(); }
};
struct D : virtual R<int> {
  virtual int f(int n) { return n; }
};
typedef int (*Fn)(int);
struct C { int g(); };
struct E : virtual R<int> { Fn get(); };
int use(D& p, C& q, E& r) { return p.f((r.get())(q.g())); }

// expect exit 0
template<class T> struct R {
  virtual int extra() { return T::missing(); }
};
struct D : virtual R<int> {
  virtual int f(int n) { return n; }
};
struct C { int g() { return 2; } };
int use(D& p, C& q) { return p.f(int{q.g()}); }
int main() { return 0; }

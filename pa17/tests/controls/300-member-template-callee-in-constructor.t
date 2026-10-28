// expect exit 0
// The template-call owner binds this after forming its explicit candidates.
struct A {
  int n;
  template<class T> void add(T value) { n += value; }
  A() : n(0) { add<int>(4); }
};
int main() { A value; return value.n - 4; }

// VALIDATION: compile-pass
// N3485 focus: 8.3.5 [dcl.fct], 13.3.1 [over.match.funcs], 14.5.2 [temp.mem]
struct X {
  template<class T> int f(T &) && { return 1; }
  template<class T> int f(T &) & { return 0; }
};
template<class T> int g(X *x, T &v) { return x->f(v); }
int main() { X x; int v = 0; return g(&x, v); }

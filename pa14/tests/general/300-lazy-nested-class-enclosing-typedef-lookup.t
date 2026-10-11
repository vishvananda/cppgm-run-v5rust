typedef int P;
template<class T> struct O {
  typedef T P;
  struct I { static void f(P &) {} };
};
int main() { long x; O<long>::I::f(x); }

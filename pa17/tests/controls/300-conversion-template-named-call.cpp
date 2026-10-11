// N3485 14.5.2p5-p6, 14.8.2.3: a.operator int*() names the specialization of
// the conversion template deduced from int*; with operator T() also present,
// ordering selects operator T*; expect exit 0
int g = 0;
struct B {
  template<class T> operator T*() { g = 1; return 0; }
  template<class T> operator T() { g = 2; return T(); }
};
int main() { B b; int* p = b.operator int*(); return p == 0 && g == 1 ? 0 : 1; }

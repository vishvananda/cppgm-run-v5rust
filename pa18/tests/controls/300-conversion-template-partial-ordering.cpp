// N3485 13.3.3p1 (last bullet), 14.5.6.2p2, 14.8.2.4p3: two equally good
// conversion template specializations are ordered by their templates: operator
// T*() beats operator T(); expect exit 0
int g = 0;
struct A {
  template<class T> operator T() { g = 1; return T(); }
  template<class T> operator T*() { g = 2; return 0; }
};
int main() { A a; int* p = a; return p == 0 && g == 2 ? 0 : 1; }

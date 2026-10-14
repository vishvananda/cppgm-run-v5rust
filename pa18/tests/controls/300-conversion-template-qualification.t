// N3485 14.8.2.3p4-p5 (and the p7 example): operator T***() converts to const
// int* const* const* through a qualification conversion of the deduced A;
// expect exit 0
struct A { template<class T> operator T***() { return 0; } };
int main() { A a; const int* const* const* p = a; return p == 0 ? 0 : 1; }

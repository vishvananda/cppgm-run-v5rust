// N3485 12.3p5, 14.5.2p6, 13.3.3p1: the base's operator T*() is not hidden by
// the derived operator T(); both give int*, and ordering selects T*;
// expect exit 0
struct B { template<class T> operator T*() { return (T*)1; } };
struct D : B { template<class T> operator T() { return T(); } };
int main() { D d; int* p = d; return p == (int*)1 ? 0 : 1; }

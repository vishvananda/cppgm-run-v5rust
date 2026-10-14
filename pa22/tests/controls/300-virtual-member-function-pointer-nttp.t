// N3485 14.3.2p1, 5.5p4: a pointer to a virtual member function as a non-type
// template argument, called through.*, dispatches to the final overrider;
// expect exit 0
struct S { int x; int y; virtual int v() { return 1; } };
struct T : S { int v() { return 2; } };
template<int (S::*F)()> int vcall(S& s) { return (s.*F)(); }
int main() { T t; return vcall<&S::v>(t) == 2 ? 0 : 1; }

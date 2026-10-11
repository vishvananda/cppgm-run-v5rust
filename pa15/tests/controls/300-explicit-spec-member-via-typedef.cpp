// N3485 14.7.3p1, 7.1.3: a member of A<short> named through a typedef
// (template<> int AS::h()) is explicitly specialized; expect exit 0
template<class T1> struct A { int h(); };
template<class T1> int A<T1>::h() { return 1; }
typedef A<short> AS;
template<> int AS::h() { return 5; }
int main() { AS s; A<int> t; return s.h() * 10 + t.h() == 51 ? 0 : 1; }

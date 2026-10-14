// N3485 14.7.3p16 (example): template<> template<class X> struct A<int>::B
// {...} specializes member class template B of A<int> as a template;
// expect exit 0
template <class T1> struct A { template<class T2> struct B { static int f() { return 1; } }; };
template <> template <class X> struct A<int>::B { static int f() { return 2; } };
int main() { return A<int>::B<char>::f() * 10 + A<char>::B<char>::f() == 21 ? 0 : 1; }

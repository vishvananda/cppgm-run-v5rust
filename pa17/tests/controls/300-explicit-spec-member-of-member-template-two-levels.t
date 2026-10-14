// N3485 14.7.3p15-p16: template<> template<> int A<char>::B<char>::mf()
// specializes a member of a member class template specialization;
// expect exit 0
template<class T1> struct A { template<class T2> struct B { int mf(); }; };
template<class T1> template<class T2> int A<T1>::B<T2>::mf() { return 1; }
template<> template<> int A<char>::B<char>::mf() { return 2; }
int main() { A<char>::B<char> b; A<int>::B<char> c; return b.mf() * 10 + c.mf() == 21 ? 0 : 1; }

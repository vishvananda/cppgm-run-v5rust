// N3485 14.5.2p1/14.5.1.2: a member class template of a member template of a
// class template may be defined outside with three template-heads:
// template<class A> template<class B> template<class C> struct X<A>::Y<B>::T
// {...}; expect exit 0
template<class A> struct X { template<class B> struct Y { template<class C> struct T; }; };
template<class A> template<class B> template<class C> struct X<A>::Y<B>::T { int f() { return sizeof(A) * 100 + sizeof(B) * 10 + sizeof(C); } };
int main() { X<int>::Y<char>::T<short> t; return t.f() == 412 ? 0 : 1; }

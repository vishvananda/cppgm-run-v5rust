template<class T> struct A;
template<class T> struct B { B(A<T>&&) { ((T*)0)->bad(); } };
template<class T> struct A { A(A&&); A(B<T>&&); };
void f(A<int>& x) { A<int> a(static_cast<A<int>&&>(x)); }
template<void(*)(A<int>&)> struct C {};
C<&f> c;

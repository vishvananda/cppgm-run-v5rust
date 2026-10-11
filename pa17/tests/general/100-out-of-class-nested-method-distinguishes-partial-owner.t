template<class T> struct A { struct B; };
template<class T> struct A<T&> { struct B; };
template<class T> struct A<T>::B { void f(); };
template<class T> struct A<T&>::B { void f(); };
template<class T> void A<T>::B::f() { typename T::missing value; }
template<class T> void A<T&>::B::f() {}
int main() { A<int&>::B value; value.f(); }

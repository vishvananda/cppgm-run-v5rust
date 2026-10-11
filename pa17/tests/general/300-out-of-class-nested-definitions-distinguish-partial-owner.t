template<class T> struct A { struct B; };
template<class T> struct A<T&> { struct B; };
template<class T> struct A<T>::B { template<class U> B(U); };
template<class T> struct A<T&>::B { template<class U> B(U); };
template<class T> template<class U> A<T>::B::B(U) { typename T::missing value; }
template<class T> template<class U> A<T&>::B::B(U) {}
int main() { A<int&>::B value(0); }

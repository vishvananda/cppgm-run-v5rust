// N3485 14.7.1p1, 11.2: instantiating S<int> instantiates its member
// template's declaration, whose return type names A<int>'s private member: not
// accessible in S<int> (not a deduction failure of the member template).
template<class T> struct A { private: typedef int type; };
template<class T> struct S {
  template<class U> static typename A<T>::type f(U u) { return u; }
};
int main() { return S<int>::f(0); }

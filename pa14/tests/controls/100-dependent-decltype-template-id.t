// N3485 7.1.6.2p4, 14.8.1p3: decltype(f<T>) in a class template member type
// names the function type of the specialization f<T> once T is known;
// expect exit 0
template<class T> int f(T t) { return static_cast<int>(t) + 1; }
template<class T> struct S { decltype(f<T>)* member; };
int main() { S<long> s; s.member = f; return s.member(2) == 3 ? 0 : 1; }

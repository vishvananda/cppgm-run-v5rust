// N3485 14.7.3p16: a member template may not be explicitly specialized while
// its enclosing class template stays unspecialized; expect reject
template<class T> struct S { template<class U> int g(U) { return 1; } };
template<class T> template<> int S<T>::g<char>(char) { return 2; }
int main() { return 0; }

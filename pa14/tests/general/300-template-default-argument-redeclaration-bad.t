// N3485 8.3.6p4/p6, 14.1p9: a later declaration of a function template (or the
// out-of-class definition of a class template member) may not add default
// arguments; must be rejected
template<class T> struct S { int f(int x); };
template<class T> int S<T>::f(int x = 0) { return x; }
int main() { S<int> s; return s.f(0); }

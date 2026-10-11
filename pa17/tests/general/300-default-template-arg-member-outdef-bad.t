// N3485 14.1p9: no default template-arguments in the template-parameter-lists
// of an out-of-class member definition; expect reject
template<class T> struct S { template<class U> int f(); };
template<class T> template<class U = int> int S<T>::f() { return 0; }
int main() { return 0; }
